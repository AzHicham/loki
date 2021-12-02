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

use futures::StreamExt;
use launch::loki::{
    tracing::{debug, error, info},
    PositiveDuration,
};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, thread};

use std::str::FromStr;
use structopt::StructOpt;
use tokio::{runtime::Builder, sync::mpsc, time::Duration};

pub fn default_endpoint() -> String {
    "amqp://guest:guest@rabbitmq:5672".to_string()
}
pub fn default_exchange() -> String {
    "navitia".to_string()
}
pub fn default_rt_topics() -> Vec<String> {
    Vec::new()
}
pub fn default_queue_auto_delete() -> bool {
    false
}
pub const DEFAULT_REAL_TIME_UPDATE_INTERVAL: &str = "00:00:30";

pub fn default_real_time_update_interval() -> PositiveDuration {
    PositiveDuration::from_str(DEFAULT_REAL_TIME_UPDATE_INTERVAL).unwrap()
}

pub const DEFAULT_RABBITMQ_CONNECT_RETRY_INTERVAL: &str = "00:00:30";
pub fn default_rabbitmq_connect_retry_interval() -> PositiveDuration {
    PositiveDuration::from_str(DEFAULT_RABBITMQ_CONNECT_RETRY_INTERVAL).unwrap()
}

pub const DEFAULT_RELOAD_REQUEST_TIME_TO_LIVE: &str = "00:00:02";
pub fn default_reload_request_time_to_live() -> PositiveDuration {
    PositiveDuration::from_str(DEFAULT_RELOAD_REQUEST_TIME_TO_LIVE).unwrap()
}

#[derive(Debug, Serialize, Deserialize, StructOpt, Clone)]
#[structopt(rename_all = "snake_case")]
pub struct RabbitMqParams {
    #[structopt(long, default_value = "default_endpoint")]
    #[serde(default = "default_endpoint")]
    pub endpoint: String,

    #[structopt(long, default_value = "default_exchange")]
    #[serde(default = "default_exchange")]
    pub exchange: String,

    #[structopt(long, default_value = "default_rt_topics")]
    #[serde(default = "default_rt_topics")]
    pub rt_topics: Vec<String>,

    #[structopt(long)]
    #[serde(default = "default_queue_auto_delete")]
    pub queue_auto_delete: bool,

    #[structopt(long, default_value = DEFAULT_REAL_TIME_UPDATE_INTERVAL)]
    #[serde(default = "default_real_time_update_interval")]
    pub real_time_update_interval: PositiveDuration,

    #[structopt(long, default_value = DEFAULT_RABBITMQ_CONNECT_RETRY_INTERVAL)]
    #[serde(default = "default_rabbitmq_connect_retry_interval")]
    pub rabbitmq_connect_retry_interval: PositiveDuration,

    #[structopt(long, default_value = DEFAULT_RELOAD_REQUEST_TIME_TO_LIVE)]
    #[serde(default = "default_reload_request_time_to_live")]
    pub reload_request_time_to_live: PositiveDuration,
}

pub struct RabbitMqWorker {
    params: RabbitMqParams,
    instance_name: String,
    host_name: String,
    real_time_queue_name: String,
    reload_queue_name: String,
    kirin_messages_sender: mpsc::Sender<Vec<gtfs_realtime::FeedMessage>>,
    kirin_messages: Vec<gtfs_realtime::FeedMessage>,
    kirin_reload_done: bool,
    reload_data_sender: mpsc::Sender<()>,
}

impl RabbitMqWorker {
    pub fn new(
        params: RabbitMqParams,
        instance_name: String,
        kirin_messages_sender: mpsc::Sender<Vec<gtfs_realtime::FeedMessage>>,
        reload_data_sender: mpsc::Sender<()>,
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

        let real_time_queue_name = format!("loki_{}_{}_real_time", host_name, instance_name);
        let reload_queue_name = format!("loki_{}_{}_reload", host_name, instance_name);
        Self {
            params,
            instance_name,
            host_name,
            real_time_queue_name,
            reload_queue_name,
            kirin_messages_sender,
            kirin_messages: Vec::new(),
            kirin_reload_done: false,
            reload_data_sender,
        }
    }

    async fn run(mut self) {
        let mut retry_interval = tokio::time::interval(Duration::from_secs(
            self.params.rabbitmq_connect_retry_interval.total_seconds(),
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
                                "Error occured in RabbitMqWorker : {}. I'll relaunch the worker.",
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
            retry_interval.tick().await;
        }
    }

    async fn connect(&self) -> Result<lapin::Channel, Error> {
        let connection = lapin::Connection::connect(
            &self.params.endpoint,
            lapin::ConnectionProperties::default(),
        )
        .await
        .map_err(|err| {
            format_err!(
                "Could not connect to {}, because : {}",
                &self.params.endpoint,
                err
            )
        })?;

        info!(
            "Successfully connected to rabbitmq at endpoint {}",
            &self.params.endpoint
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

        // we declare the exchange
        channel
            .exchange_declare(
                &self.params.exchange,
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
                    &self.params.exchange,
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
        for topic in &self.params.rt_topics {
            channel
                .queue_bind(
                    &self.real_time_queue_name,
                    &self.params.exchange,
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
                &self.real_time_queue_name, topic, &self.params.exchange,
            );
        }

        Ok(channel)
    }

    async fn reload_kirin(&mut self, channel: &lapin::Channel) -> Result<(), Error> {
        use prost::Message;
        // declare a queue to send a message to Kirin to request a full realtime reload
        let queue_name = format!(
            "kirin_reload_request_{}_{}",
            &self.host_name, &self.instance_name
        );

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
            let load_realtime = navitia_proto::LoadRealtime {
                queue_name,
                contributors: self.params.rt_topics.clone(),
                begin_date: None, // TODO : recover the dates somehow...
                end_date: None,
            };

            task.load_realtime = Some(load_realtime);
            task
        };
        let payload = task.encode_to_vec();
        let routing_key = "task.load_realtime.INSTANCE";
        let time_to_live_in_milliseconds = format!(
            "{}",
            self.params.reload_request_time_to_live.total_seconds() * 1000
        );
        let time_to_live_in_milliseconds =
            lapin::types::ShortString::from(time_to_live_in_milliseconds);
        channel
            .basic_publish(
                &self.params.exchange,
                &routing_key,
                BasicPublishOptions::default(),
                payload,
                BasicProperties::default().with_expiration(time_to_live_in_milliseconds),
            )
            .await?
            .await?;

        Ok(())
    }

    async fn main_loop(&mut self, channel: &lapin::Channel) -> Result<(), Error> {
        let mut real_time_messages_consumer = channel
            .basic_consume(
                &self.real_time_queue_name,
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
            .map_err(|err| {
                format_err!(
                    "Could not create consumer to queue {}, because {}",
                    &self.reload_queue_name,
                    err
                )
            })?;

        let mut reload_consumer = channel
            .basic_consume(
                &self.reload_queue_name,
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
            .map_err(|err| {
                format_err!(
                    "Could not create consumer to queue {}, because {}",
                    &self.reload_queue_name,
                    err
                )
            })?;

        use tokio::time;
        let interval = time::interval(Duration::from_secs(
            self.params.real_time_update_interval.total_seconds(),
        ));
        tokio::pin!(interval);

        loop {
            tokio::select! {
                // sends all messages in the buffer every X seconds
                _ = interval.tick() => {
                    debug!("It's time to send kirin messages to Master.");
                    // Timeout, no matter whether there're messages or not in Rabbitmq, we send the messages in buffer
                    self.send_proto_messages().await?;

                }
                // when a real time message arrives, put it in the buffer
                has_real_time_message = real_time_messages_consumer.next() => {
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
                                },
                                Err(err) => {
                                    error!("Could not decode kirin message into protobuf : {}", err);
                                },
                            }
                        },
                        Some(Err(err)) => {
                            error!("Error while receiving a kirin message : {:?}", err);
                        }
                        None => {
                            bail!("Consumer for kirin messages has closed.");
                        }
                    }
                }
                // listen for Reload order
                has_reload = reload_consumer.next() => {
                    // if we have unhandled kirin messages, we clear them,
                    // since we are going to request a full reload from kirin
                    self.kirin_messages.clear();
                    // send message to master to reload data from disk
                    self.reload_data_sender.send(()).await
                        .map_err(|err| format_err!("Channel reload_data has closed : {}", err))?;
                    self.reload_kirin(channel).await;
                }
            }
        }
    }

    async fn send_proto_messages(&mut self) -> Result<(), Error> {
        if self.kirin_messages.is_empty() {
            return Ok(());
        }
        let messages = std::mem::take(&mut self.kirin_messages);
        self.kirin_messages_sender
            .send(messages)
            .await
            .map_err(|err| {
                format_err!(
                    "Channel to send kirin messages to Master has closed : {}.",
                    err
                )
            })
    }

    pub fn run_in_a_thread(self) -> Result<std::thread::JoinHandle<()>, Error> {
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
