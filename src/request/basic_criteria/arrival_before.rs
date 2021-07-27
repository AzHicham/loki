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

use crate::response;
use crate::{loads_data::LoadsCount, time::PositiveDuration};

use crate::engine::engine_interface::{
    BadRequest, Journey as PTJourney, RequestInput, RequestTypes,
};
use crate::transit_data::data_interface::{Data as DataTrait, DataIters};

use super::super::generic_request::{Arrival, Arrivals, Departure, Departures, GenericRequest};
use super::Criteria;

pub mod classic_comparator;

pub struct GenericBasicArrivalBefore<'data, 'model, Data: DataTrait> {
    generic: GenericRequest<'data, 'model, Data>,
}

impl<'data, 'model, Data: DataTrait> GenericBasicArrivalBefore<'data, 'model, Data> {
    pub fn leg_arrival_penalty(&self) -> PositiveDuration {
        self.generic.leg_arrival_penalty
    }

    pub fn leg_walking_penalty(&self) -> PositiveDuration {
        self.generic.leg_walking_penalty
    }

    fn is_valid(&self, criteria: &Criteria) -> bool {
        criteria.arrival_time <= self.generic.max_arrival_time
            && criteria.nb_of_legs <= self.generic.max_nb_legs
    }

    fn board_and_ride(
        &self,
        position: &Data::Position,
        trip: &Data::Trip,
        waiting_criteria: &Criteria,
    ) -> Option<Criteria> {
        let has_debark = self.generic.transit_data.debark_time_of(trip, position);
        if let Some(debark_timeload) = has_debark {
            if waiting_criteria.arrival_time > debark_timeload.0 {
                return None;
            }
        } else {
            return None;
        }
        let mission = self.generic.transit_data.mission_of(trip);
        let previous_position = self
            .generic
            .transit_data
            .previous_on_mission(position, &mission)?;
        let departure_timeload_at_previous_stop = self
            .generic
            .transit_data
            .departure_time_of(trip, &previous_position);
        let new_criteria = Criteria {
            arrival_time: departure_timeload_at_previous_stop.0,
            nb_of_legs: waiting_criteria.nb_of_legs + 1,
            fallback_duration: waiting_criteria.fallback_duration,
            transfers_duration: waiting_criteria.transfers_duration,
        };
        Some(new_criteria)
    }

    fn best_trip_to_board(
        &self,
        position: &Data::Position,
        mission: &Data::Mission,
        waiting_criteria: &Criteria,
    ) -> Option<(Data::Trip, Criteria)> {
        let waiting_time = &waiting_criteria.arrival_time;
        self.generic
            .transit_data
            .latest_trip_that_debark_at(waiting_time, mission, position)
            .map(|(trip, arrival_time, _arrival_load)| {
                let new_criteria = Criteria {
                    arrival_time,
                    nb_of_legs: waiting_criteria.nb_of_legs + 1,
                    fallback_duration: waiting_criteria.fallback_duration,
                    transfers_duration: waiting_criteria.transfers_duration,
                };
                (trip, new_criteria)
            })
    }

    fn debark(
        &self,
        trip: &Data::Trip,
        position: &Data::Position,
        onboard_criteria: &Criteria,
    ) -> Option<Criteria> {
        debug_assert!({
            let arrival_time = &onboard_criteria.arrival_time;
            self.generic
                .transit_data
                .departure_time_of(trip, position)
                .0
                == *arrival_time
        });
        self.generic
            .transit_data
            .board_time_of(trip, position)
            .map(|board_timeload| Criteria {
                arrival_time: board_timeload.0,
                nb_of_legs: onboard_criteria.nb_of_legs,
                fallback_duration: onboard_criteria.fallback_duration,
                transfers_duration: onboard_criteria.transfers_duration,
            })
    }

    fn ride(&self, trip: &Data::Trip, position: &Data::Position, criteria: &Criteria) -> Criteria {
        let mission = self.generic.transit_data.mission_of(trip);
        let previous_position = self
            .generic
            .transit_data
            .previous_on_mission(position, &mission)
            .unwrap();
        let departure_timeload_at_previous_position = self
            .generic
            .transit_data
            .departure_time_of(trip, &previous_position);
        Criteria {
            arrival_time: departure_timeload_at_previous_position.0,
            nb_of_legs: criteria.nb_of_legs,
            fallback_duration: criteria.fallback_duration,
            transfers_duration: criteria.transfers_duration,
        }
    }

    fn transfer(
        &self,
        _from_stop: &Data::Stop,
        transfer: &Data::Transfer,
        criteria: &Criteria,
    ) -> (Data::Stop, Criteria) {
        let (arrival_stop, transfer_duration) = self.generic.transit_data.transfer(transfer);
        let new_criteria = Criteria {
            arrival_time: criteria.arrival_time - transfer_duration,
            nb_of_legs: criteria.nb_of_legs,
            fallback_duration: criteria.fallback_duration,
            transfers_duration: criteria.transfers_duration + transfer_duration,
        };
        (arrival_stop, new_criteria)
    }

    fn depart(&self, departure: &Departure) -> (Data::Stop, Criteria) {
        let (stop, fallback_duration) =
            &self.generic.arrivals_stop_point_and_fallbrack_duration[departure.idx];
        let arrival_time = self.generic.departure_datetime - *fallback_duration;
        let criteria = Criteria {
            arrival_time,
            nb_of_legs: 0,
            fallback_duration: *fallback_duration,
            transfers_duration: PositiveDuration::zero(),
        };
        (stop.clone(), criteria)
    }

    fn arrival_stop(&self, arrival: &Arrival) -> Data::Stop {
        self.generic.departures_stop_point_and_fallback_duration[arrival.idx]
            .0
            .clone()
    }

    fn arrive(&self, arrival: &Arrival, criteria: &Criteria) -> Criteria {
        let arrival_duration =
            &self.generic.departures_stop_point_and_fallback_duration[arrival.idx].1;
        Criteria {
            arrival_time: criteria.arrival_time - *arrival_duration,
            nb_of_legs: criteria.nb_of_legs,
            fallback_duration: criteria.fallback_duration + *arrival_duration,
            transfers_duration: criteria.transfers_duration,
        }
    }

    fn is_upstream(
        &self,
        upstream: &Data::Position,
        downstream: &Data::Position,
        mission: &Data::Mission,
    ) -> bool {
        !self
            .generic
            .transit_data
            .is_upstream(upstream, downstream, mission)
    }

    fn next_on_mission(
        &self,
        stop: &Data::Position,
        mission: &Data::Mission,
    ) -> Option<Data::Position> {
        self.generic.transit_data.previous_on_mission(stop, mission)
    }

    fn mission_of(&self, trip: &Data::Trip) -> Data::Mission {
        self.generic.transit_data.mission_of(trip)
    }

    fn stop_of(&self, position: &Data::Position, mission: &Data::Mission) -> Data::Stop {
        self.generic.transit_data.stop_of(position, mission)
    }

    fn nb_of_stops(&self) -> usize {
        self.generic.transit_data.nb_of_stops()
    }

    fn stop_id(&self, stop: &Data::Stop) -> usize {
        self.generic.transit_data.stop_id(stop)
    }

    fn nb_of_missions(&self) -> usize {
        self.generic.transit_data.nb_of_missions()
    }

    fn mission_id(&self, mission: &Data::Mission) -> usize {
        self.generic.transit_data.mission_id(mission)
    }

    fn new(
        model: &'model transit_model::Model,
        transit_data: &'data Data,
        request_input: &RequestInput,
    ) -> Result<Self, BadRequest>
    where
        Self: Sized,
    {
        let generic_result = GenericRequest::new(model, transit_data, request_input);
        generic_result.map(|generic| Self { generic })
    }

    pub fn data(&self) -> &Data {
        &self.generic.transit_data
    }

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
        self.generic.create_response(pt_journey, loads_count)
    }

    pub fn stop_name(&self, stop: &Data::Stop) -> String {
        self.generic.stop_name(stop)
    }
    pub fn trip_name(&self, trip: &Data::Trip) -> String {
        self.generic.trip_name(trip)
    }
    pub fn mission_name(&self, mission: &Data::Mission) -> String {
        self.generic.mission_name(mission)
    }
    pub fn position_name(&self, position: &Data::Position, mission: &Data::Mission) -> String {
        self.generic.position_name(position, mission)
    }
}

impl<'data, 'model, 'outer, Data> GenericBasicArrivalBefore<'data, 'model, Data>
where
    Data: DataTrait + DataIters<'outer>,
{
    fn departures(&'outer self) -> Departures {
        Departures {
            inner: self.generic.arrivals().inner,
        }
    }

    fn arrivals(&'outer self) -> Arrivals {
        Arrivals {
            inner: self.generic.departures().inner,
        }
    }

    fn boardable_missions_at(&'outer self, stop: &Data::Stop) -> Data::MissionsAtStop {
        self.generic.transit_data.boardable_missions_at(stop)
    }

    fn transfers_at(&'outer self, from_stop: &Data::Stop) -> Data::TransfersAtStop {
        self.generic.transit_data.transfers_at(from_stop)
    }

    fn trips_of(&'outer self, mission: &Data::Mission) -> Data::TripsOfMission {
        self.generic.transit_data.trips_of(mission)
    }
}
