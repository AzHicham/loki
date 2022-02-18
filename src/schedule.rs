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

use crate::filters::VehicleFilter;
use crate::{
    filters::{Filter, Filters, StopFilter},
    models::{ModelRefs, StopPointIdx, StopTimeIdx, VehicleJourneyIdx},
    transit_data::data_interface,
    PositiveDuration, RealTimeLevel,
};
use chrono::NaiveDateTime;

pub enum NextStopTimeError {
    BadDateTime,
}

pub struct NextStopTimeRequestInput<'a> {
    pub input: Filter<'a>,
    pub filters: Option<Filters<'a>>,
    pub from_datetime: NaiveDateTime,
    pub until_datetime: NaiveDateTime,
    pub nb_stoptimes: u32,
    pub real_time_level: RealTimeLevel,
    pub start_page: usize,
    pub count: usize,
}

pub struct NextStopTimeResponse {
    pub stop_point: StopPointIdx,
    pub vehicle_journey: VehicleJourneyIdx,
    pub datetime: NaiveDateTime,
    pub stop_time_idx: StopTimeIdx,
}

pub fn generate_stop_for_next_stoptimes<'a, 'data>(
    input: &Filter<'a>,
    model: &'data ModelRefs<'data>,
) -> Option<Vec<StopPointIdx>> {
    match input {
        Filter::Stop(StopFilter::StopPoint(id)) => model.stop_point_idx(id).map(|idx| vec![idx]),
        Filter::Stop(StopFilter::StopArea(id)) => Some(model.stop_points_of_stop_area(id)),
        Filter::Vehicle(VehicleFilter::Line(id)) => Some(model.stop_points_of_line(id)),
        Filter::Vehicle(VehicleFilter::Route(id)) => Some(model.stop_points_of_route(id)),
        Filter::Vehicle(VehicleFilter::Network(id)) => Some(model.stop_points_of_network(id)),
        Filter::Vehicle(VehicleFilter::PhysicalMode(id)) => {
            Some(model.stop_points_of_physical_mode(id))
        }
        Filter::Vehicle(VehicleFilter::CommercialMode(id)) => {
            Some(model.stop_points_of_commercial_mode(id))
        }
    }
}

pub fn next_departures<'data, 'filter, Data>(
    request: &'data NextStopTimeRequestInput<'filter>,
    model: &'data ModelRefs<'data>,
    data: &'data Data,
) -> Vec<NextStopTimeResponse>
where
    Data: data_interface::Data + data_interface::DataIters<'data>,
{
    let mut response = Vec::new();

    let from_datetime = data
        .calendar()
        .from_naive_datetime(&request.from_datetime)
        .unwrap();
    let until_datetime = data
        .calendar()
        .from_naive_datetime(&request.until_datetime)
        .unwrap();
    let stop_points_idx = generate_stop_for_next_stoptimes(&request.input, model);

    if let Some(stop_points_idx) = stop_points_idx {
        for stop_idx in stop_points_idx {
            if let Some(stop) = data.stop_point_idx_to_stop(&stop_idx) {
                for (mission, position) in data.missions_at(&stop) {
                    let mut count = 0;
                    let mut next_time = from_datetime;
                    'outer: while count < request.nb_stoptimes {
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
                                    datetime: data.to_naive_datetime(&boarding_time),
                                    stop_time_idx: data.stoptime_idx(&position, &trip),
                                });
                                count += 1;
                                next_time = boarding_time + PositiveDuration { seconds: 1 };
                            }
                            _ => {
                                break 'outer;
                            }
                        }
                    }
                }
            }
        }
    }

    response.sort_by(|lhs: &NextStopTimeResponse, rhs: &NextStopTimeResponse| {
        lhs.datetime.cmp(&rhs.datetime)
    });
    response
        .into_iter()
        .take(request.nb_stoptimes as usize)
        .collect()
}
