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

pub mod navitia_proto {
    include!(concat!(env!("OUT_DIR"), "/pbnavitia.rs"));
}
pub mod chaos_proto {
    include!(concat!(env!("OUT_DIR"), "/mod.rs"));
}

pub use chaos_proto::*;

use failure::{format_err, Error};
use lapin::{options::*, types::FieldTable, Channel, Connection, ConnectionProperties, Queue};
use launch::loki::tracing::{error, info, trace, warn};
use prost::Message;
use protobuf::parse_from_bytes;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

pub fn default_host() -> String {
    "localhost".to_string()
}
pub fn default_username() -> String {
    "guest".to_string()
}
pub fn default_password() -> String {
    "guest".to_string()
}
pub fn default_vhost() -> String {
    "/".to_string()
}
pub fn default_exchange() -> String {
    "navitia".to_string()
}
pub fn default_port() -> u16 {
    5672
}
pub fn default_rt_topics() -> Vec<String> {
    Vec::new()
}
pub fn default_queue_auto_delete() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, StructOpt, Clone)]
#[structopt(rename_all = "snake_case")]
pub struct BrockerConfig {
    #[structopt(long, default_value = "default_host")]
    #[serde(default = "default_host")]
    pub host: String,

    #[structopt(long, default_value = "default_port")]
    #[serde(default = "default_port")]
    pub port: u16,

    #[structopt(long, default_value = "default_username")]
    #[serde(default = "default_username")]
    pub username: String,

    #[structopt(long, default_value = "default_password")]
    #[serde(default = "default_password")]
    pub password: String,

    #[structopt(long, default_value = "default_vhost")]
    #[serde(default = "default_vhost")]
    pub vhost: String,

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

pub struct RealTimeWorker {
    connection: Connection,
    channel: Channel,
    queue_task: Queue,
    queue_rt: Queue,
}

impl RealTimeWorker {
    pub fn new(config: &BrockerConfig) -> Result<Self, Error> {
        let connection = RealTimeWorker::create_connection(config)?;
        let channel = RealTimeWorker::create_channel(&connection)?;
        let queue_task =
            RealTimeWorker::declare_queue(&channel, format!("{}_task", "loki_hostname").as_str())?;
        let queue_rt =
            RealTimeWorker::declare_queue(&channel, format!("{}_rt", "loki_hostname").as_str())?;
        Ok(Self {
            connection,
            channel,
            queue_task,
            queue_rt,
        })
    }

    fn create_connection(config: &BrockerConfig) -> Result<Connection, Error> {
        let address = format!(
            "amqp://{}:{}@{}:{}{}",
            config.username, config.password, config.host, config.port, config.vhost
        );
        info!("Connection to rabbitmq {}", address);
        let connection =
            Connection::connect(address.as_str(), ConnectionProperties::default()).wait()?;
        info!("connected to rabbitmq {} successfully", address);
        Ok(connection)
    }

    fn create_channel(connection: &Connection) -> Result<Channel, Error> {
        let channel = connection.create_channel().wait()?;
        info!("channel created successfully");
        Ok(channel)
    }

    fn declare_queue(channel: &Channel, queue_name: &str) -> Result<Queue, Error> {
        let queue = channel
            .queue_declare(
                queue_name,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .wait()?;
        Ok(queue)
    }

    fn consume(&self) {
        let task_consumer = self
            .channel
            .basic_consume(
                self.queue_task.name().as_str(),
                "my_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .wait()
            .expect("basic_consume");

        for delivery in task_consumer {
            info!("received message: {:?}", delivery);
            if let Ok((_, delivery)) = delivery {
                println!("{:?}", delivery);

                let proto_message = decode_amqp_task_message(&delivery);
                match proto_message {
                    Ok(proto_message) => handle_task_message(&proto_message),
                    Err(err) => {
                        error!("{}", err.to_string())
                    }
                }
            }
        }
    }

    pub fn listen(&self) {
        loop {
            self.consume();
        }
    }
}

fn decode_amqp_rt_message(
    message: &lapin::message::Delivery,
) -> Result<gtfs_realtime::FeedMessage, Error> {
    let payload = &message.data;
    parse_from_bytes::<gtfs_realtime::FeedMessage>(&payload[..]).map_err(|err| {
        format_err!(
            "Could not decode rabbitmq realtime message into protobuf: \n {}",
            err
        )
    })
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

fn handle_realtime_message(proto: &gtfs_realtime::FeedMessage) {
    for entity in proto.entity.iter() {
        if entity.get_is_deleted() {
            // delete_disruption(entity.id)
            unimplemented!();
        } else if entity.alert.is_some()
        /* has extension disruption */
        {
            // apply_disruption(entity.disruption)
            todo!();
        } else if entity.trip_update.is_some() {
            unimplemented!();
        } else {
            warn!("Unsupported gtfs rt feed")
        }
    }
}
