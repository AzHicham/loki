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

use loki_launch::{
    config::{launch_params::LocalFileParams, parse_env_var, read_env_var},
    loki::PositiveDuration,
};
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

    #[serde(default = "default_bucket_timeout")]
    pub bucket_timeout: PositiveDuration,
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

pub fn default_bucket_timeout() -> PositiveDuration {
    PositiveDuration::from_hms(0, 0, 30)
}

impl BucketParams {
    pub fn default() -> Self {
        BucketParams {
            bucket_name: default_bucket_name(),
            bucket_region: default_bucket_region(),
            bucket_access_key: default_bucket_access_key(),
            bucket_secret_key: default_bucket_secret_key(),
            data_path_key: default_data_path(),
            bucket_timeout: default_bucket_timeout(),
        }
    }

    pub fn new_from_env_vars() -> Self {
        let bucket_name =
            read_env_var("LOKI_BUCKET_NAME", default_bucket_name(), |s| s.to_string());

        let bucket_region = read_env_var("LOKI_BUCKET_REGION", default_bucket_region(), |s| {
            s.to_string()
        });
        let bucket_access_key =
            read_env_var("LOKI_BUCKET_ACCESS_KEY", default_bucket_access_key(), |s| {
                s.to_string()
            });
        let bucket_secret_key =
            read_env_var("LOKI_BUCKET_SECRET_KEY", default_bucket_secret_key(), |s| {
                s.to_string()
            });

        let data_path_key = read_env_var("LOKI_BUCKET_DATA_PATH", default_data_path(), |s| {
            s.to_string()
        });

        let bucket_timeout = parse_env_var(
            "LOKI_BUCKET_TIMEOUT",
            default_bucket_timeout(),
            PositiveDuration::from_str,
        );

        Self {
            bucket_name,
            bucket_region,
            bucket_access_key,
            bucket_secret_key,
            data_path_key,
            bucket_timeout,
        }
    }
}
