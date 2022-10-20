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

use anyhow::{Context, Error};

use loki_launch::config::launch_params::LocalFileParams;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, str::FromStr};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DataSourceParams {
    Local(LocalFileParams),
    S3(BucketParams),
}

impl DataSourceParams {
    pub fn new_from_env_vars() -> Result<Self, Error> {
        let data_source_type = std::env::var("LOKI_DATA_SOURCE_TYPE")
            .context("Could not read mandatory env var LOKI_DATA_SOURCE_TYPE")?;
        match data_source_type.trim() {
            "s3" => {
                let bucket_params = BucketParams::new_from_env_vars();
                Ok(DataSourceParams::S3(bucket_params))
            }
            "local" => {
                let local_file_params = LocalFileParams::new_from_env_vars()
                    .context("LOKI_DATA_SOURCE_TYPE is set to 'local' but I could not read local file params from env vars")?;
                Ok(DataSourceParams::Local(local_file_params))
            }
            _ => {
                anyhow::bail!(
                    "Bad LOKI_DATA_SOURCE_TYPE : '{}'. Allowed values are 's3' or 'local'",
                    data_source_type
                );
            }
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

    #[serde(default = "default_bucket_access_key")]
    pub bucket_access_key: String,

    #[serde(default = "default_bucket_secret_key")]
    pub bucket_secret_key: String,

    #[serde(default = "default_data_path")]
    pub data_path_key: String,

    #[serde(default = "default_bucket_timeout_in_ms")]
    pub bucket_timeout_in_ms: u32,
}

pub fn default_bucket_name() -> String {
    "loki".to_string()
}

pub fn default_bucket_region() -> String {
    "eu-west-1".to_string()
}

pub fn default_bucket_access_key() -> String {
    "".to_string()
}

pub fn default_bucket_secret_key() -> String {
    "".to_string()
}

pub fn default_data_path() -> String {
    "".to_string()
}

pub fn default_bucket_timeout_in_ms() -> u32 {
    30_000
}

impl BucketParams {
    pub fn default() -> Self {
        BucketParams {
            bucket_name: default_bucket_name(),
            bucket_region: default_bucket_region(),
            bucket_access_key: default_bucket_access_key(),
            bucket_secret_key: default_bucket_secret_key(),
            data_path_key: default_data_path(),
            bucket_timeout_in_ms: default_bucket_timeout_in_ms(),
        }
    }

    pub fn new_from_env_vars() -> Self {
        let bucket_name =
            std::env::var("LOKI_BUCKET_NAME").unwrap_or_else(|_| default_bucket_name());
        let bucket_region =
            std::env::var("LOKI_BUCKET_REGION").unwrap_or_else(|_| default_bucket_region());
        let bucket_access_key =
            std::env::var("LOKI_BUCKET_ACCESS_KEY").unwrap_or_else(|_| default_bucket_access_key());
        let bucket_secret_key =
            std::env::var("LOKI_BUCKET_SECRET_KEY").unwrap_or_else(|_| default_bucket_secret_key());
        let data_path_key =
            std::env::var("LOKI_BUCKET_DATA_PATH").unwrap_or_else(|_| default_data_path());
        let bucket_timeout_in_ms = {
            let string = std::env::var("LOKI_BUCKET_TIMEOUT_IN_MS").unwrap_or_default();
            u32::from_str(&string).unwrap_or_else(|_| default_bucket_timeout_in_ms())
        };

        Self {
            bucket_name,
            bucket_region,
            bucket_access_key,
            bucket_secret_key,
            data_path_key,
            bucket_timeout_in_ms,
        }
    }
}
