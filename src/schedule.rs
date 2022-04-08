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

use std::fmt;

use crate::{
    filters::{parse_filter, Filter, Filters, StopFilter, VehicleFilter},
    models::{ModelRefs, StopPointIdx, StopTimeIdx, VehicleJourneyIdx},
    transit_data::data_interface,
    transit_data_filtered::FilterMemory,
    RealTimeLevel, TransitData,
};
use chrono::{NaiveDate, NaiveDateTime};

use tracing::warn;

#[derive(Clone)]
pub enum ScheduleOn {
    BoardTimes,
    DebarkTimes,
}

pub struct ScheduleRequestInput {
    pub input_stop_points: Vec<StopPointIdx>,
    pub from_datetime: NaiveDateTime,
    pub until_datetime: NaiveDateTime,
    pub real_time_level: RealTimeLevel,
    pub nb_max_responses: usize,
    pub schedule_on: ScheduleOn,
}

#[derive(Debug)]
pub enum ScheduleRequestError {
    BadFromDatetime,
    BadUntilDatetime,
}

impl std::error::Error for ScheduleRequestError {}

impl fmt::Display for ScheduleRequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ScheduleRequestError::BadFromDatetime => {
                write!(f, "The requested from datetime is invalid.")
            }
            ScheduleRequestError::BadUntilDatetime => {
                write!(f, "The requested until datetime is invalid.")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScheduleResponse {
    pub stop_point_idx: StopPointIdx,
    pub vehicle_journey_idx: VehicleJourneyIdx,
    pub vehicle_date: NaiveDate,
    pub time: NaiveDateTime,
    pub stop_time_idx: StopTimeIdx,
}

pub fn generate_stops_for_schedule_request<T>(
    input_str: &str,
    forbidden_uris: &[T],
    model: &ModelRefs<'_>,
) -> Vec<StopPointIdx>
where
    T: AsRef<str>,
{
    if let Some(input_filter) = parse_filter(model, input_str, "next_stoptimes_request_input") {
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

        for forbidden_uri in forbidden_uris {
            let filter = parse_filter(
                model,
                forbidden_uri.as_ref(),
                "next_stoptimes_forbidden_uri",
            );
            match filter {
                Some(Filter::Stop(StopFilter::StopPoint(stop_id))) => {
                    stop_points.retain(|stop_idx| model.stop_point_id(stop_idx) != stop_id);
                }

                Some(Filter::Stop(StopFilter::StopArea(stop_area_id))) => {
                    stop_points.retain(|stop_idx| model.stop_area_id(stop_idx) != stop_area_id);
                }
                Some(_) => {
                    warn!("Unexpected forbidden_uri {} provided in a next_stop_time request. I'm gonna ignore it.", forbidden_uri.as_ref());
                }
                _ => {
                    warn!("Bad forbidden_uri {} provided in a next_stop_time request. I'm gonna ignore it.", forbidden_uri.as_ref());
                }
            };
        }
        stop_points
    } else {
        vec![]
    }
}

pub fn generate_vehicle_filters_for_schedule_request<'a, T>(
    forbidden_uris: &'a [T],
    model: &ModelRefs<'_>,
) -> Option<Filters<'a>>
where
    T: AsRef<str>,
{
    let forbidden_vehicles = forbidden_uris.iter().filter_map(|forbidden_uri| {
        match parse_filter(
            model,
            forbidden_uri.as_ref(),
            "generate_vehicle_filters_for_schedule_request",
        ) {
            Some(Filter::Vehicle(filter)) => Some(Filter::Vehicle(filter)),
            _ => None,
        }
    });
    Filters::new(forbidden_vehicles, std::iter::empty(), false, false)
}

pub fn solve_schedule_request(
    request: &ScheduleRequestInput,
    data: &TransitData,
    model: &ModelRefs<'_>,
    has_filter_memory: Option<&FilterMemory>,
) -> Result<Vec<ScheduleResponse>, ScheduleRequestError> {
    use data_interface::Data;

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
            ScheduleRequestError::BadFromDatetime
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
            ScheduleRequestError::BadUntilDatetime
        })?;

    let mut all_responses = Vec::new();
    let mut responses_at_current_stop = Vec::new();
    for stop_point_idx in &request.input_stop_points {
        let stop = if let Some(stop) = data.stop_point_idx_to_stop(stop_point_idx) {
            stop
        } else {
            let stop_point_name = model.stop_point_name(stop_point_idx);
            warn!("The stop point {stop_point_name} requested for schedule is not found in transit_data. I ignore it.");
            continue;
        };
        responses_at_current_stop.clear();
        for (mission, position) in data.missions_of(stop) {
            match request.schedule_on {
                ScheduleOn::BoardTimes => {
                    let trip_iter = data.trips_boardable_between(
                        from_datetime,
                        until_datetime,
                        &mission,
                        &position,
                        request.real_time_level,
                    );
                    let response_iter = trip_iter.filter_map(|trip| {
                        let vehicle_journey_idx = data.vehicle_journey_idx(&trip);
                        if let Some(filter_memory) = has_filter_memory {
                            if !filter_memory.is_vehicle_journey_allowed(&vehicle_journey_idx) {
                                return None;
                            }
                        }
                        let vehicle_date = data.day_of(&trip);
                        let time = data.board_time_of(&trip, &position).map(
                            |(second_since_dataset_start, _)| {
                                data.calendar()
                                    .to_naive_datetime(second_since_dataset_start)
                            },
                        )?;
                        let stop_time_idx = data.stoptime_idx(&position, &trip);
                        Some(ScheduleResponse {
                            stop_point_idx: stop_point_idx.clone(),
                            vehicle_journey_idx,
                            vehicle_date,
                            time,
                            stop_time_idx,
                        })
                    });
                    let response_iter = response_iter.take(request.nb_max_responses);
                    responses_at_current_stop.extend(response_iter);
                }
                ScheduleOn::DebarkTimes => {
                    let trip_iter = data.trips_debarkable_between(
                        from_datetime,
                        until_datetime,
                        &mission,
                        &position,
                        request.real_time_level,
                    );
                    let response_iter = trip_iter.filter_map(|trip| {
                        let vehicle_journey_idx = data.vehicle_journey_idx(&trip);
                        if let Some(filter_memory) = has_filter_memory {
                            if !filter_memory.is_vehicle_journey_allowed(&vehicle_journey_idx) {
                                return None;
                            }
                        }
                        let vehicle_date = data.day_of(&trip);
                        let time = data.debark_time_of(&trip, &position).map(
                            |(second_since_dataset_start, _)| {
                                data.calendar()
                                    .to_naive_datetime(second_since_dataset_start)
                            },
                        )?;
                        let stop_time_idx = data.stoptime_idx(&position, &trip);
                        Some(ScheduleResponse {
                            stop_point_idx: stop_point_idx.clone(),
                            vehicle_journey_idx,
                            vehicle_date,
                            time,
                            stop_time_idx,
                        })
                    });
                    let response_iter = response_iter.take(request.nb_max_responses);
                    responses_at_current_stop.extend(response_iter);
                }
            }
        }

        responses_at_current_stop.sort_unstable_by_key(|response| response.time);
        // we may obtain twice the same (vehicle_journey, day) because of local zones
        // when that happens, it will be at the same stop_time_idx AND at the same response.time
        // Since responses_at_current_stop is sorted by response.time, two copies of the same (vehicle_journey, day)
        // will appears consecutively in the vector, and we may use dedup()
        // to remove duplicate
        responses_at_current_stop.dedup_by(|resp_a, resp_b| {
            resp_a.vehicle_journey_idx == resp_b.vehicle_journey_idx
                && resp_a.vehicle_date == resp_b.vehicle_date
                && resp_a.stop_time_idx == resp_b.stop_time_idx
        });

        all_responses.extend_from_slice(&responses_at_current_stop);
    }

    all_responses.sort_unstable_by_key(|response| response.time);

    all_responses.truncate(request.nb_max_responses);

    Ok(all_responses)
}
