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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum InputDataType {
    Gtfs,
    Ntfs,
}

impl std::str::FromStr for InputDataType {
    type Err = InputDataTypeConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let result = match s {
            "ntfs" => InputDataType::Ntfs,
            "gtfs" => InputDataType::Gtfs,
            _ => {
                return Err(InputDataTypeConfigError {
                    input_type_name: s.to_string(),
                })
            }
        };
        Ok(result)
    }
}

impl std::fmt::Display for InputDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputDataType::Gtfs => write!(f, "gtfs"),
            InputDataType::Ntfs => write!(f, "ntfs"),
        }
    }
}

#[derive(Debug)]
pub struct InputDataTypeConfigError {
    input_type_name: String,
}

impl std::fmt::Display for InputDataTypeConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad input data type give : `{}`", self.input_type_name)
    }
}
