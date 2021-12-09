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

use launch::{config, loki::PositiveDuration};

use serde::{Deserialize, Serialize};
use std::{fmt::Debug, str::FromStr};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfig {
    #[serde(flatten)]
    pub launch_params: config::LaunchParams,

    /// zmq socket to listen for protobuf requests
    pub requests_socket: String,

    pub instance_name: String,

    #[serde(flatten)]
    pub request_default_params: config::RequestParams,

    #[serde(flatten)]
    pub rabbitmq_params: RabbitMqParams,

    /// number of workers that solve requests in parallel
    #[serde(default = "default_nb_workers")]
    pub nb_workers: usize,
}

impl ServerConfig {
    pub fn new(input_data_path: std::path::PathBuf, zmq_socket: &str, instance_name: &str) -> Self {
        Self {
            launch_params: config::LaunchParams::new(input_data_path),
            requests_socket: zmq_socket.to_string(),
            instance_name: instance_name.to_string(),
            request_default_params: config::RequestParams::default(),
            rabbitmq_params: RabbitMqParams::default(),
            nb_workers: default_nb_workers(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RabbitMqParams {
    #[serde(default = "default_rabbitmq_endpoint")]
    pub rabbitmq_endpoint: String,

    #[serde(default = "default_rabbitmq_exchange")]
    pub rabbitmq_exchange: String,

    #[serde(default = "default_rabbitmq_real_time_topics")]
    pub rabbitmq_real_time_topics: Vec<String>,

    #[serde(default = "default_rabbitmq_queue_auto_delete")]
    pub rabbitmq_queue_auto_delete: bool,

    #[serde(default = "default_real_time_update_interval")]
    pub real_time_update_interval: PositiveDuration,

    #[serde(default = "default_rabbitmq_connect_retry_interval")]
    pub rabbitmq_connect_retry_interval: PositiveDuration,

    #[serde(default = "default_reload_request_time_to_live")]
    pub reload_request_time_to_live: PositiveDuration,

    #[serde(default = "default_reload_kirin_timeout")]
    pub reload_kirin_timeout: PositiveDuration,
}

pub fn default_nb_workers() -> usize {
    1
}

pub fn default_rabbitmq_endpoint() -> String {
    "amqp://guest:guest@rabbitmq:5672".to_string()
}

pub fn default_rabbitmq_exchange() -> String {
    "navitia".to_string()
}

pub fn default_rabbitmq_real_time_topics() -> Vec<String> {
    Vec::new()
}

pub fn default_rabbitmq_queue_auto_delete() -> bool {
    false
}

pub fn default_real_time_update_interval() -> PositiveDuration {
    PositiveDuration::from_str("00:00:30").unwrap()
}

pub fn default_rabbitmq_connect_retry_interval() -> PositiveDuration {
    PositiveDuration::from_str("00:00:10").unwrap()
}

pub fn default_reload_request_time_to_live() -> PositiveDuration {
    PositiveDuration::from_str("00:00:02").unwrap()
}

pub fn default_reload_kirin_timeout() -> PositiveDuration {
    PositiveDuration::from_str("00:00:10").unwrap()
}

impl Default for RabbitMqParams {
    fn default() -> Self {
        Self {
            rabbitmq_endpoint: default_rabbitmq_endpoint(),
            rabbitmq_exchange: default_rabbitmq_exchange(),
            rabbitmq_real_time_topics: default_rabbitmq_real_time_topics(),
            rabbitmq_queue_auto_delete: default_rabbitmq_queue_auto_delete(),
            real_time_update_interval: default_real_time_update_interval(),
            rabbitmq_connect_retry_interval: default_rabbitmq_connect_retry_interval(),
            reload_request_time_to_live: default_reload_request_time_to_live(),
            reload_kirin_timeout: default_reload_kirin_timeout(),
        }
    }
}
