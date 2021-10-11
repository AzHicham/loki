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

use super::chaos_proto::{chaos, gtfs_realtime, kirin};
use super::navitia_proto;

use failure::{format_err, Error};
use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, ExchangeDeclareOptions, QueueBindOptions,
    QueueDeclareOptions,
};
use lapin::{types::FieldTable, Channel, Connection, ConnectionProperties, ExchangeKind, Queue};

use launch::loki::tracing::{error, info, trace, warn};
use prost::Message as MessageTrait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use structopt::StructOpt;

pub fn default_exchange() -> String {
    "navitia".to_string()
}
pub fn default_rt_topics() -> Vec<String> {
    Vec::new()
}
pub fn default_queue_auto_delete() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, StructOpt, Clone)]
#[structopt(rename_all = "snake_case")]
pub struct BrokerConfig {
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
}

pub struct RabbitMqWorker {
    connection: Connection,
    channel: Channel,
    queue: Queue,
    proto_messages: Vec<gtfs_realtime::FeedMessage>,
}

// TODO : listen to both queue_task and queue_rt -> later
// handle init : ask kirin to send
impl RabbitMqWorker {
    pub fn new(config: &BrokerConfig) -> Result<Self, Error> {
        let connection = create_connection(config)?;
        let channel = create_channel(config, &connection)?;

        // TODO : create a name unique to this instance of loki
        // for example : hostname_instance
        let queue_name = format!("{}_rt", "loki_hostname");
        let queue = declare_queue(config, &channel, queue_name.as_str())?;
        bind_queue(
            &channel,
            &config.rt_topics.as_slice(),
            &config.exchange,
            queue_name.as_str(),
        )?;

        Ok(Self {
            connection,
            channel,
            queue,
        })
    }

    async fn consume(&self) {
        let rt_consumer = self
            .channel
            .basic_consume(
                self.queue.name().as_str(),
                "rt_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("basic_consume");

        for (channel, delivery) in rt_consumer.into_iter().flatten() {
            channel
                .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                .wait()
                .expect("ack"); // TODO : what to do when ack fail ?

            let proto_message = decode_amqp_rt_message(&delivery);
            match proto_message {
                Ok(proto_message) => {
                    self.proto_messages.push(proto_message)
                    // TODO :
                    // handle_realtime_message(&proto_message, &self.rt_model)
                }
                Err(err) => {
                    error!("{}", err.to_string())
                }
            }
        }

        // TODO : send self.proto_messages to master
    }

    pub fn listen(&self) {
        loop {
            self.consume();
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
    info!("channel created successfully");
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
