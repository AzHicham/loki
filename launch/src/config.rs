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

pub mod comparator_type;
pub mod input_data_type;
pub mod launch_params;
pub mod request_params;

use std::fmt::{Debug, Display};

pub use comparator_type::ComparatorType;
pub use input_data_type::InputDataType;
pub use launch_params::LaunchParams;
use loki::tracing::warn;
pub use request_params::RequestParams;

// - var not set -> use default value
// - var set but non-unicode -> warn and use default value
// - var set but not parsable -> warn and use default value
pub fn parse_env_var<T, Parser, ParseErr>(var_name: &str, default_value: T, parser: Parser) -> T
where
    Parser: Fn(&str) -> Result<T, ParseErr>,
    ParseErr: Display,
    T: Debug,
{
    match std::env::var(var_name) {
        Ok(s) => match parser(&s) {
            Ok(val) => val,
            Err(err) => {
                warn!(
                    "Could not parse env var {} : {}. I'll use the default value '{:?}' instead",
                    var_name, err, default_value
                );
                default_value
            }
        },
        Err(std::env::VarError::NotPresent) => default_value,
        Err(std::env::VarError::NotUnicode(err)) => {
            warn!(
                "Badly formed env var {} : {:?}. I'll use the default value {:?} instead",
                var_name, err, default_value
            );
            default_value
        }
    }
}

// for infaillible parser
pub fn read_env_var<T, Parser>(var_name: &str, default_value: T, parser: Parser) -> T
where
    Parser: Fn(&str) -> T,
    T: Debug,
{
    parse_env_var(var_name, default_value, |s| -> Result<T, &'static str> {
        Ok(parser(s))
    })
}
