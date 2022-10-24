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

use anyhow::bail;

use loki_launch::config::parse_env_var;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, str::FromStr};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ChaosParams {
    /// connection string to the chaos database
    /// for example : "postgres://guest:guest@localhost:5432/chaos"
    pub database: String,

    /// During reload of chaos disruption
    /// we will ask the database to send
    /// blocks of rows of size `batch_size`
    #[serde(default = "default_batch_size")]
    pub batch_size: u32,
}

pub fn default_batch_size() -> u32 {
    1_000_000
}

impl ChaosParams {
    pub fn new_from_env_vars() -> Result<Option<Self>, anyhow::Error> {
        let database = match std::env::var("LOKI_CHAOS_DATABASE") {
            Ok(s) => s,
            Err(std::env::VarError::NotPresent) => {
                // the variable is not set, so it means that chaos should not be used
                return Ok(None);
            }
            Err(std::env::VarError::NotUnicode(err)) => {
                bail!("Badly formed LOKI_CHAOS_DATABASE  : {:?}", err);
            }
        };

        let batch_size =
            parse_env_var("LOKI_CHAOS_BATCH_SIZE", default_batch_size(), u32::from_str);

        Ok(Some(Self {
            database,
            batch_size,
        }))
    }
}
