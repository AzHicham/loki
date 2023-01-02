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

use std::path::PathBuf;

use super::{read_env_var, InputDataType};
use anyhow::Context;
use loki::PositiveDuration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct LaunchParams {
    /// directory containing ntfs/gtfs files to load
    pub input_data_path: std::path::PathBuf,

    /// type of input data given (ntfs/gtfs)
    #[serde(default)]
    pub input_data_type: InputDataType,

    /// path to the passengers' occupancy file
    pub occupancy_data_path: Option<std::path::PathBuf>,

    /// the transfer duration between a stop point and itself
    #[serde(default = "default_transfer_duration")]
    pub default_transfer_duration: PositiveDuration,
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
            occupancy_data_path: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct LocalFileParams {
    pub input_data_path: std::path::PathBuf,
    pub occupancy_data_path: Option<std::path::PathBuf>,
}

impl LocalFileParams {
    pub fn new_from_env_vars() -> Result<Self, anyhow::Error> {
        let input_data_path = std::env::var("LOKI_INPUT_DATA_PATH")
            .map(PathBuf::from)
            .context("Could not read mandatory env var LOKI_INPUT_DATA_PATH")?;

        let occupancy_data_path =
            read_env_var("LOKI_OCCUPANCY_DATA_PATH", None, |s| Some(PathBuf::from(s)));

        Ok(Self {
            input_data_path,
            occupancy_data_path,
        })
    }
}
