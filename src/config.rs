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

use std::{fmt::Display, str::FromStr};

use serde::Deserialize;

use crate::PositiveDuration;

pub const DEFAULT_TRANSFER_DURATION: &str = "00:01:00";

pub fn default_transfer_duration() -> PositiveDuration {
    PositiveDuration::from_str(DEFAULT_TRANSFER_DURATION).unwrap()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataImplem {
    Periodic,
    Daily,
    LoadsPeriodic,
    LoadsDaily,
}
impl std::str::FromStr for DataImplem {
    type Err = DataImplemConfigError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use DataImplem::*;
        let implem = match s {
            "periodic" => Periodic,
            "daily" => Daily,
            "loads_periodic" => LoadsPeriodic,
            "loads_daily" => LoadsDaily,
            _ => {
                return Err(DataImplemConfigError {
                    implem_name: s.to_string(),
                })
            }
        };
        Ok(implem)
    }
}

#[derive(Debug)]
pub struct DataImplemConfigError {
    implem_name: String,
}

impl std::fmt::Display for DataImplem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use DataImplem::*;
        match self {
            Periodic => write!(f, "periodic"),
            Daily => write!(f, "daily"),
            LoadsPeriodic => write!(f, "loads_periodic"),
            LoadsDaily => write!(f, "loads_daily"),
        }
    }
}

impl std::fmt::Display for DataImplemConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad implem configuration given : `{}`", self.implem_name)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriteriaImplem {
    Loads,
    Basic,
}
impl std::str::FromStr for CriteriaImplem {
    type Err = CriteriaImplemConfigError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let request_type = match s {
            "loads" => CriteriaImplem::Loads,
            "classic" => CriteriaImplem::Basic,
            _ => {
                return Err(CriteriaImplemConfigError {
                    criteria_implem_name: s.to_string(),
                })
            }
        };
        Ok(request_type)
    }
}

impl std::fmt::Display for CriteriaImplem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CriteriaImplem::Loads => write!(f, "loads"),
            CriteriaImplem::Basic => write!(f, "basic"),
        }
    }
}

#[derive(Debug)]
pub struct CriteriaImplemConfigError {
    criteria_implem_name: String,
}

impl std::fmt::Display for CriteriaImplemConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Bad criteria_implem given : `{}`",
            self.criteria_implem_name
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparatorType {
    Loads,
    Basic,
}
impl std::str::FromStr for ComparatorType {
    type Err = RequestTypeConfigError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let request_type = match s {
            "loads" => ComparatorType::Loads,
            "basic" => ComparatorType::Basic,
            _ => {
                return Err(RequestTypeConfigError {
                    request_type_name: s.to_string(),
                })
            }
        };
        Ok(request_type)
    }
}

impl std::fmt::Display for ComparatorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComparatorType::Loads => write!(f, "loads"),
            ComparatorType::Basic => write!(f, "basic"),
        }
    }
}

#[derive(Debug)]
pub struct RequestTypeConfigError {
    request_type_name: String,
}

impl std::fmt::Display for RequestTypeConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad request type given : `{}`", self.request_type_name)
    }
}

pub const DEFAULT_LEG_ARRIVAL_PENALTY: &str = "00:02:00";
pub const DEFAULT_LEG_WALKING_PENALTY: &str = "00:02:00";
pub const DEFAULT_MAX_NB_LEGS: &str = "10";
pub const DEFAULT_MAX_JOURNEY_DURATION: &str = "24:00:00";

pub fn default_leg_arrival_penalty() -> PositiveDuration {
    PositiveDuration::from_str(DEFAULT_LEG_ARRIVAL_PENALTY).unwrap()
}

pub fn default_leg_walking_penalty() -> PositiveDuration {
    PositiveDuration::from_str(DEFAULT_LEG_WALKING_PENALTY).unwrap()
}

pub fn default_max_nb_of_legs() -> u8 {
    u8::from_str(DEFAULT_MAX_NB_LEGS).unwrap()
}

pub fn default_max_journey_duration() -> PositiveDuration {
    PositiveDuration::from_str(DEFAULT_MAX_JOURNEY_DURATION).unwrap()
}

pub struct RequestParams {
    /// penalty to apply to arrival time for each vehicle leg in a journey
    pub leg_arrival_penalty: PositiveDuration,

    /// penalty to apply to walking time for each vehicle leg in a journey
    pub leg_walking_penalty: PositiveDuration,

    /// maximum number of vehicle legs in a journey
    pub max_nb_of_legs: u8,

    /// maximum duration of a journey
    pub max_journey_duration: PositiveDuration,
}

impl Default for RequestParams {
    fn default() -> Self {
        let max_nb_of_legs: u8 = FromStr::from_str(DEFAULT_MAX_NB_LEGS).unwrap();
        Self {
            leg_arrival_penalty: FromStr::from_str(DEFAULT_LEG_ARRIVAL_PENALTY).unwrap(),
            leg_walking_penalty: FromStr::from_str(DEFAULT_LEG_WALKING_PENALTY).unwrap(),
            max_nb_of_legs,
            max_journey_duration: FromStr::from_str(DEFAULT_MAX_JOURNEY_DURATION).unwrap(),
        }
    }
}

impl Display for RequestParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "--leg_arrival_penalty {} --leg_walking_penalty {} --max_nb_of_legs {} --max_journey_duration {}",
                self.leg_arrival_penalty,
                self.leg_walking_penalty,
                self.max_nb_of_legs,
                self.max_journey_duration
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputType {
    Gtfs,
    Ntfs,
}

impl FromStr for InputType {
    type Err = InputTypeConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let result = match s {
            "ntfs" => InputType::Ntfs,
            "gtfs" => InputType::Gtfs,
            _ => {
                return Err(InputTypeConfigError {
                    input_type_name: s.to_string(),
                })
            }
        };
        Ok(result)
    }
}

impl std::fmt::Display for InputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputType::Gtfs => write!(f, "gtfs"),
            InputType::Ntfs => write!(f, "ntfs"),
        }
    }
}

#[derive(Debug)]
pub struct InputTypeConfigError {
    input_type_name: String,
}

impl std::fmt::Display for InputTypeConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad input data type give : `{}`", self.input_type_name)
    }
}
