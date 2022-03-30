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

use launch::config::{
    launch_params::{default_transfer_duration, LocalFileParams},
    InputDataType,
};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, str::FromStr};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    pub instance_name: String,

    /// zmq socket to listen for protobuf requests
    pub requests_socket: String,

    /// type of input data given (ntfs/gtfs)
    #[serde(default)]
    pub input_data_type: InputDataType,

    /// the transfer duration between a stop point and itself
    #[serde(default = "default_transfer_duration")]
    pub default_transfer_duration: PositiveDuration,

    /// number of workers that solve requests in parallel
    #[serde(default = "default_nb_workers")]
    pub nb_workers: u16,

    // param to load data from either local file or S3
    pub data_source: DataSourceParams,

    #[serde(default)]
    pub default_request_params: config::RequestParams,

    #[serde(default)]
    pub rabbitmq: RabbitMqParams,

    #[serde(default)]
    pub chaos_params: ChaosParams,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DataSourceParams {
    Local(LocalFileParams),
    S3(BucketParams),
}

impl ServerConfig {
    pub fn new(input_data_path: std::path::PathBuf, zmq_socket: &str, instance_name: &str) -> Self {
        Self {
            default_transfer_duration: default_transfer_duration(),
            data_source: DataSourceParams::Local(LocalFileParams {
                input_data_path,
                loads_data_path: None,
            }),
            input_data_type: Default::default(),
            requests_socket: zmq_socket.to_string(),
            instance_name: instance_name.to_string(),
            default_request_params: config::RequestParams::default(),
            rabbitmq: RabbitMqParams::default(),
            chaos_params: ChaosParams::default(),
            nb_workers: default_nb_workers(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct RabbitMqParams {
    #[serde(default = "default_rabbitmq_endpoint")]
    pub endpoint: String,

    #[serde(default = "default_rabbitmq_exchange")]
    pub exchange: String,

    #[serde(default = "default_rabbitmq_real_time_topics")]
    pub real_time_topics: Vec<String>,

    #[serde(default = "default_rabbitmq_queue_auto_delete")]
    pub queue_auto_delete: bool,

    #[serde(default = "default_real_time_update_interval")]
    pub real_time_update_interval: PositiveDuration,

    #[serde(default = "default_rabbitmq_connect_retry_interval")]
    pub connect_retry_interval: PositiveDuration,

    #[serde(default = "default_reload_request_time_to_live")]
    pub reload_request_time_to_live: PositiveDuration,

    #[serde(default = "default_reload_kirin_timeout")]
    pub reload_kirin_timeout: PositiveDuration,
}

pub fn default_nb_workers() -> u16 {
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
            endpoint: default_rabbitmq_endpoint(),
            exchange: default_rabbitmq_exchange(),
            real_time_topics: default_rabbitmq_real_time_topics(),
            queue_auto_delete: default_rabbitmq_queue_auto_delete(),
            real_time_update_interval: default_real_time_update_interval(),
            connect_retry_interval: default_rabbitmq_connect_retry_interval(),
            reload_request_time_to_live: default_reload_request_time_to_live(),
            reload_kirin_timeout: default_reload_kirin_timeout(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ChaosParams {
    #[serde(default = "default_chaos_database")]
    pub chaos_database: String,
    #[serde(default = "default_batch_size")]
    pub chaos_batch_size: u32,
}

pub fn default_chaos_database() -> String {
    "postgres://guest:guest@localhost:5432/chaos".to_string()
}

pub fn default_batch_size() -> u32 {
    5000
}

impl Default for ChaosParams {
    fn default() -> Self {
        Self {
            chaos_database: default_chaos_database(),
            chaos_batch_size: default_batch_size(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct BucketParams {
    #[serde(default = "default_bucket_name")]
    pub bucket_name: String,

    #[serde(default = "default_bucket_region")]
    pub bucket_region: String,

    #[serde(default)]
    pub bucket_access_key: String,

    #[serde(default)]
    pub bucket_secret_key: String,

    #[serde(default)]
    pub data_path_key: String,

    #[serde(default = "default_bucket_timeout_in_ms")]
    pub bucket_timeout_in_ms: u32,
}

impl Default for BucketParams {
    fn default() -> Self {
        BucketParams {
            bucket_name: default_bucket_name(),
            bucket_region: default_bucket_region(),
            bucket_access_key: "".to_string(),
            bucket_secret_key: "".to_string(),
            data_path_key: "".to_string(),
            bucket_timeout_in_ms: default_bucket_timeout_in_ms(),
        }
    }
}

pub fn default_bucket_name() -> String {
    "loki".to_string()
}

pub fn default_bucket_region() -> String {
    "eu-west-1".to_string()
}

pub fn default_bucket_timeout_in_ms() -> u32 {
    30_000
}

#[cfg(test)]
mod tests {

    use super::super::read_config;
    use std::{path::PathBuf, str::FromStr};

    #[test]
    fn test_config_for_data_in_local_folder() {
        let path = PathBuf::from_str(env!("CARGO_MANIFEST_DIR"))
            .unwrap()
            .join("config_files")
            .join("data_in_local_folder.toml");

        let read_result = read_config(&path);
        assert!(
            read_config(&path).is_ok(),
            "Error while reading config file {:?} : {:?}",
            &path,
            read_result
        );
    }

    #[test]
    fn test_config_for_data_in_s3() {
        let path = PathBuf::from_str(env!("CARGO_MANIFEST_DIR"))
            .unwrap()
            .join("config_files")
            .join("data_in_s3.toml");

        let read_result = read_config(&path);
        assert!(
            read_config(&path).is_ok(),
            "Error while reading config file {:?} : {:?}",
            &path,
            read_result
        );
    }

    #[test]
    fn test_typo_in_config() {
        let path = PathBuf::from_str(env!("CARGO_MANIFEST_DIR"))
            .unwrap()
            .join("config_files")
            .join("typo_in_config.toml");

        assert!(read_config(&path).is_err());
    }
}
