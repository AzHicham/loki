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
#![allow(dead_code)]
pub mod model_builder;

use env_logger::Env;
use failure::{format_err, Error};
use launch::{
    config,
    config::launch_params::default_transfer_duration,
    datetime::DateTimeRepresent,
    loki::{
        response,
        response::VehicleSection,
        timetables::{Timetables as TimetablesTrait, TimetablesIter},
        Idx, RequestInput, StopPoint,
    },
    solver::Solver,
};
use loki::{chrono::TimeZone, filters::Filters, realtime::real_time_model::RealTimeModel};

use loki::{chrono_tz, tracing::debug};

use loki::{
    transit_model::Model, DailyData, LoadsData, NaiveDateTime, PeriodicData, PeriodicSplitVjData,
    PositiveDuration, TransitData, VehicleJourney,
};
use model_builder::AsDateTime;
use std::fmt::Debug;

pub fn init_logger() {
    let _ = env_logger::Builder::from_env(
        // use log level specified by RUST_LOG env var if set
        //  and default to the "debug" level when RUST_LOG is not set
        Env::default().default_filter_or("debug"),
    )
    .is_test(true)
    .try_init();
}

pub struct Config<'a> {
    pub request_params: config::RequestParams,

    pub datetime: NaiveDateTime,

    pub datetime_represent: DateTimeRepresent,

    pub comparator_type: config::ComparatorType,

    pub data_implem: config::DataImplem,

    pub default_transfer_duration: PositiveDuration,

    /// name of the start stop_area
    pub start: String,

    /// name of the end stop_area
    pub end: String,

    // Allowed_uri
    pub allowed_uri: Vec<&'a str>,

    // Forbidden_uri
    pub forbidden_uri: Vec<&'a str>,
}

impl<'a> Config<'a> {
    pub fn new(datetime: impl AsDateTime, start: &str, end: &str) -> Self {
        Self::new_timezoned(datetime, &model_builder::DEFAULT_TIMEZONE, start, end)
    }

    pub fn new_timezoned(
        datetime: impl AsDateTime,
        timezone: &chrono_tz::Tz,
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
            data_implem: Default::default(),
            default_transfer_duration: default_transfer_duration(),
            start: start.into(),
            end: end.into(),
            allowed_uri: Default::default(),
            forbidden_uri: Default::default(),
        }
    }
}

pub fn make_request_from_config(config: &Config) -> Result<RequestInput, Error> {
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
    };
    Ok(request_input)
}

pub fn build_and_solve(
    real_time_model: &RealTimeModel,
    model: &Model,
    loads_data: &LoadsData,
    config: &Config,
) -> Result<Vec<response::Response>, Error> {
    match config.data_implem {
        config::DataImplem::Periodic => {
            build_and_solve_inner::<PeriodicData>(real_time_model, model, loads_data, config)
        }
        config::DataImplem::Daily => {
            build_and_solve_inner::<DailyData>(real_time_model, model, loads_data, config)
        }
        config::DataImplem::PeriodicSplitVj => {
            build_and_solve_inner::<PeriodicSplitVjData>(real_time_model, model, loads_data, config)
        }
    }
}

fn build_and_solve_inner<Timetables>(
    real_time_model: &RealTimeModel,
    model: &Model,
    loads_data: &LoadsData,
    config: &Config,
) -> Result<Vec<response::Response>, Error>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
    Timetables::Mission: 'static,
    Timetables::Position: 'static,
{
    use loki::DataTrait;
    let data: TransitData<Timetables> =
        launch::read::build_transit_data(model, loads_data, &config.default_transfer_duration);

    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    let filters = Filters::new(model, &config.forbidden_uri, &config.allowed_uri);

    let request_input = make_request_from_config(config)?;

    let responses = solver.solve_request(
        &data,
        real_time_model,
        model,
        &request_input,
        filters,
        &config.comparator_type,
        &config.datetime_represent,
    )?;
    for response in responses.iter() {
        debug!("{}", response.print(real_time_model, model)?);
    }
    Ok(responses)
}

pub fn from_to_stop_point_names<'a>(
    vehicle_section: &VehicleSection,
    real_time_model: &'a RealTimeModel,
    model: &'a Model,
) -> Result<(&'a str, &'a str), Error> {
    let from_stop_name = vehicle_section
        .from_stop_point_name(real_time_model, model)
        .ok_or_else(|| {
            format_err!(
                "No stoptime at idx {} for vehicle journey {}",
                vehicle_section.from_stoptime_idx,
                real_time_model.vehicle_journey_name(&vehicle_section.vehicle_journey, &model)
            )
        })?;
    let to_stop_name = vehicle_section
        .to_stop_point_name(real_time_model, model)
        .ok_or_else(|| {
            format_err!(
                "No stoptime at idx {} for vehicle journey {}",
                vehicle_section.to_stoptime_idx,
                real_time_model.vehicle_journey_name(&vehicle_section.vehicle_journey, &model)
            )
        })?;

    Ok((from_stop_name, to_stop_name))
}
