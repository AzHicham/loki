// Copyright  (C) 2020, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Hove (www.kisio.com).
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
    master_worker::DataAndModels,
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

use std::{ops::Deref, time::SystemTime};

use futures::StreamExt;
use launch::{
    loki::{
        chrono::Utc,
        chrono_tz,
        models::{
            base_model::BaseModel,
            real_time_disruption::{
                chaos_disruption::{cancel_chaos_disruption, store_and_apply_chaos_disruption},
                kirin_disruption::store_and_apply_kirin_disruption,
            },
            RealTimeModel,
        },
        tracing::{debug, error, info, log::trace, warn},
        DataTrait, NaiveDateTime,
    },
    timer::duration_since,
};

use std::{
    sync::{Arc, RwLock},
    thread,
};

use crate::{
    data_downloader::{DataDownloader, DownloadStatus},
    handle_chaos_message::handle_chaos_protobuf,
    server_config::DataSourceParams,
};
use launch::config::launch_params::LocalFileParams;
use tokio::{runtime::Builder, sync::mpsc, time::Duration};

pub struct DataWorker {
    config: ServerConfig,

    data_and_models: Arc<RwLock<DataAndModels>>,

    load_balancer_channels: LoadBalancerChannels,

    host_name: String,
    real_time_queue_name: String,
    reload_queue_name: String,

    realtime_messages: Vec<gtfs_realtime::FeedMessage>,
    kirin_reload_done: bool,

    status_update_sender: mpsc::UnboundedSender<StatusUpdate>,

    shutdown_sender: mpsc::Sender<()>,

    data_source: DataSource,
}

impl DataWorker {
    pub fn new(
        config: ServerConfig,
        data_and_models: Arc<RwLock<DataAndModels>>,
        load_balancer_channels: LoadBalancerChannels,
        status_update_sender: mpsc::UnboundedSender<StatusUpdate>,
        shutdown_sender: mpsc::Sender<()>,
    ) -> Result<Self, Error> {
        let host_name = hostname::get()
            .context("Could not retrieve hostname.")
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
        let uuid = uuid::Uuid::new_v4();

        let instance_name = &config.instance_name;
        let real_time_queue_name =
            format!("loki_{}_{}_real_time_{}", host_name, instance_name, uuid);
        let reload_queue_name = format!("loki_{}_{}_reload_{}", host_name, instance_name, uuid);

        let data_source = match &config.data_source {
            DataSourceParams::Local(local_file_params) => {
                DataSource::Local(local_file_params.clone())
            }
            DataSourceParams::S3(bucket_params) => {
                let data_downloader = DataDownloader::new(bucket_params)?;
                DataSource::S3(data_downloader)
            }
        };

        info!("Data worker created.");
        Ok(Self {
            config,
            data_and_models,
            load_balancer_channels,
            host_name,
            real_time_queue_name,
            reload_queue_name,
            realtime_messages: Vec::new(),
            kirin_reload_done: false,
            status_update_sender,
            shutdown_sender,
            data_source,
        })
    }

    async fn run(mut self) {
        let run_result = self.run_loop().await;
        error!("DataWorker stopped : {:?}", run_result);

        let _ = self.shutdown_sender.send(()).await;
    }

    async fn run_loop(&mut self) -> Result<(), Error> {
        info!("DataWorker starts initial load data.");
        self.load_data()
            .await
            .with_context(|| "Error while loading data".to_string())?;

        // After loading data from disk, load all disruption in chaos database
        // Then apply all extracted disruptions
        if let Err(err) = self.reload_chaos().await {
            error!("Error while reloading chaos. {:?}", err);
        }
        info!("DataWorker completed initial load data.");

        let rabbitmq_connect_retry_interval =
            Duration::from_secs(self.config.rabbitmq.connect_retry_interval.total_seconds());

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
                        "DataWorker was disconnected from rabbitmq. I'll try to reconnect. {:?} ",
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
                .rabbitmq
                .real_time_update_interval
                .total_seconds(),
        ));
        tokio::pin!(interval);

        loop {
            tokio::select! {
                // apply all messages in the buffer every X seconds
                _ = interval.tick() => {
                    if ! self.realtime_messages.is_empty() {
                        info!("It's time to apply {} real time messages.", self.realtime_messages.len());
                        self.apply_realtime_messages().await?;
                        info!("Successfully applied real time messages.");
                    }
                }
                // when a real time message arrives, put it in the buffer
                has_real_time_message = real_time_messages_consumer.next() => {
                    info!("Received a real time message.");
                    self.handle_incoming_realtime_message(has_real_time_message).await?;
                }
                // listen for Reload order
                has_reload_message = reload_consumer.next() => {
                    info!("Received a message on the reload queue.");
                    self.handle_reload_message(has_reload_message, channel).await?;
                }
            }
        }
    }

    async fn load_data(&mut self) -> Result<DataReloadStatus, Error> {
        let config = &self.config;

        let new_base_model = match &mut self.data_source {
            DataSource::S3(data_downloader) => match data_downloader.download_data().await {
                Ok(DownloadStatus::Ok(data_reader)) => launch::read::read_model_from_zip_reader(
                    data_reader,
                    None,
                    "S3",
                    config.input_data_type.clone(),
                    config.default_transfer_duration,
                ),
                Ok(DownloadStatus::AlreadyPresent) => return Ok(DataReloadStatus::Skipped),
                Err(err) => Err(err),
            },
            DataSource::Local(local_files) => launch::read::read_model(
                local_files,
                config.input_data_type.clone(),
                config.default_transfer_duration,
            ),
        };

        let updater = move |data_and_models: &mut DataAndModels| {
            let new_base_model = new_base_model
                .map_err(|err| {
                    error!(
                        "Could not read data. {:?}.I'll keep running with an empty model.",
                        err
                    );
                })
                .unwrap_or_else(|_| BaseModel::empty());

            info!("Model loaded");
            info!("Starting to build data");
            let new_data = launch::read::build_transit_data(&new_base_model);
            info!("Data loaded");
            let new_real_time_model = RealTimeModel::new();
            data_and_models.0 = new_data;
            data_and_models.1 = new_base_model;
            data_and_models.2 = new_real_time_model;

            let calendar = data_and_models.0.calendar();
            let now = Utc::now().naive_utc();
            let base_data_info = BaseDataInfo {
                start_date: calendar.first_date(),
                end_date: calendar.last_date(),
                last_load_at: now,
                dataset_created_at: data_and_models.1.dataset_created_at(),
                timezone: data_and_models.1.timezone_model().unwrap_or(chrono_tz::UTC),
                contributors: data_and_models.1.contributors().map(|c| c.id).collect(),
                publisher_name: data_and_models.1.pubisher_name().map(ToString::to_string),
            };
            Ok(base_data_info)
        };

        let load_result = self.update_data_and_models(updater).await;

        match load_result {
            Ok(base_data_info) => {
                self.send_status_update(StatusUpdate::BaseDataLoad(base_data_info))?;
                Ok(DataReloadStatus::Ok)
            }
            Err(err) => {
                self.send_status_update(StatusUpdate::BaseDataLoadFailed)?;
                Err(err)
            }
        }
    }

    async fn reload_chaos(&mut self) -> Result<(), Error> {
        info!("Start loading chaos disruptions from database");
        let chaos_reload_start_time = SystemTime::now();
        let chaos_params = match &self.config.chaos {
            Some(chaos_params) => chaos_params,
            None => {
                warn!("Chaos is not configured. I skip reload of chaos disruptions.");
                return Ok(());
            }
        };
        let (start_date, end_date) = {
            let rw_lock_read_guard = self.data_and_models.read().map_err(|err| {
                format_err!(
                    "DataWorker failed to acquire read lock on data_and_models : {}",
                    err
                )
            })?;
            let (data, _, _) = rw_lock_read_guard.deref();
            let calendar = data.calendar();
            (calendar.first_date(), calendar.last_date())
        }; // lock is released
        let chaos_disruptions_result = chaos::models::read_chaos_disruption_from_database(
            chaos_params,
            (start_date, end_date),
            &self.config.rabbitmq.real_time_topics,
        );
        info!(
            "Loading chaos disruptions from database completed in {} ms",
            duration_since(chaos_reload_start_time)
        );
        match chaos_disruptions_result {
            Err(err) => error!("Loading chaos database failed : {:?}.", err),
            Ok(disruptions) => {
                info!("Loading chaos disruptions from database succeeded. I'll now apply these disruptions.");
                let updater = |data_and_models: &mut DataAndModels| {
                    let data = &mut data_and_models.0;
                    let base_model = &data_and_models.1;
                    let real_time_model = &mut data_and_models.2;
                    for chaos_disruption in disruptions {
                        match handle_chaos_protobuf(&chaos_disruption) {
                            Ok(disruption) => {
                                store_and_apply_chaos_disruption(
                                    real_time_model,
                                    disruption,
                                    base_model,
                                    data,
                                );
                            }
                            Err(err) => {
                                error!(
                                    "Error while decoding chaos disruption protobuf : {:?}",
                                    err
                                );
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
        info!("Finished loading and applying chaos disruptions from database.");
        Ok(())
    }

    async fn apply_realtime_messages(&mut self) -> Result<(), Error> {
        let messages = std::mem::take(&mut self.realtime_messages);
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
        let timer = SystemTime::now();

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

            updater(&mut lock_guard)
        }; // lock_guard is now released

        trace!("DataWorker ask LoadBalancer to Start.");
        self.send_order_to_load_balancer(LoadBalancerOrder::Start)
            .await?;

        info!("Updated data in {} ms", duration_since(timer));

        update_result
    }

    async fn send_order_to_load_balancer(&mut self, order: LoadBalancerOrder) -> Result<(), Error> {
        self.load_balancer_channels
            .order_sender
            .send(order.clone())
            .await
            .with_context(|| format!("Could not send order {:?} to load balancer.", order))
    }

    async fn handle_incoming_realtime_message(
        &mut self,
        has_real_time_message: Option<Result<lapin::message::Delivery, lapin::Error>>,
    ) -> Result<(), Error> {
        match has_real_time_message {
            Some(Ok(delivery)) => {
                // acknowledge reception of the message
                let _ = delivery
                    .ack(BasicAckOptions::default())
                    .await
                    .map_err(|err| {
                        error!(
                            "Error while acknowledging reception of realtime message : {:?}",
                            err
                        );
                    });

                let proto_message_result =
                    gtfs_realtime::FeedMessage::parse_from_bytes(delivery.data.as_slice());
                match proto_message_result {
                    Ok(proto_message) => {
                        self.realtime_messages.push(proto_message);
                        Ok(())
                    }
                    Err(err) => {
                        error!("Could not decode realtime message into protobuf. {:?}", err);
                        Ok(())
                    }
                }
            }
            Some(Err(err)) => {
                error!("Error while receiving a realtime message. {:?}", err);
                Ok(())
            }
            None => {
                bail!("Consumer for realtime messages has closed.");
            }
        }
    }

    async fn handle_reload_message(
        &mut self,
        has_reload_message: Option<Result<lapin::message::Delivery, lapin::Error>>,
        channel: &lapin::Channel,
    ) -> Result<(), Error> {
        match has_reload_message {
            Some(Ok(delivery)) => {
                // acknowledge reception of the message
                let _ = delivery
                    .ack(BasicAckOptions::default())
                    .await
                    .map_err(|err| {
                        error!(
                            "Error while acknowledging reception of reload message. {:?}",
                            err
                        );
                    });

                let task_message_result = navitia_proto::Task::decode(delivery.data.as_slice());
                match task_message_result {
                    Ok(proto_message) => {
                        let action = proto_message.action();
                        if let navitia_proto::Action::Reload = action {
                            debug!("Received a Reload order.");
                            let reload_result = self.load_data().await?;
                            match reload_result {
                                DataReloadStatus::Ok => {
                                    // if we have unhandled kirin messages, we clear them,
                                    // since we are going to request a full reload from kirin
                                    self.realtime_messages.clear();
                                    // After loading data from disk, load all disruption in chaos database
                                    // Then apply all extracted disruptions
                                    if let Err(err) = self.reload_chaos().await {
                                        error!("Error during reload of Chaos database : {:?}", err);
                                    }
                                    self.reload_kirin(channel).await?;
                                    debug!("Reload completed successfully.");
                                }
                                DataReloadStatus::Skipped => {
                                    info!("Reload skipped");
                                }
                            }
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
        let endpoint = &self.config.rabbitmq.endpoint;
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
        let exchange = &self.config.rabbitmq.exchange;
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
                QueueDeclareOptions {
                    exclusive: true,
                    auto_delete: true,
                    ..QueueDeclareOptions::default()
                },
                FieldTable::default(),
            )
            .await
            .with_context(|| format!("Could not declare queue {}", &self.real_time_queue_name))?;

        info!(
            "Queue declared for kirin real time : {}",
            &self.real_time_queue_name
        );

        let exchange = &self.config.rabbitmq.exchange;
        // bind topics to the real time queue
        for topic in &self.config.rabbitmq.real_time_topics {
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
                QueueDeclareOptions {
                    exclusive: true,
                    auto_delete: true,
                    ..QueueDeclareOptions::default()
                },
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
                &self.config.rabbitmq.exchange,
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
                contributors: self.config.rabbitmq.real_time_topics.clone(),
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
                .rabbitmq
                .reload_request_time_to_live
                .total_seconds()
                * 1000
        );
        let time_to_live_in_milliseconds =
            lapin::types::ShortString::from(time_to_live_in_milliseconds);

        // send the reload task to kirin
        channel
            .basic_publish(
                &self.config.rabbitmq.exchange,
                routing_key,
                BasicPublishOptions::default(),
                &payload,
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
            self.config.rabbitmq.reload_kirin_timeout.total_seconds(),
        );

        let has_message = tokio::time::timeout(timeout, consumer.next()).await;

        match has_message {
            Err(_err) => {
                error!("Realtime reload timed out. I'll keep running without it.");
            }
            Ok(message) => {
                info!("Realtime reload message received. Starting to apply these updates.");
                self.handle_incoming_realtime_message(message).await?;
                self.apply_realtime_messages().await?;
                info!("Realtime reload completed successfully.");

                // Update last kirin reload datetime in case of success only
                let now = Utc::now().naive_utc();
                self.send_status_update(StatusUpdate::KirinReload(now))?;
            }
        }

        delete_queue(channel, &queue_name).await?;

        Ok(())
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
        .context("Received a FeedMessage with a bad header datetime.")?;

    for feed_entity in &message.entity {
        let result = handle_feed_entity(data_and_models, feed_entity, &header_datetime);
        if let Err(err) = result {
            error!(
                "An error occured while handling FeedMessage with timestamp {}. {:?}",
                header_datetime, err
            );
        }
    }
    Ok(())
}

fn handle_feed_entity(
    data_and_models: &mut DataAndModels,
    feed_entity: &chaos_proto::gtfs_realtime::FeedEntity,
    header_datetime: &NaiveDateTime,
) -> Result<(), Error> {
    let id = feed_entity
        .id
        .as_ref()
        .ok_or_else(|| format_err!("FeedEntity has no id"))?;

    let data = &mut data_and_models.0;
    let base_model = &data_and_models.1;
    let real_time_model = &mut data_and_models.2;

    if matches!(feed_entity.is_deleted, Some(true)) {
        cancel_chaos_disruption(real_time_model, id, base_model, data);
    } else if let Some(chaos_disruption) = exts::disruption.get(feed_entity) {
        let chaos_disruption = handle_chaos_protobuf(&chaos_disruption)
            .with_context(|| format!("Could not handle chaos disruption in FeedEntity {}", id))?;
        store_and_apply_chaos_disruption(real_time_model, chaos_disruption, base_model, data);
    } else if feed_entity.trip_update.is_some() {
        let kirin_disruption =
            handle_kirin_protobuf(feed_entity, header_datetime, &data_and_models.1).with_context(
                || format!("Could not handle kirin disruption in FeedEntity {}", id),
            )?;
        store_and_apply_kirin_disruption(real_time_model, kirin_disruption, base_model, data);
    } else {
        bail!(
            "FeedEntity {} is a Kirin message but has no trip_update",
            id
        );
    };

    Ok(())
}

fn parse_header_datetime(
    message: &chaos_proto::gtfs_realtime::FeedMessage,
) -> Result<NaiveDateTime, Error> {
    if let Some(header) = message.header.as_ref() {
        if let Some(timestamp) = header.timestamp {
            make_datetime(timestamp)
        } else {
            bail!("FeedHeader has no timestamp");
        }
    } else {
        bail!("FeedMessage has no header");
    }
}

enum DataReloadStatus {
    Ok,
    Skipped,
}

pub enum DataSource {
    Local(LocalFileParams),
    S3(DataDownloader),
}
