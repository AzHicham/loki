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

use loki::{
    models::base_model::BaseModel, NaiveDateTime, PositiveDuration, RealTimeLevel, RequestInput,
};

use crate::config::RequestParams;

pub fn make_query_stop_areas(
    base_model: &BaseModel,
    datetime: &NaiveDateTime,
    from_stop_area: &str,
    to_stop_area: &str,
    request_params: &RequestParams,
) -> Result<RequestInput, UnknownStopArea> {
    let departures_stop_point_and_fallback_duration =
        stops_of_stop_area(base_model, from_stop_area, PositiveDuration::zero())?;
    let arrivals_stop_point_and_fallback_duration =
        stops_of_stop_area(base_model, to_stop_area, PositiveDuration::zero())?;

    let request_input = RequestInput {
        datetime: *datetime,
        departures_stop_point_and_fallback_duration,
        arrivals_stop_point_and_fallback_duration,
        leg_arrival_penalty: request_params.leg_arrival_penalty,
        leg_walking_penalty: request_params.leg_walking_penalty,
        max_nb_of_legs: request_params.max_nb_of_legs,
        max_journey_duration: request_params.max_journey_duration,
        too_late_threshold: request_params.too_late_threshold,
        real_time_level: RealTimeLevel::Base,
    };

    Ok(request_input)
}

pub fn stops_of_stop_area(
    base_model: &BaseModel,
    stop_area_uri: &str,
    duration_to_stops: PositiveDuration,
) -> Result<Vec<(String, PositiveDuration)>, UnknownStopArea> {
    let mut result = Vec::new();
    for stop_idx in base_model.stop_points() {
        let stop_area_name = base_model.stop_area_name(stop_idx);
        let stop_name = base_model.stop_point_name(stop_idx);
        if stop_area_name == stop_area_uri {
            result.push((stop_name.to_string(), duration_to_stops));
        }
    }

    Ok(result)
}

#[derive(Debug)]
pub struct UnknownStopArea {
    uri: String,
}

impl std::error::Error for UnknownStopArea {}

impl std::fmt::Display for UnknownStopArea {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown stop area : `{}`", self.uri)
    }
}
