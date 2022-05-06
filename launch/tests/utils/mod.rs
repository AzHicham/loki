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
#![allow(dead_code)]
pub mod disruption_builder;
pub mod model_builder;

use anyhow::{format_err, Error};
use launch::{
    config,
    config::launch_params::default_transfer_duration,
    datetime::DateTimeRepresent,
    loki::{response, response::VehicleSection, RequestInput},
    solver::Solver,
};
use loki::{
    chrono::TimeZone,
    filters::{parse_filter, Filters},
    models::ModelRefs,
    RealTimeLevel,
};

use loki::{chrono_tz, tracing::debug};

use loki::{NaiveDateTime, PositiveDuration, TransitData};
use model_builder::AsDateTime;

pub struct Config<'a> {
    pub request_params: config::RequestParams,

    pub datetime: NaiveDateTime,

    pub datetime_represent: DateTimeRepresent,

    pub comparator_type: config::ComparatorType,

    pub default_transfer_duration: PositiveDuration,

    /// name of the start stop_area
    pub start: String,

    /// name of the end stop_area
    pub end: String,

    pub allowed_uris: Vec<&'a str>,

    pub forbidden_uris: Vec<&'a str>,

    pub wheelchair_accessible: bool,
    pub bike_accessible: bool,
}

impl<'a> Config<'a> {
    pub fn new(datetime: impl AsDateTime, start: &str, end: &str) -> Self {
        Self::new_timezoned(datetime, model_builder::DEFAULT_TIMEZONE, start, end)
    }

    pub fn new_timezoned(
        datetime: impl AsDateTime,
        timezone: chrono_tz::Tz,
        start: &str,
        end: &str,
    ) -> Self {
        let naive_datetime = datetime.as_datetime();
        let timezoned_datetime = timezone.from_local_datetime(&naive_datetime).unwrap();
        let utc_datetime = timezoned_datetime.naive_utc();
        Config {
            request_params: Default::default(),
            datetime: utc_datetime,
            datetime_represent: Default::default(),
            comparator_type: Default::default(),
            default_transfer_duration: default_transfer_duration(),
            start: start.into(),
            end: end.into(),
            allowed_uris: Default::default(),
            forbidden_uris: Default::default(),
            bike_accessible: false,
            wheelchair_accessible: false,
        }
    }
}

pub fn make_request_from_config(config: &Config) -> RequestInput {
    let datetime = config.datetime;

    let start_stop_point_uri = &config.start;
    let end_stop_point_uri = &config.end;

    let request_input = RequestInput {
        datetime,
        departures_stop_point_and_fallback_duration: vec![(
            start_stop_point_uri.clone(),
            PositiveDuration::zero(),
        )],
        arrivals_stop_point_and_fallback_duration: vec![(
            end_stop_point_uri.clone(),
            PositiveDuration::zero(),
        )],
        leg_arrival_penalty: config.request_params.leg_arrival_penalty,
        leg_walking_penalty: config.request_params.leg_walking_penalty,
        max_nb_of_legs: config.request_params.max_nb_of_legs,
        max_journey_duration: config.request_params.max_journey_duration,
        too_late_threshold: config.request_params.too_late_threshold,
        real_time_level: config.request_params.real_time_level,
    };
    request_input
}

pub fn build_and_solve(
    model: &ModelRefs<'_>,
    config: &Config,
) -> Result<Vec<response::Response>, Error> {
    use loki::DataTrait;
    let data: TransitData = launch::read::build_transit_data(model.base);

    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    let forbidden_filters = config
        .forbidden_uris
        .iter()
        .filter_map(|forbidden_uri| parse_filter(model, forbidden_uri, "test"));

    let allowed_filters = config
        .allowed_uris
        .iter()
        .filter_map(|forbidden_uri| parse_filter(model, forbidden_uri, "test"));

    let filters = Filters::new(
        forbidden_filters,
        allowed_filters,
        config.wheelchair_accessible,
        config.bike_accessible,
    );

    let request_input = make_request_from_config(config);

    let responses = solver.solve_journey_request(
        &data,
        model,
        &request_input,
        filters,
        &config.comparator_type,
        &config.datetime_represent,
    )?;
    for response in responses.iter() {
        debug!("{}", response.print(model)?);
    }
    Ok(responses)
}

pub fn from_to_stop_point_names<'a>(
    vehicle_section: &VehicleSection,
    model: &'a ModelRefs<'a>,
    real_time_level: RealTimeLevel,
) -> Result<(&'a str, &'a str), Error> {
    let from_stop_name = vehicle_section
        .from_stop_point_name(model, real_time_level)
        .ok_or_else(|| {
            format_err!(
                "No stoptime at idx {:?} for vehicle journey {}",
                vehicle_section.from_stoptime_idx,
                model.vehicle_journey_name(&vehicle_section.vehicle_journey)
            )
        })?;
    let to_stop_name = vehicle_section
        .to_stop_point_name(model, real_time_level)
        .ok_or_else(|| {
            format_err!(
                "No stoptime at idx {:?} for vehicle journey {}",
                vehicle_section.to_stoptime_idx,
                model.vehicle_journey_name(&vehicle_section.vehicle_journey)
            )
        })?;

    Ok((from_stop_name, to_stop_name))
}
