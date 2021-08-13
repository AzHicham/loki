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

use failure::{format_err, Error};
use launch::config;
use launch::datetime::DateTimeRepresent;
use launch::loki::response::VehicleSection;
use launch::loki::{response, Idx, RequestInput, StopPoint};
use launch::solver::Solver;
use loki::log::info;
use loki::transit_model::Model;
use loki::DataWithIters;
use loki::{LoadsData, PositiveDuration};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use structopt::StructOpt;

pub const DEFAULT_TRANSFER_DURATION: &str = "00:01:00";

pub fn default_transfer_duration() -> PositiveDuration {
    use std::str::FromStr;
    PositiveDuration::from_str(DEFAULT_TRANSFER_DURATION).unwrap()
}

#[derive(Serialize, Deserialize, StructOpt)]
#[structopt(rename_all = "snake_case")]
pub struct Config {
    #[serde(flatten)]
    #[structopt(flatten)]
    pub request_params: config::RequestParams,

    #[structopt(long)]
    pub datetime: String,

    #[serde(default)]
    #[structopt(long, default_value)]
    pub datetime_represent: DateTimeRepresent,

    #[serde(default)]
    #[structopt(long, default_value)]
    pub comparator_type: config::ComparatorType,

    #[structopt(long, default_value = DEFAULT_TRANSFER_DURATION)]
    #[serde(default = "default_transfer_duration")]
    transfer_duration: PositiveDuration,

    /// name of the start stop_area
    #[structopt(long)]
    pub start: String,

    /// name of the end stop_area
    #[structopt(long)]
    pub end: String,
}

impl Config {
    pub fn new(datetime: String, start: String, end: String) -> Self {
        Config {
            request_params: Default::default(),
            datetime,
            datetime_represent: Default::default(),
            comparator_type: Default::default(),
            transfer_duration: default_transfer_duration(),
            start,
            end,
        }
    }
}

fn make_request_from_config(config: &Config) -> Result<RequestInput, Error> {
    let datetime = launch::datetime::parse_datetime(&config.datetime)?;

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

pub fn build_and_solve<Data>(
    model: &Model,
    loads_data: &LoadsData,
    config: &Config,
) -> Result<Vec<response::Response>, Error>
where
    Data: DataWithIters,
{
    info!("Transit model loaded");
    info!(
        "Number of vehicle journeys : {}",
        model.vehicle_journeys.len()
    );
    info!("Number of routes : {}", model.routes.len());
    let data_timer = SystemTime::now();
    let data = Data::new(model, loads_data, config.transfer_duration);
    let data_build_duration = data_timer.elapsed().unwrap().as_millis();

    info!("Data constructed in {} ms", data_build_duration);
    info!("Number of missions {} ", data.nb_of_missions());
    info!("Number of trips {} ", data.nb_of_trips());
    info!(
        "Validity dates between {} and {}",
        data.calendar().first_date(),
        data.calendar().last_date()
    );

    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    let request_input = make_request_from_config(config)?;

    let responses = solver.solve_request(
        &data,
        model,
        &request_input,
        &config.comparator_type,
        &config.datetime_represent,
    )?;
    Ok(responses)
}

pub fn make_pt_from_vehicle<'a>(
    vehicle_section: &VehicleSection,
    model: &'a Model,
) -> Result<(&'a StopPoint, &'a StopPoint), Error> {
    let vehicle_journey = &model.vehicle_journeys[vehicle_section.vehicle_journey];

    let from_stoptime_idx = vehicle_section.from_stoptime_idx;
    let from_stoptime = vehicle_journey
        .stop_times
        .get(from_stoptime_idx)
        .ok_or_else(|| {
            format_err!(
                "No stoptime at idx {} for vehicle journey {}",
                vehicle_section.from_stoptime_idx,
                vehicle_journey.id
            )
        })?;
    let from_stop_point_idx = from_stoptime.stop_point_idx;
    let from_stop_point = make_stop_point(from_stop_point_idx, model);

    let to_stoptime_idx = vehicle_section.to_stoptime_idx;
    let to_stoptime = vehicle_journey
        .stop_times
        .get(to_stoptime_idx)
        .ok_or_else(|| {
            format_err!(
                "No stoptime at idx {} for vehicle journey {}",
                vehicle_section.from_stoptime_idx,
                vehicle_journey.id
            )
        })?;
    let to_stop_point_idx = to_stoptime.stop_point_idx;
    let to_stop_point = make_stop_point(to_stop_point_idx, model);

    Ok((from_stop_point, to_stop_point))
}

pub fn make_stop_point(stop_point_idx: Idx<StopPoint>, model: &Model) -> &StopPoint {
    &model.stop_points[stop_point_idx]
}
