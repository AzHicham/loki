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

pub mod chaos_params;
pub mod data_source_params;
pub mod http_params;
pub mod rabbitmq_params;

use anyhow::{Context, Error};
use loki_launch::{
    config,
    config::{parse_env_var, RequestParams},
    loki::PositiveDuration,
};

use loki_launch::config::{
    launch_params::{default_transfer_duration, LocalFileParams},
    InputDataType,
};

use serde::{Deserialize, Serialize};
use std::{fmt::Debug, str::FromStr};
use tracing::warn;

use self::{
    chaos_params::ChaosParams, data_source_params::DataSourceParams, http_params::HttpParams,
    rabbitmq_params::RabbitMqParams,
};

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

    /// Configures the connection to a chaos database that will be used
    /// to retreive the history of chaos disruptions when the public transport data is (re)loaded
    /// If None, the retreival of past chaos disruptions will be disabled.
    /// Defaults to None.
    #[serde(default)]
    pub chaos: Option<ChaosParams>,

    /// Configures the http endpoint for status and health checks
    #[serde(default)]
    pub http: HttpParams,
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
            http: HttpParams::default(),
            instance_name: instance_name.to_string(),
            default_request_params: config::RequestParams::default(),
            rabbitmq: RabbitMqParams::default(),
            chaos: None,
            nb_workers: default_nb_workers(),
        }
    }

    pub fn new_from_env_vars() -> Result<Self, Error> {
        let instance_name = std::env::var("LOKI_INSTANCE_NAME")
            .context("Could not read mandatory env var LOKI_INSTANCE_NAME")?;

        let requests_socket = std::env::var("LOKI_REQUESTS_SOCKET")
            .context("Could not read read mandatory env var LOKI_REQUESTS_SOCKET")?;

        let input_data_type = parse_env_var(
            "LOKI_INPUT_DATA_TYPE",
            InputDataType::default(),
            InputDataType::from_str,
        );

        let default_transfer_duration = parse_env_var(
            "LOKI_DEFAULT_TRANSFER_DURATION",
            default_transfer_duration(),
            PositiveDuration::from_str,
        );

        let nb_workers = parse_env_var("LOKI_NB_WORKERS", default_nb_workers(), u16::from_str);

        let data_source = DataSourceParams::new_from_env_vars()
            .context("Could not read DataSourceParams from env vars")?;

        let default_request_params = RequestParams::new_from_env_vars();

        let rabbitmq = RabbitMqParams::new_from_env_vars();

        let chaos = ChaosParams::new_from_env_vars().unwrap_or_else(|err| {
            warn!("Error reading chaos configuration from env vars. I'll keep running without chaos. {:?}. ", err);
            None
        });

        let http = HttpParams::new_from_env_vars();

        Ok(Self {
            instance_name,
            requests_socket,
            input_data_type,
            default_transfer_duration,
            nb_workers,
            data_source,
            default_request_params,
            rabbitmq,
            chaos,
            http,
        })
    }
}

pub fn default_nb_workers() -> u16 {
    1
}

#[cfg(test)]
mod tests {

    use crate::server_config::{data_source_params::DataSourceParams, ServerConfig};

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
            "Error while reading config file {:?} : {:#?}",
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

    #[test]
    fn test_vars_empty() {
        let params = ServerConfig::new_from_env_vars();
        assert!(params.is_err());
    }

    #[test]
    fn test_instance_name_only() {
        temp_env::with_var("LOKI_INSTANCE_NAME", Some("my-instance"), || {
            let params = ServerConfig::new_from_env_vars();

            assert!(
                params.is_err(),
                "Error reading config from env vars {:?}",
                params
            );
        })
    }

    #[test]
    fn test_no_data_source_type() {
        temp_env::with_vars(
            vec![
                ("LOKI_INSTANCE_NAME", Some("my-instance")),
                ("LOKI_REQUESTS_SOCKET", Some("tcp://127.0.0.1:30001")),
            ],
            || {
                let params = ServerConfig::new_from_env_vars();

                assert!(
                    params.is_err(),
                    "Error reading config from env vars {:?}",
                    params
                );
            },
        )
    }

    #[test]
    fn test_no_data_path_type() {
        temp_env::with_vars(
            vec![
                ("LOKI_INSTANCE_NAME", Some("my-instance")),
                ("LOKI_REQUESTS_SOCKET", Some("tcp://127.0.0.1:30001")),
                ("LOKI_DATA_SOURCE_TYPE", Some("local")),
            ],
            || {
                let params = ServerConfig::new_from_env_vars();

                assert!(
                    params.is_err(),
                    "Error reading config from env vars {:?}",
                    params
                );
            },
        )
    }

    #[test]
    fn test_env_vars_minimal() {
        temp_env::with_vars(
            vec![
                ("LOKI_INSTANCE_NAME", Some("my-instance")),
                ("LOKI_REQUESTS_SOCKET", Some("tcp://127.0.0.1:30001")),
                ("LOKI_DATA_SOURCE_TYPE", Some("local")),
                ("LOKI_INPUT_DATA_PATH", Some("/path/to/my/ntfs")),
            ],
            || {
                let params = ServerConfig::new_from_env_vars();

                assert!(
                    params.is_ok(),
                    "Error reading config from env vars {:?}",
                    params
                );
                let params = params.unwrap();
                assert_eq!(&params.instance_name, "my-instance");
                assert_eq!(&params.requests_socket, "tcp://127.0.0.1:30001");
                match params.data_source {
                    DataSourceParams::Local(local_file_params) => {
                        assert_eq!(
                            local_file_params.input_data_path,
                            PathBuf::from("/path/to/my/ntfs")
                        )
                    }
                    _ => {
                        panic!("Error reading config from env vars");
                    }
                }
            },
        )
    }
}
