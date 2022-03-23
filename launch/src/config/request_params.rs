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

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use structopt::StructOpt;

use loki::{PositiveDuration, RealTimeLevel};
#[derive(Debug, Clone, Serialize, Deserialize, StructOpt)]
#[structopt(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct RequestParams {
    /// penalty to apply to arrival time for each vehicle leg in a journey
    #[structopt(long, default_value = DEFAULT_LEG_ARRIVAL_PENALTY)]
    #[serde(default = "default_leg_arrival_penalty")]
    pub leg_arrival_penalty: PositiveDuration,

    /// penalty to apply to walking time for each vehicle leg in a journey
    #[structopt(long, default_value = DEFAULT_LEG_WALKING_PENALTY)]
    #[serde(default = "default_leg_walking_penalty")]
    pub leg_walking_penalty: PositiveDuration,

    /// maximum number of vehicle legs in a journey
    #[structopt(long, default_value = DEFAULT_MAX_NB_LEGS)]
    #[serde(default = "default_max_nb_of_legs")]
    pub max_nb_of_legs: u8,

    /// maximum duration of a journey
    #[structopt(long, default_value = DEFAULT_MAX_JOURNEY_DURATION)]
    #[serde(default = "default_max_journey_duration")]
    pub max_journey_duration: PositiveDuration,

    /// discard any journey that arrive later than
    /// the arrival time of another journey + too_late_threshold
    #[structopt(long, default_value = DEFAULT_TOO_LATE_THRESHOLD)]
    #[serde(default = "default_too_late_threshold")]
    pub too_late_threshold: PositiveDuration,

    /// Which version of the data to use for computing journeys ?
    /// base : the initial scheduled provided in the ntfs, disregarding real time updates
    /// real_time : includes the real time updates
    #[structopt(long, default_value = DEFAULT_REAL_TIME_LEVEL)]
    #[serde(default = "default_real_time_level")]
    pub real_time_level: RealTimeLevel,
}

pub const DEFAULT_LEG_ARRIVAL_PENALTY: &str = "00:02:00";
pub const DEFAULT_LEG_WALKING_PENALTY: &str = "00:02:00";
pub const DEFAULT_MAX_NB_LEGS: &str = "10";
pub const DEFAULT_MAX_JOURNEY_DURATION: &str = "24:00:00";
pub const DEFAULT_TOO_LATE_THRESHOLD: &str = "02:00:00";
pub const DEFAULT_REAL_TIME_LEVEL: &str = "base";

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

pub fn default_too_late_threshold() -> PositiveDuration {
    PositiveDuration::from_str(DEFAULT_TOO_LATE_THRESHOLD).unwrap()
}

pub fn default_real_time_level() -> RealTimeLevel {
    RealTimeLevel::Base
}

impl Default for RequestParams {
    fn default() -> Self {
        Self {
            leg_arrival_penalty: default_leg_arrival_penalty(),
            leg_walking_penalty: default_leg_walking_penalty(),
            max_nb_of_legs: default_max_nb_of_legs(),
            max_journey_duration: default_max_journey_duration(),
            too_late_threshold: default_too_late_threshold(),
            real_time_level: default_real_time_level(),
        }
    }
}
