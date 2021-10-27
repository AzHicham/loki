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

use super::{chaos_proto::gtfs_realtime, navitia_proto};

use failure::{format_err, Error};
use lapin::{
    options::{
        BasicAckOptions, BasicConsumeOptions, ExchangeDeclareOptions, QueueBindOptions,
        QueueDeclareOptions,
    },
    types::FieldTable,
    Channel, Connection, ConnectionProperties, Consumer, ExchangeKind, Queue,
};

use futures::StreamExt;
use launch::loki::tracing::{debug, error, info, trace};
use prost::Message as MessageTrait;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, thread};

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
pub fn default_timeout() -> u64 {
    100
}
pub fn default_connection_timeout() -> u64 {
    10000
}

#[derive(Default, Debug, Serialize, Deserialize, StructOpt, Clone)]
#[structopt(rename_all = "snake_case")]
pub struct BrokerConfig {
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

    #[structopt(long)]
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    #[structopt(long)]
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout: u64,
}

pub struct RabbitMqWorker {
    channel: Channel,
    queue: Queue,
    proto_messages: Vec<gtfs_realtime::FeedMessage>,
    amqp_message_sender: mpsc::Sender<Vec<gtfs_realtime::FeedMessage>>,
    timeout: u64,
}

pub fn listen_amqp_in_a_thread(
    config: BrokerConfig,
    amqp_message_sender: mpsc::Sender<Vec<gtfs_realtime::FeedMessage>>,
) -> Result<std::thread::JoinHandle<()>, Error> {
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| format_err!("Failed to build tokio runtime. Error : {}", err))?;

    let thread_builder = thread::Builder::new().name("loki_amqp_worker".to_string());
    let handle = thread_builder
        .spawn(move || runtime.block_on(init_and_listen_amqp(config, amqp_message_sender)))?;
    Ok(handle)
}

async fn init_and_listen_amqp(
    config: BrokerConfig,
    amqp_message_sender: mpsc::Sender<Vec<gtfs_realtime::FeedMessage>>,
) {
    let mut retry_interval =
        tokio::time::interval(Duration::from_millis(config.connection_timeout));
    loop {
        let amqp_worker = RabbitMqWorker::new(config.clone(), amqp_message_sender.clone());
        match amqp_worker {
            Ok(mut worker) => {
                let res = worker.consume().await;
                if let Err(err) = res {
                    error!("RabbitmqWorker: An error occurred: {}", err);
                }
            }
            Err(err) => {
                error!("Connection to rabbitmq failed: {}", err);
            }
        };
        // If connection fails or an error occurs then retry connecting after x seconds
        retry_interval.tick().await;
    }
}

// TODO : listen to both queue_task and queue_rt -> later
// handle init : ask kirin to send
impl RabbitMqWorker {
    pub fn new(
        config: BrokerConfig,
        amqp_message_sender: mpsc::Sender<Vec<gtfs_realtime::FeedMessage>>,
    ) -> Result<Self, Error> {
        let connection = create_connection(&config)?;
        let channel = create_channel(&config, &connection)?;

        // TODO : create a name unique to this instance of loki
        // for example : hostname_instance
        let queue_name = format!("{}_rt", "loki_hostname");
        let queue = declare_queue(&config, &channel, queue_name.as_str())?;
        bind_queue(
            &channel,
            config.rt_topics.as_slice(),
            &config.exchange,
            queue_name.as_str(),
        )?;

        Ok(Self {
            channel,
            queue,
            proto_messages: Vec::new(),
            amqp_message_sender,
            timeout: config.timeout,
        })
    }

    async fn consume(&mut self) -> Result<(), Error> {
        let mut rt_consumer: Consumer = self
            .channel
            .basic_consume(
                self.queue.name().as_str(),
                "rt_consumer",
                BasicConsumeOptions {
                    nowait: false,
                    ..BasicConsumeOptions::default()
                },
                FieldTable::default(),
            )
            .await?;

        loop {
            use tokio::time::timeout;
            let res = timeout(Duration::from_millis(self.timeout), rt_consumer.next()).await;

            if let Ok(opt_message) = res {
                match opt_message {
                    Some(Ok((channel, delivery))) => {
                        channel
                            .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                            .await?;

                        let proto_message = decode_amqp_rt_message(&delivery);
                        match proto_message {
                            Ok(proto_message) => {
                                self.proto_messages.push(proto_message);
                            }
                            Err(err) => {
                                error!("Error decoding proto message : {}", err.to_string())
                            }
                        }
                    }
                    Some(Err(err)) => {
                        return Err(format_err!(
                            "An error occurred when consuming amqp message : {}",
                            err
                        ));
                    }
                    None => {
                        return Err(format_err!(
                            "An unknown error occurred when consuming amqp message",
                        ));
                    }
                }
            } else {
                // timeout : we suppose queue is empty
                // if self.proto_messages contains proto_messages
                // we send them to MasterWorker
                if !self.proto_messages.is_empty() {
                    info!("We got {} proto_messages", self.proto_messages.len());
                    let proto_message = std::mem::take(&mut self.proto_messages);
                    self.amqp_message_sender
                        .send(proto_message)
                        .await
                        .map_err(|err| {
                            format_err!("AMQP Worker could not send proto_message : {}.", err)
                        })?;
                }
            }
        }
    }
}

fn decode_amqp_task_message(
    message: &lapin::message::Delivery,
) -> Result<navitia_proto::Task, Error> {
    let payload = &message.data;
    navitia_proto::Task::decode(&payload[..]).map_err(|err| {
        format_err!(
            "Could not decode rabbitmq task message into protobuf: \n {}",
            err
        )
    })
}

fn handle_task_message(proto: &navitia_proto::Task) {
    let has_action = navitia_proto::Action::from_i32(proto.action);
    match has_action {
        Some(navitia_proto::Action::Reload) => {
            info!("Reload")
            // TODO!
            // load_data
            // as for realtime -> send message to kirin
        }
        _ => trace!("Task ignored"),
    }
}

fn create_connection(config: &BrokerConfig) -> Result<Connection, Error> {
    let address = &config.endpoint;
    info!("Connection to rabbitmq {}", address);
    let connection =
        Connection::connect(address.as_str(), ConnectionProperties::default()).wait()?;
    info!("connected to rabbitmq {} successfully", address);
    Ok(connection)
}

fn create_channel(config: &BrokerConfig, connection: &Connection) -> Result<Channel, Error> {
    let channel = connection.create_channel().wait()?;
    channel
        .exchange_declare(
            &config.exchange,
            ExchangeKind::Topic,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .wait()?;
    debug!("channel created successfully");
    Ok(channel)
}

fn declare_queue(
    config: &BrokerConfig,
    channel: &Channel,
    queue_name: &str,
) -> Result<Queue, Error> {
    let queue = channel
        .queue_declare(
            queue_name,
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .wait()?;
    for topic in &config.rt_topics {
        channel
            .queue_bind(
                queue_name,
                &config.exchange,
                topic,
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .wait()?;
    }

    debug!("queue {} declared successfully", queue_name);
    Ok(queue)
}

fn bind_queue(
    channel: &Channel,
    topics: &[String],
    exchange: &str,
    queue_name: &str,
) -> Result<(), Error> {
    for topic in topics {
        channel
            .queue_bind(
                queue_name,
                exchange,
                topic,
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .wait()?;
    }
    debug!(
        "queue {} binded successfully, exchange : {}, topics : {:?}",
        queue_name, exchange, topics
    );
    Ok(())
}

fn decode_amqp_rt_message(
    message: &lapin::message::Delivery,
) -> Result<gtfs_realtime::FeedMessage, Error> {
    use protobuf::Message;
    let payload = &message.data;
    gtfs_realtime::FeedMessage::parse_from_bytes(&payload[..]).map_err(|err| {
        format_err!(
            "Could not decode rabbitmq realtime message into protobuf: \n {}",
            err
        )
    })
}
