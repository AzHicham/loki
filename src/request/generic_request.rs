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

use crate::traits;
use crate::{
    loads_data::LoadsCount,
    time::{PositiveDuration, SecondsSinceDatasetUTCStart},
};

use log::warn;
use traits::{BadRequest, RequestTypes};
use transit_model::Model;

pub struct GenericRequest<'data, 'model, Data: traits::Data> {
    pub(super) transit_data: &'data Data,
    pub(super) model : & 'model Model,
    pub(super) departure_datetime: SecondsSinceDatasetUTCStart,
    pub(super) departures_stop_point_and_fallback_duration: Vec<(Data::Stop, PositiveDuration)>,
    pub(super) arrivals_stop_point_and_fallbrack_duration: Vec<(Data::Stop, PositiveDuration)>,
    pub(super) leg_arrival_penalty: PositiveDuration,
    pub(super) leg_walking_penalty: PositiveDuration,
    pub(super) max_arrival_time: SecondsSinceDatasetUTCStart,
    pub(super) max_nb_legs: u8,
}

impl<'data, 'model, Data> GenericRequest<'data, 'model, Data>
where
    Data: traits::Data,
{
    pub fn new(
        model: &'model transit_model::Model,
        transit_data: &'data Data,
        request_input: &traits::RequestInput,
    ) -> Result<Self, BadRequest>
    where
        Self: Sized,
    {
        let departure_datetime = transit_data
            .calendar()
            .from_naive_datetime(&request_input.departure_datetime)
            .ok_or_else(|| {
                warn!(
                    "The departure datetime {:?} is out of bound of the allowed dates. \
                    Allowed dates are between {:?} and {:?}.",
                    request_input.departure_datetime,
                    transit_data.calendar().first_datetime(),
                    transit_data.calendar().last_datetime(),
                );
                BadRequest::DepartureDatetime
            })?;

        let departures : Vec<_> = request_input.departures_stop_point_and_fallback_duration
            .iter()
            .enumerate()
            .filter_map(|(idx, (stop_point_uri, fallback_duration))| {
                let stop_point_uri = stop_point_uri.to_string();
                let stop_idx = model.stop_points.get_idx(&stop_point_uri).or_else(|| {
                    warn!(
                        "The {}th departure stop point {} is not found in model. \
                                I ignore it.",
                        idx, stop_point_uri
                    );
                    None
                })?;
                let stop = transit_data.stop_point_idx_to_stop(&stop_idx).or_else(|| {
                    warn!(
                        "The {}th departure stop point {} with idx {:?} is not found in transit_data. \
                            I ignore it",
                        idx, stop_point_uri, stop_idx
                    );
                    None
                })?;
                Some((stop, *fallback_duration))
            })
            .collect();
        if departures.is_empty() {
            return Err(BadRequest::NoValidDepartureStop);
        }

        let arrivals : Vec<_> = request_input.arrivals_stop_point_and_fallback_duration
            .iter()
            .enumerate()
            .filter_map(|(idx, (stop_point_uri, fallback_duration))| {
                let stop_point_uri = stop_point_uri.to_string();
                let stop_idx = model.stop_points.get_idx(&stop_point_uri).or_else(|| {
                    warn!(
                        "The {}th arrival stop point {} is not found in model. \
                                I ignore it.",
                        idx, stop_point_uri
                    );
                    None
                })?;
                let stop = transit_data.stop_point_idx_to_stop(&stop_idx).or_else(|| {
                    warn!(
                        "The {}th arrival stop point {} with idx {:?} is not found in transit_data. \
                            I ignore it",
                        idx, stop_point_uri, stop_idx
                    );
                    None
                })?;
                Some((stop, *fallback_duration))
            })
            .collect();

        if arrivals.is_empty() {
            return Err(BadRequest::NoValidArrivalStop);
        }

        let result = Self {
            transit_data,
            model,
            departure_datetime,
            departures_stop_point_and_fallback_duration: departures,
            arrivals_stop_point_and_fallbrack_duration: arrivals,
            leg_arrival_penalty: request_input.leg_arrival_penalty,
            leg_walking_penalty: request_input.leg_walking_penalty,
            max_arrival_time: departure_datetime + request_input.max_journey_duration,
            max_nb_legs: request_input.max_nb_of_legs,
        };

        Ok(result)
    }
}

use crate::response;
use crate::traits::Journey as PTJourney;
impl<'data, 'model, Data> GenericRequest<'data, 'model, Data>
where
    Data: traits::Data,
{
    pub fn create_response<R>(
        &self,
        pt_journey: &PTJourney<R>,
        loads_count: LoadsCount,
    ) -> Result<response::Journey<Data>, response::BadJourney<Data>>
    where
        R: RequestTypes<
            Departure = Departure,
            Arrival = Arrival,
            Trip = Data::Trip,
            Position = Data::Position,
            Transfer = Data::Transfer,
        >,
    {
        let departure_datetime = self.departure_datetime;
        let departure_idx = pt_journey.departure_leg.departure.idx;
        let departure_fallback_duration =
            &self.departures_stop_point_and_fallback_duration[departure_idx].1;

        let first_vehicle = response::VehicleLeg {
            trip: pt_journey.departure_leg.trip.clone(),
            board_position: pt_journey.departure_leg.board_position.clone(),
            debark_position: pt_journey.departure_leg.debark_position.clone(),
        };

        let arrival_fallback_duration =
            &self.arrivals_stop_point_and_fallbrack_duration[pt_journey.arrival.idx].1;

        let connections = pt_journey.connection_legs.iter().map(|connection_leg| {
            let transfer = connection_leg.transfer.clone();
            let vehicle_leg = response::VehicleLeg {
                trip: connection_leg.trip.clone(),
                board_position: connection_leg.board_position.clone(),
                debark_position: connection_leg.debark_position.clone(),
            };
            (transfer, vehicle_leg)
        });

        response::Journey::new(
            departure_datetime,
            *departure_fallback_duration,
            first_vehicle,
            connections,
            *arrival_fallback_duration,
            loads_count,
            &self.transit_data,
        )
    }
}

use crate::traits::Data as DataTrait;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Departure {
    pub(super) idx: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Arrival {
    pub(super) idx: usize,
}

impl<'data, 'model, Data> GenericRequest<'data, 'model, Data>
where
    Data: DataTrait,
{
    pub(super) fn departures(&self) -> Departures {
        let nb_of_departures = self.departures_stop_point_and_fallback_duration.len();
        Departures {
            inner: 0..nb_of_departures,
        }
    }

    pub(super) fn arrivals(&self) -> Arrivals {
        let nb_of_arrivals = self.arrivals_stop_point_and_fallbrack_duration.len();
        Arrivals {
            inner: 0..nb_of_arrivals,
        }
    }
}

pub struct Departures {
    pub(super) inner: std::ops::Range<usize>,
}

impl Iterator for Departures {
    type Item = Departure;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|idx| Departure { idx })
    }
}

pub struct Arrivals {
    pub(super) inner: std::ops::Range<usize>,
}

impl Iterator for Arrivals {
    type Item = Arrival;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|idx| Arrival { idx })
    }
}


impl<'data, 'model, Data> GenericRequest<'data, 'model, Data>
where
    Data: traits::Data,
{
    pub fn stop_name(&self, stop : & Data::Stop) -> String {
        let stop_point_idx = self.transit_data.stop_point_idx(stop);
        let stop_point = &self.model.stop_points[stop_point_idx];
        stop_point.id.clone()
    }

    pub fn trip_name(&self, trip : & Data::Trip) -> String {
        let vehicle_journey_idx = self.transit_data.vehicle_journey_idx(trip);
        let date = self.transit_data.day_of(trip);
        let vehicle_journey = &self.model.vehicle_journeys[vehicle_journey_idx];
        format!("{}_{}_{}", vehicle_journey.id, date.to_string(), vehicle_journey.route_id)
    }

    pub fn mission_name(&self, mission : & Data::Mission ) -> String {
        let mission_id = self.transit_data.mission_id(mission);
        format!("{}", mission_id)
    }

    pub fn position_name(&self, position : & Data::Position, mission : & Data::Mission) -> String {
        let stop = self.transit_data.stop_of(position, mission);
        let stop_name = self.stop_name(&stop);
        let mission_name = self.mission_name(mission);
        format!("{}_{}", stop_name, mission_name)
    }

}