// Copyright  (C) 2020, Kisio Digital and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Kisio Digital (www.kisio.com).
// Help us simplify mobility and open public transport:
// a non ending quest to the responsive locomotion way of traveling!
//
// This contribution is a part of the research and development work of the
// IVA Project which aims to enhance traveler information and is carried out
// under the leadership of the Technological Research Institute SystemX,
// with the partnership and support of the transport organization authority
// Ile-De-France Mobilités (IDFM), SNCF, and public funds
// under the scope of the French Program "Investissements d’Avenir".
//
// LICENCE: This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// Stay tuned using
// twitter @navitia
// channel `#navitia` on riot https://riot.im/app/#/room/#navitia:matrix.org
// https://groups.google.com/d/forum/navitia
// www.navitia.io

use crate::{
    chaos, chaos_proto,
    handle_chaos_message::make_datetime,
    handle_kirin_message::handle_kirin_protobuf,
    load_balancer::{LoadBalancerChannels, LoadBalancerOrder},
    master_worker::{DataAndModels, Timetable},
    server_config::ServerConfig,
    status_worker::{BaseDataInfo, StatusUpdate},
};

use super::{
    chaos_proto::{chaos::exts, gtfs_realtime},
    navitia_proto,
};
use prost::Message as ProstMessage;
use protobuf::Message as ProtobufMessage;

use anyhow::{bail, format_err, Context, Error};
use lapin::{
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions,
        QueueBindOptions, QueueDeclareOptions,
    },
    types::FieldTable,
    BasicProperties, ExchangeKind,
};

use std::ops::Deref;

use futures::StreamExt;
use launch::loki::{
    chrono::Utc,
    models::{base_model::BaseModel, RealTimeModel},
    tracing::{debug, error, info, log::trace, warn},
    DataTrait, NaiveDateTime,
};

use std::{
    sync::{Arc, RwLock},
    thread,
};

use crate::handle_chaos_message::handle_chaos_protobuf;
use tokio::{runtime::Builder, sync::mpsc, time::Duration};

pub struct DataWorker {
    config: ServerConfig,

    data_and_models: Arc<RwLock<DataAndModels>>,

    load_balancer_channels: LoadBalancerChannels,

    host_name: String,
    real_time_queue_name: String,
    reload_queue_name: String,

    kirin_messages: Vec<gtfs_realtime::FeedMessage>,
    kirin_reload_done: bool,

    status_update_sender: mpsc::UnboundedSender<StatusUpdate>,

    shutdown_sender: mpsc::Sender<()>,
}

impl DataWorker {
    pub fn new(
        config: ServerConfig,
        data_and_models: Arc<RwLock<DataAndModels>>,
        load_balancer_channels: LoadBalancerChannels,
        status_update_sender: mpsc::UnboundedSender<StatusUpdate>,
        shutdown_sender: mpsc::Sender<()>,
    ) -> Self {
        let host_name = hostname::get()
            .context("Could not retreive hostname.")
            .and_then(|os_string| {
                os_string
                    .into_string()
                    .map_err(|_| format_err!("Could not convert hostname to String."))
            })
            .unwrap_or_else(|err| {
                error!(
                    "Could not retreive hostname. I'll use 'unknown_host' as hostname. {:?}",
                    err
                );
                String::from("unknown_host")
            });

        let instance_name = &config.instance_name;
        let real_time_queue_name = format!("loki_{}_{}_real_time", host_name, instance_name);
        let reload_queue_name = format!("loki_{}_{}_reload", host_name, instance_name);
        info!("Data worker created.");
        Self {
            config,
            data_and_models,
            load_balancer_channels,
            host_name,
            real_time_queue_name,
            reload_queue_name,
            kirin_messages: Vec::new(),
            kirin_reload_done: false,
            status_update_sender,
            shutdown_sender,
        }
    }

    async fn run(mut self) {
        let run_result = self.run_loop().await;
        error!("DataWorker stopped : {:?}", run_result);

        let _ = self.shutdown_sender.send(()).await;
    }

    async fn run_loop(&mut self) -> Result<(), Error> {
        debug!("DataWorker starts initial load data from disk.");
        self.load_data_from_disk()
            .await
            .with_context(|| "Error while loading data from disk.".to_string())?;

        // After loading data from disk, load all disruption in chaos database
        // Then apply all extracted disruptions
        if let Err(err) = self.reload_chaos().await {
            error!("Error while reloading kirin. {:?}", err);
        }

        let rabbitmq_connect_retry_interval = Duration::from_secs(
            self.config
                .rabbitmq_params
                .rabbitmq_connect_retry_interval
                .total_seconds(),
        );

        // A future that will tick() at regular interval
        // cf https://docs.rs/tokio/1.14.0/tokio/time/fn.interval.html
        let mut retry_interval = tokio::time::interval(rabbitmq_connect_retry_interval);
        // we want to skip missed tick()s
        // https://docs.rs/tokio/1.14.0/tokio/time/enum.MissedTickBehavior.html#variant.Skip
        retry_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            // the first tick() completes immediately
            // cf https://docs.rs/tokio/1.14.0/tokio/time/fn.interval.html
            retry_interval.tick().await;

            let has_connection = self.connect().await;

            match has_connection {
                Ok(channel) => {
                    info!("Connected to RabbitMq.");
                    self.send_status_update(StatusUpdate::RabbitMqConnected)?;
                    if !self.kirin_reload_done {
                        let reload_result = self.reload_kirin(&channel).await;
                        match reload_result {
                            Ok(()) => {
                                // the connection to rabbitmq may close
                                // when this happens, we will exit main_loop(), and connect() again
                                // but we don't want to ask Kirin for a full reload
                                // we just want to restart the main_loop()
                                self.kirin_reload_done = true;
                            }
                            Err(err) => {
                                error!("Error while reloading kirin : {:?}", err);
                                continue;
                            }
                        }
                    }
                    let result = self.main_loop(&channel).await;
                    error!(
                        "DataWorker main loop exited. I'll relaunch the worker. {:?} ",
                        result
                    );
                    self.send_status_update(StatusUpdate::RabbitMqDisconnected)?;
                }
                Err(err) => {
                    error!(
                        "Error while connecting to rabbitmq. I'll try to reconnect later. {:?}",
                        err
                    );
                }
            }
        }
    }

    async fn main_loop(&mut self, channel: &lapin::Channel) -> Result<(), Error> {
        let mut real_time_messages_consumer =
            create_consumer(channel, &self.real_time_queue_name).await?;

        let mut reload_consumer = create_consumer(channel, &self.reload_queue_name).await?;

        let interval = tokio::time::interval(Duration::from_secs(
            self.config
                .rabbitmq_params
                .real_time_update_interval
                .total_seconds(),
        ));
        tokio::pin!(interval);

        loop {
            tokio::select! {
                // sends all messages in the buffer every X seconds
                _ = interval.tick() => {
                    if ! self.kirin_messages.is_empty() {
                        trace!("It's time to apply {} real time updates.", self.kirin_messages.len());
                        self.apply_realtime_messages().await?;
                        trace!("Successfully applied real time updates.");
                    }
                }
                // when a real time message arrives, put it in the buffer
                has_real_time_message = real_time_messages_consumer.next() => {
                    info!("Received a real time message.");
                    self.handle_incoming_kirin_message(has_real_time_message).await?;
                }
                // listen for Reload order
                has_reload_message = reload_consumer.next() => {
                    info!("Received a message on the reload queue.");
                    self.handle_reload_message(has_reload_message, channel).await?;
                }
            }
        }
    }

    async fn load_data_from_disk(&mut self) -> Result<(), Error> {
        let launch_params = self.config.launch_params.clone();
        let updater = move |data_and_models: &mut DataAndModels| {
            let new_base_model = launch::read::read_model(&launch_params)
                .map_err(|err| {
                    error!(
                        "Could not read data from disk at {:?}. {:?}. \
                            I'll keep running with an empty model.",
                        launch_params.input_data_path, err
                    )
                })
                .unwrap_or_else(|()| BaseModel::empty());

            info!("Model loaded");
            info!("Starting to build data");
            let new_data = launch::read::build_transit_data::<Timetable>(&new_base_model);
            info!("Data loaded");
            let new_real_time_model = RealTimeModel::new();
            data_and_models.0 = new_data;
            data_and_models.1 = new_base_model;
            data_and_models.2 = new_real_time_model;
            let calendar = data_and_models.0.calendar();
            Ok((*calendar.first_date(), *calendar.last_date()))
        };

        let (start_date, end_date) = self.update_data_and_models(updater).await?;

        let now = Utc::now().naive_utc();
        let base_data_info = BaseDataInfo {
            start_date,
            end_date,
            last_load_at: now,
        };
        self.send_status_update(StatusUpdate::BaseDataLoad(base_data_info))
    }

    async fn reload_chaos(&mut self) -> Result<(), Error> {
        let (start_date, end_date) = {
            let rw_lock_read_guard = self.data_and_models.read().map_err(|err| {
                format_err!(
                    "DataWorker failed to acquire read lock on data_and_models : {}",
                    err
                )
            })?;
            let (data, _, _) = rw_lock_read_guard.deref();
            let calendar = data.calendar();
            (*calendar.first_date(), *calendar.last_date())
        }; // lock is released
        match chaos::models::read_chaos_disruption_from_database(
            &self.config.chaos_params,
            (start_date, end_date),
            &self.config.rabbitmq_params.rabbitmq_real_time_topics,
        ) {
            Err(err) => error!("Loading chaos database failed : {:?}.", err),
            Ok(disruptions) => {
                let updater = |data_and_models: &mut DataAndModels| {
                    let data = &mut data_and_models.0;
                    let base_model = &data_and_models.1;
                    let real_time_model = &mut data_and_models.2;
                    for chaos_disruption in disruptions {
                        match handle_chaos_protobuf(&chaos_disruption) {
                            Ok(disruption) => real_time_model
                                .store_and_apply_disruption(disruption, base_model, data),
                            Err(err) => {
                                error!("Error while applying chaos disruption : {:?}", err)
                            }
                        }
                    }
                    Ok(())
                };
                self.update_data_and_models(updater).await?;

                let now = Utc::now().naive_utc();
                self.send_status_update(StatusUpdate::ChaosReload(now))?;
            }
        }
        Ok(())
    }

    async fn apply_realtime_messages(&mut self) -> Result<(), Error> {
        let messages = std::mem::take(&mut self.kirin_messages);
        let updater = |data_and_models: &mut DataAndModels| {
            for message in messages {
                let result = handle_realtime_message(data_and_models, &message);
                if let Err(err) = result {
                    error!("Could not handle real time message. {:?}", err);
                }
            }
            Ok(())
        };

        self.update_data_and_models(updater).await?;

        let now = Utc::now().naive_utc();
        self.send_status_update(StatusUpdate::RealTimeUpdate(now))
    }

    async fn update_data_and_models<Updater, T>(&mut self, updater: Updater) -> Result<T, Error>
    where
        Updater: FnOnce(&mut DataAndModels) -> Result<T, Error>,
    {
        trace!("DataWorker ask LoadBalancer to Stop.");
        self.send_order_to_load_balancer(LoadBalancerOrder::Stop)
            .await?;

        self.load_balancer_channels
            .stopped_receiver
            .recv()
            .await
            .ok_or_else(|| format_err!("Channel load_balancer_stopped has closed."))?;
        trace!("DataWorker received Stopped signal from LoadBalancer.");

        let update_result = {
            let mut lock_guard = self.data_and_models.write().map_err(|err| {
                format_err!(
                    "DataWorker worker failed to acquire write lock on data_and_models. {}.",
                    err
                )
            })?;

            updater(&mut *lock_guard)
        }; // lock_guard is now released

        trace!("DataWorker ask LoadBalancer to Start.");
        self.send_order_to_load_balancer(LoadBalancerOrder::Start)
            .await?;

        update_result
    }

    async fn send_order_to_load_balancer(&mut self, order: LoadBalancerOrder) -> Result<(), Error> {
        self.load_balancer_channels
            .order_sender
            .send(order.clone())
            .await
            .with_context(|| format!("Could not send order {:?} to load balancer.", order))
    }

    async fn handle_incoming_kirin_message(
        &mut self,
        has_real_time_message: Option<
            Result<(lapin::Channel, lapin::message::Delivery), lapin::Error>,
        >,
    ) -> Result<(), Error> {
        match has_real_time_message {
            Some(Ok((channel, delivery))) => {
                // acknowledge reception of the message
                let _ = channel
                    .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                    .await
                    .map_err(|err| {
                        error!(
                            "Error while acknowleding reception of kirin message : {:?}",
                            err
                        );
                    });

                let proto_message_result =
                    gtfs_realtime::FeedMessage::parse_from_bytes(delivery.data.as_slice());
                match proto_message_result {
                    Ok(proto_message) => {
                        self.kirin_messages.push(proto_message);
                        Ok(())
                    }
                    Err(err) => {
                        error!("Could not decode kirin message into protobuf. {:?}", err);
                        Ok(())
                    }
                }
            }
            Some(Err(err)) => {
                error!("Error while receiving a kirin message. {:?}", err);
                Ok(())
            }
            None => {
                bail!("Consumer for kirin messages has closed.");
            }
        }
    }

    async fn handle_reload_message(
        &mut self,
        has_reload_message: Option<
            Result<(lapin::Channel, lapin::message::Delivery), lapin::Error>,
        >,
        channel: &lapin::Channel,
    ) -> Result<(), Error> {
        match has_reload_message {
            Some(Ok((reload_channel, delivery))) => {
                // acknowledge reception of the message
                let _ = reload_channel
                    .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                    .await
                    .map_err(|err| {
                        error!(
                            "Error while acknowleding reception of reload message. {:?}",
                            err
                        );
                    });

                let task_message_result = navitia_proto::Task::decode(delivery.data.as_slice());
                match task_message_result {
                    Ok(proto_message) => {
                        let action = proto_message.action();
                        if let navitia_proto::Action::Reload = action {
                            debug!("Received a Reload order.");
                            // if we have unhandled kirin messages, we clear them,
                            // since we are going to request a full reload from kirin
                            self.kirin_messages.clear();
                            self.load_data_from_disk().await?;
                            // After loading data from disk, load all disruption in chaos database
                            // Then apply all extracted disruptions
                            if let Err(err) = self.reload_chaos().await {
                                error!("Error during reload of Chaos database : {:?}", err);
                            }
                            self.reload_kirin(channel).await?;
                            debug!("Reload completed successfully.");
                        } else {
                            error!(
                                "Receive a reload message with unhandled action value : {:?}",
                                action
                            );
                        }

                        Ok(())
                    }
                    Err(err) => {
                        error!("Could not decode reload message into protobuf. {:?}", err);
                        Ok(())
                    }
                }
            }
            Some(Err(err)) => {
                error!("Error while receiving a reload message. {:?}", err);
                Ok(())
            }
            None => {
                bail!("Consumer for reload messages has closed.");
            }
        }
    }

    async fn connect(&self) -> Result<lapin::Channel, Error> {
        let endpoint = &self.config.rabbitmq_params.rabbitmq_endpoint;
        let connection =
            lapin::Connection::connect(endpoint, lapin::ConnectionProperties::default())
                .await
                .with_context(|| format!("Could not connect to rabbitmq endpoint {}", endpoint))?;

        info!(
            "Successfully connected to rabbitmq at endpoint {}",
            endpoint
        );
        let channel = connection.create_channel().await?;

        // we declare the exchange
        let exchange = &self.config.rabbitmq_params.rabbitmq_exchange;
        channel
            .exchange_declare(
                exchange,
                ExchangeKind::Topic,
                ExchangeDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .with_context(|| format!("Could not delete rabbit mq exchange {}", exchange))?;

        self.connect_real_time_queue(&channel).await?;
        self.connect_reload_queue(&channel).await?;

        Ok(channel)
    }

    async fn connect_real_time_queue(&self, channel: &lapin::Channel) -> Result<(), Error> {
        // let's first delete the queue, in case it existed and was not properly deleted
        channel
            .queue_delete(
                &self.real_time_queue_name,
                lapin::options::QueueDeleteOptions::default(),
            )
            .await
            .with_context(|| {
                format!(
                    "Could not delete queue named {}",
                    &self.real_time_queue_name
                )
            })?;

        // declare real time queue
        channel
            .queue_declare(
                &self.real_time_queue_name,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .with_context(|| format!("Could not declare queue {}", &self.real_time_queue_name))?;

        info!(
            "Queue declared for kirin real time : {}",
            &self.real_time_queue_name
        );

        let exchange = &self.config.rabbitmq_params.rabbitmq_exchange;
        // bind topics to the real time queue
        for topic in &self.config.rabbitmq_params.rabbitmq_real_time_topics {
            channel
                .queue_bind(
                    &self.real_time_queue_name,
                    exchange,
                    topic,
                    QueueBindOptions::default(),
                    FieldTable::default(),
                )
                .await
                .with_context(|| {
                    format!(
                        "Could not bind queue {} to topic {}",
                        &self.real_time_queue_name, topic
                    )
                })?;

            info!(
                "Kirin real time queue {} binded successfully to topic {} on exchange {}",
                &self.real_time_queue_name, topic, exchange,
            );
        }

        Ok(())
    }

    async fn connect_reload_queue(&self, channel: &lapin::Channel) -> Result<(), Error> {
        // let's first delete the queues, in case they existed and were not properly deleted
        delete_queue(channel, &self.reload_queue_name).await?;

        // declare reload_data queue
        channel
            .queue_declare(
                &self.reload_queue_name,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .with_context(|| format!("Could not declare queue {}", &self.reload_queue_name))?;

        info!("Queue declared for reload : {}", &self.reload_queue_name);

        // bind the reload queue to the topic instance_name.task.*
        let topic = format!("{}.task.*", self.config.instance_name);
        channel
            .queue_bind(
                &self.reload_queue_name,
                &self.config.rabbitmq_params.rabbitmq_exchange,
                &topic,
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await
            .with_context(|| format!("Could not bind queue named {}", &self.reload_queue_name))?;

        info!(
            "Reload queue {}  binded to topic: {}",
            &self.reload_queue_name, topic
        );

        Ok(())
    }

    async fn reload_kirin(&mut self, channel: &lapin::Channel) -> Result<(), Error> {
        info!("Asking Kirin for a full realtime reload.");

        // declare a queue to send a message to Kirin to request a full realtime reload
        let queue_name = format!(
            "kirin_reload_request_{}_{}",
            &self.host_name, &self.config.instance_name
        );

        // let's first delete the queue, just in case
        delete_queue(channel, &queue_name).await?;

        channel
            .queue_declare(
                &queue_name,
                QueueDeclareOptions {
                    passive: false,
                    durable: false,
                    exclusive: true,
                    auto_delete: true,
                    nowait: false,
                },
                FieldTable::default(),
            )
            .await
            .with_context(|| format!("Could not declare queue {}", &self.reload_queue_name))?;

        // let's create the reload task to be sent into the queue
        let task = {
            let mut task = navitia_proto::Task::default();
            task.set_action(navitia_proto::Action::LoadRealtime);

            let (start_date, end_date) = {
                let lock = self.data_and_models.read().map_err(|err| {
                    format_err!(
                        "DataWorker failed to acquire read lock on data_and_models : {}",
                        err
                    )
                })?;

                let (data, _, _) = lock.deref();
                let start_date = data.calendar().first_date().format("%Y%m%d").to_string();
                let end_date = data.calendar().last_date().format("%Y%m%d").to_string();
                (start_date, end_date)
            }; // lock is dropped here

            let load_realtime = navitia_proto::LoadRealtime {
                queue_name: queue_name.clone(),
                contributors: self
                    .config
                    .rabbitmq_params
                    .rabbitmq_real_time_topics
                    .clone(),
                begin_date: Some(start_date),
                end_date: Some(end_date),
            };

            task.load_realtime = Some(load_realtime);
            task
        };

        let payload = task.encode_to_vec();
        let routing_key = "task.load_realtime.INSTANCE";
        let time_to_live_in_milliseconds = format!(
            "{}",
            self.config
                .rabbitmq_params
                .reload_request_time_to_live
                .total_seconds()
                * 1000
        );
        let time_to_live_in_milliseconds =
            lapin::types::ShortString::from(time_to_live_in_milliseconds);

        // send the reload task to kirin
        channel
            .basic_publish(
                &self.config.rabbitmq_params.rabbitmq_exchange,
                routing_key,
                BasicPublishOptions::default(),
                payload,
                BasicProperties::default().with_expiration(time_to_live_in_milliseconds),
            )
            .await?
            .await?;

        info!(
            "Realtime reload task sent in queue {} with routing_key {}",
            queue_name, routing_key
        );

        // wait for the reload messages from kirin
        let mut consumer = create_consumer(channel, &queue_name).await?;
        let timeout = std::time::Duration::from_secs(
            self.config
                .rabbitmq_params
                .reload_kirin_timeout
                .total_seconds(),
        );

        let has_message = tokio::time::timeout(timeout, consumer.next()).await;

        match has_message {
            Err(_err) => {
                error!("Realtime reload timed out. I'll keep running without it.");
            }
            Ok(message) => {
                info!("Realtime reload message received. Starting to apply these updates.");
                self.handle_incoming_kirin_message(message).await?;
                self.apply_realtime_messages().await?;
                info!("Realtime reload completed successfully.");
            }
        }

        delete_queue(channel, &queue_name).await?;

        let now = Utc::now().naive_utc();
        self.send_status_update(StatusUpdate::KirinReload(now))
    }

    fn send_status_update(&self, status_update: StatusUpdate) -> Result<(), Error> {
        self.status_update_sender
            .send(status_update)
            .map_err(|err| {
                format_err!(
                    "StatusWorker channel to send status updates has closed. {}",
                    err
                )
            })
    }

    pub fn run_in_a_thread(self) -> Result<std::thread::JoinHandle<()>, Error> {
        // copied from https://tokio.rs/tokio/topics/bridging#sending-messages

        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Failed to build tokio runtime.")?;

        let thread_builder = thread::Builder::new().name("loki_data_worker".to_string());
        let handle = thread_builder.spawn(move || runtime.block_on(self.run()))?;
        Ok(handle)
    }
}

async fn create_consumer(
    channel: &lapin::Channel,
    queue_name: &str,
) -> Result<lapin::Consumer, Error> {
    channel
        .basic_consume(
            queue_name,
            "",
            BasicConsumeOptions {
                no_local: true,
                no_ack: false,
                exclusive: false,
                ..BasicConsumeOptions::default()
            },
            FieldTable::default(),
        )
        .await
        .with_context(|| format!("Could not create consumer to queue {}.", queue_name))
}

async fn delete_queue(channel: &lapin::Channel, queue_name: &str) -> Result<u32, Error> {
    channel
        .queue_delete(queue_name, lapin::options::QueueDeleteOptions::default())
        .await
        .with_context(|| format!("Could not create delete to queue {}.", queue_name))
}

fn handle_realtime_message(
    data_and_models: &mut DataAndModels,
    message: &chaos_proto::gtfs_realtime::FeedMessage,
) -> Result<(), Error> {
    let header_datetime = parse_header_datetime(message)
        .map_err(|err| {
            warn!(
                "Received a FeedMessage with a bad header datetime. {:?}",
                err
            );
        })
        .ok();

    for feed_entity in &message.entity {
        let result = handle_feed_entity(data_and_models, feed_entity, &header_datetime);
        if let Err(err) = result {
            let datetime_str = header_datetime
                .map(|datetime| format!("{}", datetime))
                .unwrap_or("BadHeaderDatetime".to_string());
            error!(
                "An error occured while handling FeedMessage with timestamp {}. {:?}",
                datetime_str, err
            )
        }
    }
    Ok(())
}

fn handle_feed_entity(
    data_and_models: &mut DataAndModels,
    feed_entity: &chaos_proto::gtfs_realtime::FeedEntity,
    header_datetime: &Option<NaiveDateTime>,
) -> Result<(), Error> {
    if !feed_entity.has_id() {
        bail!("FeedEntity has no id");
    }
    let id = feed_entity.get_id();
    if feed_entity.get_is_deleted() {
        bail!(
            "FeedEntity {} has is_deleted == true. This is not supported",
            id
        );
    }

    let disruption = if let Some(chaos_disruption) = exts::disruption.get(feed_entity) {
        handle_chaos_protobuf(&chaos_disruption)
            .with_context(|| format!("Could not handle chaos disruption in FeedEntity {}", id))?
    } else if feed_entity.has_trip_update() {
        let calendar = data_and_models.0.calendar();
        let calendar_period = (*calendar.first_date(), *calendar.last_date());
        handle_kirin_protobuf(feed_entity, header_datetime, &calendar_period)
            .with_context(|| format!("Could not handle kirin disruption in FeedEntity {}", id))?
    } else {
        bail!(
            "FeedEntity {} is a Kirin message but has no trip_update",
            id
        );
    };

    let data = &mut data_and_models.0;
    let base_model = &data_and_models.1;
    let real_time_model = &mut data_and_models.2;

    real_time_model.store_and_apply_disruption(disruption, base_model, data);
    Ok(())
}

fn parse_header_datetime(
    message: &chaos_proto::gtfs_realtime::FeedMessage,
) -> Result<NaiveDateTime, Error> {
    if message.has_header() {
        let header = message.get_header();
        if header.has_timestamp() {
            let timestamp = header.get_timestamp();
            make_datetime(timestamp)
        } else {
            bail!("FeedHeader has no timestamp");
        }
    } else {
        bail!("FeedMessage has no header");
    }
}
