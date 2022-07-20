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

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum DateTimeRepresent {
    Departure,
    Arrival,
}

impl Default for DateTimeRepresent {
    fn default() -> Self {
        DateTimeRepresent::Departure
    }
}

impl std::fmt::Display for DateTimeRepresent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DateTimeRepresent::Departure => write!(f, "departure"),
            DateTimeRepresent::Arrival => write!(f, "arrival"),
        }
    }
}

impl std::str::FromStr for DateTimeRepresent {
    type Err = DateTimeRepresentConfigError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let request_type = match s {
            "departure" => DateTimeRepresent::Departure,
            "arrival" => DateTimeRepresent::Arrival,
            _ => {
                return Err(DateTimeRepresentConfigError {
                    datetime_represent_name: s.to_string(),
                })
            }
        };
        Ok(request_type)
    }
}

pub fn parse_datetime(string_datetime: &str) -> Result<loki::NaiveDateTime, BadDateTime> {
    let try_datetime = loki::NaiveDateTime::parse_from_str(string_datetime, "%Y%m%dT%H%M%S");
    match try_datetime {
        Ok(datetime) => Ok(datetime),
        Err(_) => {
            let err = BadDateTime {
                string_datetime: string_datetime.to_string(),
            };
            Err(err)
        }
    }
}

#[derive(Debug)]
pub struct BadDateTime {
    string_datetime: String,
}

impl std::error::Error for BadDateTime {}

impl std::fmt::Display for BadDateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Unable to parse {} as a datetime. Expected format is 20190628T163215",
            self.string_datetime
        )
    }
}

#[derive(Debug)]
pub struct DateTimeRepresentConfigError {
    datetime_represent_name: String,
}

impl std::error::Error for DateTimeRepresentConfigError {}

impl std::fmt::Display for DateTimeRepresentConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Bad datetime_represent : `{}`",
            self.datetime_represent_name
        )
    }
}
