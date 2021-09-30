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

pub mod chaos_proto {
    include!(concat!(env!("OUT_DIR"), "/mod.rs"));
}
pub use chaos_proto::*;

use super::navitia_proto;

use failure::{format_err, Error};
use lapin::{
    options::*, types::FieldTable, Channel, Connection, ConnectionProperties, ExchangeKind, Queue,
};
use launch::loki::realtime::rt_model::UpdateType::{Delete, Update};
use launch::loki::realtime::rt_model::{
    DateTimePeriod, DeleteInfo, RealTimeModel, RealTimeUpdate, SeverityEffect, UpdateInfo,
};

use launch::loki::tracing::{error, info, trace, warn};
use launch::loki::NaiveDateTime;
use prost::Message as MessageTrait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
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
    rt_model: Arc<Mutex<RealTimeModel>>,
}

impl RealTimeWorker {
    pub fn new(config: &BrockerConfig, rt_model: Arc<Mutex<RealTimeModel>>) -> Result<Self, Error> {
        let connection = RealTimeWorker::create_connection(config)?;
        let channel = RealTimeWorker::create_channel(config, &connection)?;

        let rt_queue_name = format!("{}_rt", "loki_hostname");
        let queue_task = RealTimeWorker::declare_queue(config, &channel, rt_queue_name.as_str())?;
        RealTimeWorker::bind_queue(
            &channel,
            config.rt_topics.as_slice(),
            &config.exchange,
            rt_queue_name.as_str(),
        )?;

        let task_queue_name = format!("{}_task", "loki_hostname");
        let queue_rt = RealTimeWorker::declare_queue(config, &channel, task_queue_name.as_str())?;
        RealTimeWorker::bind_queue(&channel, &[], &config.exchange, task_queue_name.as_str())?;

        Ok(Self {
            connection,
            channel,
            queue_task,
            queue_rt,
            rt_model,
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

    fn create_channel(config: &BrockerConfig, connection: &Connection) -> Result<Channel, Error> {
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
        config: &BrockerConfig,
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

    fn consume(&self) {
        let rt_consumer = self
            .channel
            .basic_consume(
                self.queue_rt.name().as_str(),
                "rt_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .wait()
            .expect("basic_consume");

        for delivery in rt_consumer.into_iter().flatten() {
            let (channel, delivery) = delivery;
            channel
                .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                .wait()
                .expect("ack");

            let proto_message = decode_amqp_rt_message(&delivery);
            match proto_message {
                Ok(proto_message) => handle_realtime_message(&proto_message, &self.rt_model),
                Err(err) => {
                    error!("{}", err.to_string())
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

fn handle_realtime_message(
    proto: &gtfs_realtime::FeedMessage,
    rt_model: &Arc<Mutex<RealTimeModel>>,
) {
    info!("proto : {:?}", proto);

    let mut rt_model = rt_model.lock().unwrap();

    for entity in proto.entity.iter() {
        if entity.get_is_deleted() {
            unimplemented!();
        } else if let Some(disruption) = chaos::exts::disruption.get(entity) {
            let info_updates = chaos_updates(&disruption);
            rt_model.add_trip_update(info_updates);
        } else if entity.has_trip_update() {
            let info_updates = kirin_updates(entity, entity.get_trip_update());
        } else {
            warn!("Unsupported gtfs rt feed")
        }
    }
}

fn chaos_updates(disruption: &chaos::Disruption) -> Vec<RealTimeUpdate> {
    let mut trip_updates = Vec::new();

    for impact in disruption.get_impacts().iter() {
        for pt_object in impact.get_informed_entities().iter() {
            match pt_object.get_pt_object_type() {
                chaos::PtObject_Type::trip => {
                    let update_info = generate_delete_info(disruption.get_id(), impact, pt_object);
                    trip_updates.push(RealTimeUpdate::VehicleUpdate(Delete(update_info)))
                }
                chaos::PtObject_Type::route => {
                    let update_info = generate_delete_info(disruption.get_id(), impact, pt_object);
                    trip_updates.push(RealTimeUpdate::RouteUpdate(Delete(update_info)))
                }
                chaos::PtObject_Type::line => {
                    let update_info = generate_delete_info(disruption.get_id(), impact, pt_object);
                    trip_updates.push(RealTimeUpdate::LineUpdate(Delete(update_info)))
                }
                chaos::PtObject_Type::network => {
                    let update_info = generate_delete_info(disruption.get_id(), impact, pt_object);
                    trip_updates.push(RealTimeUpdate::NetworkUpdate(Delete(update_info)))
                }
                _ => (),
            }
        }
    }
    info!("updates {:?}", trip_updates);
    trip_updates
}

fn generate_delete_info(
    disruption_id: &str,
    impact: &chaos::Impact,
    pt_object: &chaos::PtObject,
) -> DeleteInfo {
    DeleteInfo {
        disruption_id: disruption_id.to_string(),
        pt_object_id: pt_object.get_uri().to_string(),
        severity_effect: make_severity_effect(&impact.get_severity().get_effect()),
        application_periods: impact
            .get_application_periods()
            .iter()
            .map(|period| make_datetime_period(period))
            .collect(),
    }
}

fn kirin_updates(
    entity: &gtfs_realtime::FeedEntity,
    trip_update: &gtfs_realtime::TripUpdate,
) -> RealTimeUpdate {
    let disruption_id = entity.get_id().to_string();
    if let Some(effect) = kirin::exts::effect.get(trip_update) {
        let effect = make_severity_effect(&effect);
        if let SeverityEffect::NoService = effect {
            let delete_info = DeleteInfo {
                disruption_id,
                pt_object_id: "".to_string(),
                severity_effect: effect,
                application_periods: vec![],
            };
            return RealTimeUpdate::VehicleUpdate(Delete(delete_info));
        } else {
        }
    } else {
        error!("Kirin message must have an effect extension")
    }

    let update_info = UpdateInfo { disruption_id };
    RealTimeUpdate::VehicleUpdate(Update(update_info))
}

fn make_severity_effect(proto_severity_effect: &gtfs_realtime::Alert_Effect) -> SeverityEffect {
    match proto_severity_effect {
        gtfs_realtime::Alert_Effect::NO_SERVICE => SeverityEffect::NoService,
        gtfs_realtime::Alert_Effect::REDUCED_SERVICE => SeverityEffect::ReducedService,
        gtfs_realtime::Alert_Effect::SIGNIFICANT_DELAYS => SeverityEffect::SignificantDelay,
        gtfs_realtime::Alert_Effect::DETOUR => SeverityEffect::Detour,
        gtfs_realtime::Alert_Effect::ADDITIONAL_SERVICE => SeverityEffect::AdditionalService,
        gtfs_realtime::Alert_Effect::MODIFIED_SERVICE => SeverityEffect::ModifiedService,
        gtfs_realtime::Alert_Effect::OTHER_EFFECT => SeverityEffect::OtherEffect,
        gtfs_realtime::Alert_Effect::UNKNOWN_EFFECT => SeverityEffect::UnknownEffect,
        gtfs_realtime::Alert_Effect::STOP_MOVED => SeverityEffect::StopMoved,
    }
}

fn make_datetime_period(proto_period: &gtfs_realtime::TimeRange) -> DateTimePeriod {
    DateTimePeriod {
        start: NaiveDateTime::from_timestamp(proto_period.get_start() as i64, 0),
        end: NaiveDateTime::from_timestamp(proto_period.get_end() as i64, 0),
    }
}
