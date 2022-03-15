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

use super::InputDataType;
use loki::PositiveDuration;
use structopt::StructOpt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, StructOpt, Clone)]
#[structopt(rename_all = "snake_case")]
pub struct LaunchParams {
    /// directory containing ntfs/gtfs files to load
    #[structopt(long)]
    pub input_data_path: std::path::PathBuf,

    /// type of input data given (ntfs/gtfs)
    #[structopt(long, default_value)]
    #[serde(default)]
    pub input_data_type: InputDataType,

    /// path to the passengers loads file
    #[structopt(long)]
    pub loads_data_path: Option<std::path::PathBuf>,

    /// the transfer duration between a stop point and itself
    #[structopt(long, default_value = DEFAULT_TRANSFER_DURATION)]
    #[serde(default = "default_transfer_duration")]
    pub default_transfer_duration: PositiveDuration,

    #[structopt(skip)]
    pub bucket_params: BucketParams,
}

pub const DEFAULT_TRANSFER_DURATION: &str = "00:01:00";

pub fn default_transfer_duration() -> PositiveDuration {
    use std::str::FromStr;
    PositiveDuration::from_str(DEFAULT_TRANSFER_DURATION).unwrap()
}

impl LaunchParams {
    pub fn new(input_data_path: std::path::PathBuf) -> Self {
        Self {
            input_data_path,
            input_data_type: InputDataType::Ntfs,
            default_transfer_duration: default_transfer_duration(),
            loads_data_path: None,
            bucket_params: BucketParams::default(),
        }
    }
}

pub enum StorageType {
    Local(LocalDataFiles),
    Object(BucketParams),
}

#[derive(Debug, Serialize, Deserialize, StructOpt, Clone)]
#[structopt(rename_all = "snake_case")]
pub struct LocalDataFiles {
    #[structopt(long)]
    pub input_data_path: std::path::PathBuf,

    #[structopt(long)]
    pub loads_data_path: Option<std::path::PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Clone, StructOpt)]
pub struct BucketParams {
    #[serde(default = "default_bucket_name")]
    pub bucket_name: String,

    #[serde(default = "default_bucket_region")]
    pub bucket_region: String,

    #[serde(default = "default_bucket_access_key")]
    pub bucket_access_key: String,

    #[serde(default = "default_bucket_secret_key")]
    pub bucket_secret_key: String,

    #[serde(default)]
    pub s3_data_path: String,

    #[serde(default)]
    pub s3_load_data_path: Option<String>,
}

impl Default for BucketParams {
    fn default() -> Self {
        BucketParams {
            bucket_name: default_bucket_name(),
            bucket_region: default_bucket_region(),
            bucket_access_key: default_bucket_access_key(),
            bucket_secret_key: default_bucket_secret_key(),
            s3_data_path: "".to_string(),
            s3_load_data_path: None,
        }
    }
}

pub fn default_bucket_name() -> String {
    "loki".to_string()
}

pub fn default_bucket_access_key() -> String {
    "".to_string()
}

pub fn default_bucket_secret_key() -> String {
    "".to_string()
}

pub fn default_bucket_region() -> String {
    "eu-west-1".to_string()
}
