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

use crate::filters::parse_filter;
use crate::{
    filters::{Filter, Filters, StopFilter, VehicleFilter},
    models::{ModelRefs, StopPointIdx, StopTimeIdx, VehicleJourneyIdx},
    transit_data::data_interface,
    PositiveDuration, RealTimeLevel,
};
use chrono::{NaiveDate, NaiveDateTime};
use std::fmt;
use tracing::warn;

#[derive(Debug)]
pub enum NextStopTimeError {
    BadDateTimeError,
}
impl std::error::Error for NextStopTimeError {}

impl fmt::Display for NextStopTimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NextStopTimeError::BadDateTimeError => write!(
                f,
                "The requested datetime is out of the validity period of the data."
            ),
        }
    }
}

pub struct NextStopTimeRequestInput<'a> {
    pub input_stop_points: Vec<StopPointIdx>,
    pub forbidden_vehicle: Option<Filters<'a>>,
    pub from_datetime: NaiveDateTime,
    pub until_datetime: NaiveDateTime,
    pub max_response: u32,
    pub real_time_level: RealTimeLevel,
}

pub struct NextStopTimeResponse {
    pub stop_point: StopPointIdx,
    pub vehicle_journey: VehicleJourneyIdx,
    pub boarding_time: NaiveDateTime,
    pub vehicle_date: NaiveDate,
    pub stop_time_idx: StopTimeIdx,
}

pub fn generate_stops_for_next_stoptimes_request<'a, 'data, T>(
    input_str: &'a T,
    forbidden_uri: &'a [T],
    model: &'data ModelRefs<'data>,
) -> Vec<StopPointIdx>
where
    T: AsRef<str>,
{
    if let Some(input_filter) =
        parse_filter(model, input_str.as_ref(), "next_stoptimes_request_input")
    {
        let mut stop_points = match input_filter {
            Filter::Stop(StopFilter::StopPoint(id)) => model
                .stop_point_idx(id)
                .map_or_else(Vec::new, |idx| vec![idx]),
            Filter::Stop(StopFilter::StopArea(id)) => model.stop_points_of_stop_area(id),
            Filter::Vehicle(VehicleFilter::Line(id)) => model.stop_points_of_line(id),
            Filter::Vehicle(VehicleFilter::Route(id)) => model.stop_points_of_route(id),
            Filter::Vehicle(VehicleFilter::Network(id)) => model.stop_points_of_network(id),
            Filter::Vehicle(VehicleFilter::PhysicalMode(id)) => {
                model.stop_points_of_physical_mode(id)
            }
            Filter::Vehicle(VehicleFilter::CommercialMode(id)) => {
                model.stop_points_of_commercial_mode(id)
            }
        };

        for uri in forbidden_uri {
            let filter = parse_filter(model, uri.as_ref(), "next_stoptimes_forbidden_uri");
            let forbidden_stops = match filter {
                Some(Filter::Stop(StopFilter::StopPoint(id))) => model
                    .stop_point_idx(id)
                    .map_or_else(Vec::new, |idx| vec![idx]),
                Some(Filter::Stop(StopFilter::StopArea(id))) => model.stop_points_of_stop_area(id),
                _ => vec![],
            };
            stop_points.retain(|idx| !forbidden_stops.contains(idx))
        }
        stop_points
    } else {
        vec![]
    }
}

pub fn next_departures<'data, 'filter, Data>(
    request: &'data NextStopTimeRequestInput<'filter>,
    data: &'data Data,
) -> Result<Vec<NextStopTimeResponse>, NextStopTimeError>
where
    Data: data_interface::Data + data_interface::DataIters<'data>,
{
    let mut response = Vec::new();

    let calendar = data.calendar();
    let from_datetime = calendar
        .from_naive_datetime(&request.from_datetime)
        .ok_or_else(|| {
            warn!(
                "The requested from_datetime {:?} is out of bound of the allowed dates. \
                Allowed dates are between {:?} and {:?}.",
                request.from_datetime,
                calendar.first_datetime(),
                calendar.last_datetime(),
            );
            NextStopTimeError::BadDateTimeError
        })?;
    let until_datetime = data
        .calendar()
        .from_naive_datetime(&request.until_datetime)
        .ok_or_else(|| {
            warn!(
                "The requested until_datetime {:?} is out of bound of the allowed dates. \
                Allowed dates are between {:?} and {:?}.",
                request.from_datetime,
                calendar.first_datetime(),
                calendar.last_datetime(),
            );
            NextStopTimeError::BadDateTimeError
        })?;

    for stop_idx in &request.input_stop_points {
        if let Some(stop) = data.stop_point_idx_to_stop(stop_idx) {
            for (mission, position) in data.missions_at(&stop) {
                let mut count = 0;
                let mut next_time = from_datetime;
                'inner: while count < request.max_response {
                    let earliest_trip_time = data.earliest_trip_to_board_at(
                        &next_time,
                        &mission,
                        &position,
                        &request.real_time_level,
                    );
                    match earliest_trip_time {
                        Some((trip, boarding_time, _)) if boarding_time < until_datetime => {
                            response.push(NextStopTimeResponse {
                                stop_point: stop_idx.clone(),
                                vehicle_journey: data.vehicle_journey_idx(&trip),
                                boarding_time: data.to_naive_datetime(&boarding_time),
                                vehicle_date: data.day_of(&trip),
                                stop_time_idx: data.stoptime_idx(&position, &trip),
                            });
                            count += 1;
                            next_time = boarding_time + PositiveDuration { seconds: 1 };
                        }
                        _ => {
                            break 'inner;
                        }
                    }
                }
            }
        }
    }

    response.sort_by(|lhs: &NextStopTimeResponse, rhs: &NextStopTimeResponse| {
        lhs.boarding_time.cmp(&rhs.boarding_time)
    });

    Ok(response
        .into_iter()
        .take(request.max_response as usize)
        .collect())
}
