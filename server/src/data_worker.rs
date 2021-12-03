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

use crate::{config::{RabbitMqParams, Config}, master_worker::{DataAndModels, LoadBalancerChannels, Timetable, LoadBalancerOrder}, handle_kirin_message::handle_kirin_protobuf};

use super::chaos_proto::gtfs_realtime;
use super::navitia_proto;
use protobuf::Message;

use failure::{bail, format_err, Error};
use lapin::{
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions,
        QueueBindOptions, QueueDeclareOptions,
    },
    types::FieldTable,
    BasicProperties, ExchangeKind,
};

use std::{
    ops::Deref,
};

use futures::StreamExt;
use launch::loki::{
    tracing::{debug, error, info},
    models::RealTimeModel, DataTrait,
};

use std::{thread, sync::{Arc, RwLock}};



use tokio::{runtime::Builder,  time::Duration};


pub struct DataWorker {
    config: Config,

    data_and_models: Arc<RwLock<DataAndModels>>,

    load_balancer_channels: LoadBalancerChannels,
    
    host_name: String,
    real_time_queue_name: String,
    reload_queue_name: String,


    kirin_messages: Vec<gtfs_realtime::FeedMessage>,
    kirin_reload_done: bool,

    


}

impl DataWorker {
    pub fn new(
        config: Config,
        data_and_models: Arc<RwLock<DataAndModels>>,
        load_balancer_channels: LoadBalancerChannels,
    ) -> Self {
        let host_name = hostname::get()
            .map_err(|err| format_err!("Could not retreive hostname : {}.", err))
            .and_then(|os_string| {
                os_string
                    .into_string()
                    .map_err(|_| format_err!("Could not convert hostname to String."))
            })
            .unwrap_or_else(|err| {
                error!(
                    "Could not retreive hostname : {}. I'll use 'unknown_host' as hostname.",
                    err
                );
                String::from("unknown_host")
            });

        let instance_name = config.instance_name;
        let real_time_queue_name = format!("loki_{}_{}_real_time", host_name, instance_name);
        let reload_queue_name = format!("loki_{}_{}_reload", host_name, instance_name);
        Self {
            config,
            data_and_models,
            load_balancer_channels,
            host_name,
            real_time_queue_name,
            reload_queue_name,
            kirin_messages: Vec::new(),
            kirin_reload_done: false,
        }
    }

    async fn run(mut self) -> Result<(), Error> {


        self.load_data_from_disk().await?;

        let mut rabbitmq_connect_retry_interval = tokio::time::interval(Duration::from_secs(
            self.config.rabbitmq_params.rabbitmq_connect_retry_interval.total_seconds(),
        ));

        loop {
            let has_connection = self.connect().await;

            match has_connection {
                Ok(channel) => {
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
                                error!("Error while reloading real time : {}", err);
                                continue;
                            }
                        }
                    }
                    let result = self.main_loop(&channel).await;
                    match result {
                        Ok(()) => {
                            // this means we should exit the program
                            break;
                        }
                        Err(err) => {
                            error!(
                                "Error occured in DataWorker : {}. I'll relaunch the worker.",
                                err
                            );
                        }
                    }
                }
                Err(err) => {
                    error!(
                        "Connection to rabbitmq failed : {}. I'll try to reconnect later.",
                        err
                    );
                }
            }
            // If connection fails or an error occurs then retry connecting after x seconds
            rabbitmq_connect_retry_interval.tick().await;
        }

        Ok(())
    }


    async fn main_loop(&mut self, channel: &lapin::Channel) -> Result<(), Error> {
        let mut real_time_messages_consumer = create_consumer(channel, &self.real_time_queue_name).await?;

        let mut reload_consumer = create_consumer(channel, &self.reload_queue_name).await?;

        use tokio::time;
        let interval = time::interval(Duration::from_secs(
            self.config.rabbitmq_params.real_time_update_interval.total_seconds(),
        ));
        tokio::pin!(interval);

        loop {
            tokio::select! {
                // sends all messages in the buffer every X seconds
                _ = interval.tick() => {
                    debug!("It's time to apply real time updates.");
                    let messages = std::mem::take(& mut self.kirin_messages);
                    self.apply_realtime_messages(messages).await?;
                    debug!("Successfully applied real time updates.");

                }
                // when a real time message arrives, put it in the buffer
                has_real_time_message = real_time_messages_consumer.next() => {
                    self.handle_incoming_kirin_message(has_real_time_message).await?;
                }
                // listen for Reload order
                has_reload = reload_consumer.next() => {
                    // if we have unhandled kirin messages, we clear them,
                    // since we are going to request a full reload from kirin
                    self.kirin_messages.clear();
                    debug!("Received a Reload order.");
                    self.load_data_from_disk().await?;
                    self.reload_kirin(channel).await?;
                    debug!("Reload completed successfully.");
                }
            }
        }
    }


    async fn load_data_from_disk(&mut self) -> Result<(), Error> {
        let launch_params = self.config.launch_params.clone();
        let updater = move |data_and_models: &mut DataAndModels| {
            let new_base_model = launch::read::read_model(&launch_params).map_err(|err| {
                format_err!(
                    "Could not read data from disk at {:?}, because {}",
                    &launch_params.input_data_path,
                    err
                )
            })?;
            info!("Model loaded");
            info!("Starting to build data");
            let new_data = launch::read::build_transit_data::<Timetable>(
                &new_base_model,
                &launch_params.default_transfer_duration,
            );
            info!("Data loaded");
            let new_real_time_model = RealTimeModel::new();
            data_and_models.0 = new_data;
            data_and_models.1 = new_base_model;
            data_and_models.2 = new_real_time_model;
            Ok(())
        };

        self.update_data_and_models(updater).await
    }

    async fn apply_realtime_messages(
        &mut self,
        messages: Vec<gtfs_realtime::FeedMessage>,
    ) -> Result<(), Error> {
        let updater = |data_and_models: &mut DataAndModels| {
            let data = &mut data_and_models.0;
            let base_model = &data_and_models.1;
            let real_time_model = &mut data_and_models.2;
            for message in messages.into_iter() {
                for feed_entity in message.entity {
                    let disruption_result = handle_kirin_protobuf(&feed_entity);
                    match disruption_result {
                        Err(err) => {
                            error!("Could not handle a kirin message {}", err);
                        }
                        Ok(disruption) => {
                            real_time_model.apply_disruption(&disruption, base_model, data);
                        }
                    }
                }
            }
            Ok(())
        };

        self.update_data_and_models(updater).await
    }

    async fn update_data_and_models<Updater>(&mut self, updater: Updater) -> Result<(), Error>
    where
        Updater: FnOnce(&mut DataAndModels) -> Result<(), Error>,
    {
        self.send_order_to_load_balancer(LoadBalancerOrder::Stop).await?;
        // Wait for the LoadBalancer to Stop
        debug!("MasterWork waiting for LoadBalancer to Stop");
        self.load_balancer_channels
            .stopped_receiver
            .recv()
            .await
            .ok_or_else(|| format_err!("Channel load_balancer_stopped has closed."))?;

        {
            let mut lock_guard = self.data_and_models.write().map_err(|err| {
                format_err!(
                    "Master worker failed to acquire write lock on data_and_models. {}.",
                    err
                )
            })?;

            updater(&mut *lock_guard)?;
        } // lock_guard is now released

        debug!("Master ask LoadBalancer to Start");
        self.send_order_to_load_balancer(LoadBalancerOrder::Start)
            .await
    }

    async fn send_order_to_load_balancer(
        &mut self,
        order: LoadBalancerOrder,
    ) -> Result<(), Error> {
        self
            .load_balancer_channels
            .order_sender
            .send(order.clone())
            .await
            .map_err(|err| format_err!("Could not send order {:?} to load balancer : {}", order, err))
    }

    async fn handle_incoming_kirin_message(& mut self, 
        has_real_time_message : Option<Result<(lapin::Channel, lapin::message::Delivery), lapin::Error>>) 
        -> Result<(), Error>
        {
        match has_real_time_message {
            Some(Ok((channel, delivery))) => {
                // acknowledge reception of the message
                channel
                    .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                    .await
                    .map_err(|err| { error!("Error while acknowleding reception of kirin message : {}", err); });

                let proto_message_result = gtfs_realtime::FeedMessage::parse_from_bytes(&delivery.data.as_slice());
                match proto_message_result {
                    Ok(proto_message) => {
                        self.kirin_messages.push(proto_message);
                        Ok(())
                    },
                    Err(err) => {
                        error!("Could not decode kirin message into protobuf : {}", err);
                        Ok(())
                    },
                }
            },
            Some(Err(err)) => {
                error!("Error while receiving a kirin message : {:?}", err);
                Ok(())
            }
            None => {
                bail!("Consumer for kirin messages has closed.");
            }
        }
    }

    async fn connect(&self) -> Result<lapin::Channel, Error> {
        let endpoint = &self.config.rabbitmq_params.rabbitmq_endpoint;
        let connection = lapin::Connection::connect(
            endpoint,
            lapin::ConnectionProperties::default(),
        )
        .await
        .map_err(|err| {
            format_err!(
                "Could not connect to {}, because : {}",
                endpoint,
                err
            )
        })?;

        info!(
            "Successfully connected to rabbitmq at endpoint {}",
            endpoint
        );
        let channel = connection.create_channel().await?;

        // let's first delete the queues, in case they existed and were not properly deleted
        channel
            .queue_delete(
                &self.real_time_queue_name,
                lapin::options::QueueDeleteOptions::default(),
            )
            .await
            .map_err(|err| {
                format_err!(
                    "Could not delete queue {}, because : {}",
                    &self.real_time_queue_name,
                    err
                )
            })?;
        channel
            .queue_delete(
                &self.real_time_queue_name,
                lapin::options::QueueDeleteOptions::default(),
            )
            .await
            .map_err(|err| {
                format_err!(
                    "Could not delete queue {}, because : {}",
                    &self.real_time_queue_name,
                    err
                )
            })?;

        let exchange = &self.config.rabbitmq_params.rabbitmq_exchange;
        // we declare the exchange
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
            .map_err(|err| {
                format_err!(
                    "Could not delete exchange {}, because : {}",
                    exchange,
                    err
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
            .map_err(|err| {
                format_err!(
                    "Could not declare queue {}, because : {}",
                    &self.real_time_queue_name,
                    err
                )
            })?;
        info!(
            "Queue declared for kirin real time : {}",
            &self.real_time_queue_name
        );

        // declare reload_data queue
        channel
            .queue_declare(
                &self.reload_queue_name,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(|err| {
                format_err!(
                    "Could not declare queue {}, because : {}",
                    &self.reload_queue_name,
                    err
                )
            })?;
        info!(
            "Queue declared for kirin reload : {}",
            &self.reload_queue_name
        );

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
                .map_err(|err| {
                    format_err!(
                        "Could not bind queue {} to topic {}, because : {}",
                        &self.reload_queue_name,
                        topic,
                        err
                    )
                })?;

            info!(
                "Kirin real time queue {} binded successfully to topic {} on exchange {}",
                &self.real_time_queue_name, topic, exchange,
            );
        }

        Ok(channel)
    }

    async fn reload_kirin(&mut self, channel: &lapin::Channel) -> Result<(), Error> {
        use prost::Message;
        // declare a queue to send a message to Kirin to request a full realtime reload
        let queue_name = format!(
            "kirin_reload_request_{}_{}",
            &self.host_name, &self.config.instance_name
        );

        channel.queue_declare(
                &queue_name,
                QueueDeclareOptions {
                    passive: false,
                    durable: false,
                    exclusive: true,
                    auto_delete: true,
                    nowait: false,
                },
                FieldTable::default(),
            ).await
            .map_err(|err| {
                format_err!(
                    "Could not declare queue {} to request realtime reload to Kirin, because : {}",
                    &queue_name,
                    err
                )
            })?;

        // let's create the reload task to be sent into the queue
        let task = {
            let mut task = navitia_proto::Task::default();
            task.set_action(navitia_proto::Action::LoadRealtime);

            let (start_date, end_date) = {
                let lock = self.data_and_models.read().map_err(|err|
                    format_err!("DataWorker failed to acquire read lock on data_and_models : {}", err)
                )?;

                let (data, _, _ ) = lock.deref();
                let start_date = data.calendar().first_date().format("%Y%m%d").to_string();
                let end_date = data.calendar().last_date().format("%Y%m%d").to_string();
                (start_date, end_date)
            }; // lock is dropped here

            let load_realtime = navitia_proto::LoadRealtime {
                queue_name,
                contributors: self.config.rabbitmq_params.rabbitmq_real_time_topics.clone(),
                begin_date: Some(start_date), 
                end_date:  Some(end_date),
            };

            task.load_realtime = Some(load_realtime);
            task
        };
        let payload = task.encode_to_vec();
        let routing_key = "task.load_realtime.INSTANCE";
        let time_to_live_in_milliseconds = format!(
            "{}",
            self.config.rabbitmq_params.reload_request_time_to_live.total_seconds() * 1000
        );
        let time_to_live_in_milliseconds =
            lapin::types::ShortString::from(time_to_live_in_milliseconds);

        // send the reload task to kirin
        channel
            .basic_publish(
                &self.config.rabbitmq_params.rabbitmq_exchange,
                &routing_key,
                BasicPublishOptions::default(),
                payload,
                BasicProperties::default().with_expiration(time_to_live_in_milliseconds),
            )
            .await?
            .await?;

        // wait for the reload messages

        Ok(())
    }


    pub fn run_in_a_thread(self) -> Result<std::thread::JoinHandle<Result<(), Error>>, Error> {
        // copied from https://tokio.rs/tokio/topics/bridging#sending-messages

        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| format_err!("Failed to build tokio runtime. Error : {}", err))?;

        let thread_builder = thread::Builder::new().name("loki_zmq_worker".to_string());
        let handle = thread_builder.spawn(move || runtime.block_on(self.run()))?;
        Ok(handle)
    }
}

async fn create_consumer(channel : & lapin::Channel, queue_name : &str) -> Result<lapin::Consumer, Error>{

    channel.basic_consume(
        queue_name,
        "",
        BasicConsumeOptions {
            no_local: true,
            no_ack: false,
            exclusive: false,
            ..BasicConsumeOptions::default()
        },
        FieldTable::default(),
    ).await
    .map_err(|err| {
        format_err!(
            "Could not create consumer to queue {}, because {}",
            queue_name,
            err
        )
    })
}
