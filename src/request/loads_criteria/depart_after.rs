use chrono::NaiveDateTime;

use crate::{PositiveDuration, loads_data::LoadsCount, response, traits::{self, BadRequest, RequestTypes}};
use crate::traits::Journey as PTJourney;

use super::super::generic_request::{GenericRequest, Arrival, Arrivals, Departure, Departures};
use super::Criteria;

pub mod loads_comparator;
pub mod classic_comparator;

pub struct GenericLoadsDepartAfter<'data, Data: traits::Data> {
    generic: GenericRequest<'data, Data>,
}

impl<'data, 'model, Data: traits::Data>  GenericLoadsDepartAfter<'data, Data> {

    pub fn leg_arrival_penalty(&self) ->  PositiveDuration {
        self.generic.leg_arrival_penalty
    }

    pub fn leg_walking_penalty(&self) ->  PositiveDuration {
        self.generic.leg_walking_penalty
    }



    pub fn is_valid(&self, criteria: & Criteria) -> bool {
        criteria.arrival_time <= self.generic.max_arrival_time
            && criteria.nb_of_legs <= self.generic.max_nb_legs
    }

    fn board_and_ride(
        &self,
        position: &Data::Position,
        trip: &Data::Trip,
        waiting_criteria: & Criteria,
    ) -> Option<Criteria> {
        let has_board = self.generic.transit_data.board_time_of(trip, position);
        if let Some(board_timeload) = has_board {
            if waiting_criteria.arrival_time > board_timeload.0 {
                return None;
            }
        } else {
            return None;
        }
        let mission = self.generic.transit_data.mission_of(trip);
        let next_position = self
            .generic
            .transit_data
            .next_on_mission(position, &mission)?;
        let (arrival_time_at_next_stop, load) = self
            .generic
            .transit_data
            .arrival_time_of(trip, &next_position);
        let new_criteria = Criteria {
            arrival_time: arrival_time_at_next_stop,
            nb_of_legs: waiting_criteria.nb_of_legs + 1,
            fallback_duration: waiting_criteria.fallback_duration,
            transfers_duration: waiting_criteria.transfers_duration,
            loads_count: waiting_criteria.loads_count.add(load),
        };
        Some(new_criteria)
    }


    pub fn best_trip_to_board(
        &self,
        position: &Data::Position,
        mission: &Data::Mission,
        waiting_criteria: &Criteria,
    ) -> Option<(Data::Trip, Criteria)> {
        let waiting_time = &waiting_criteria.arrival_time;
        self.generic
            .transit_data
            .earliest_trip_to_board_at(waiting_time, mission, position)
            .map(|(trip, arrival_time, load)| {
                let new_criteria = Criteria {
                    arrival_time,
                    nb_of_legs: waiting_criteria.nb_of_legs + 1,
                    fallback_duration: waiting_criteria.fallback_duration,
                    transfers_duration: waiting_criteria.transfers_duration,
                    loads_count: waiting_criteria.loads_count.add(load),
                };
                (trip, new_criteria)
            })
    }


    pub fn debark(
        &self,
        trip: &Data::Trip,
        position: &Data::Position,
        onboard_criteria: &Criteria,
    ) -> Option<Criteria> {
        debug_assert!({
            let arrival_time = &onboard_criteria.arrival_time;
            self.generic.transit_data.arrival_time_of(trip, position).0 == *arrival_time
        });
        self.generic
            .transit_data
            .debark_time_of(trip, position)
            .map(|(debark_time, _load)| Criteria {
                arrival_time: debark_time,
                nb_of_legs: onboard_criteria.nb_of_legs,
                fallback_duration: onboard_criteria.fallback_duration,
                transfers_duration: onboard_criteria.transfers_duration,
                loads_count: onboard_criteria.loads_count.clone(),
            })
    }


    fn ride(
        &self,
        trip: &Data::Trip,
        position: &Data::Position,
        criteria: &Criteria,
    ) -> Criteria {
        let mission = self.generic.transit_data.mission_of(trip);
        let next_position = self
            .generic
            .transit_data
            .next_on_mission(position, &mission)
            .unwrap();
        let (arrival_time_at_next_position, load) = self
            .generic
            .transit_data
            .arrival_time_of(trip, &next_position);
        Criteria {
            arrival_time: arrival_time_at_next_position,
            nb_of_legs: criteria.nb_of_legs,
            fallback_duration: criteria.fallback_duration,
            transfers_duration: criteria.transfers_duration,
            loads_count: criteria.loads_count.add(load),
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
            arrival_time: criteria.arrival_time + transfer_duration,
            nb_of_legs: criteria.nb_of_legs,
            fallback_duration: criteria.fallback_duration,
            transfers_duration: criteria.transfers_duration + transfer_duration,
            loads_count: criteria.loads_count.clone(),
        };
        (arrival_stop, new_criteria)
    }

    fn depart(&self, departure: &Departure) -> (Data::Stop, Criteria) {
        let (stop, fallback_duration) =
            &self.generic.departures_stop_point_and_fallback_duration[departure.idx];
        let arrival_time = self.generic.departure_datetime + *fallback_duration;
        let criteria = Criteria {
            arrival_time,
            nb_of_legs: 0,
            fallback_duration: *fallback_duration,
            transfers_duration: PositiveDuration::zero(),
            loads_count: LoadsCount::zero(),
        };
        (stop.clone(), criteria)
    }

    fn arrival_stop(&self, arrival: &Arrival) -> Data::Stop {
        self.generic.arrivals_stop_point_and_fallbrack_duration[arrival.idx]
            .0
            .clone()
    }

    fn arrive(&self, arrival: &Arrival, criteria: &Criteria) -> Criteria {
        let arrival_duration =
            &self.generic.arrivals_stop_point_and_fallbrack_duration[arrival.idx].1;
        Criteria {
            arrival_time: criteria.arrival_time + *arrival_duration,
            nb_of_legs: criteria.nb_of_legs,
            fallback_duration: criteria.fallback_duration + *arrival_duration,
            transfers_duration: criteria.transfers_duration,
            loads_count: criteria.loads_count.clone(),
        }
    }

    fn is_upstream(
        &self,
        upstream: &Data::Position,
        downstream: &Data::Position,
        mission: &Data::Mission,
    ) -> bool {
        self.generic
            .transit_data
            .is_upstream(upstream, downstream, mission)
    }

    fn next_on_mission(
        &self,
        stop: &Data::Position,
        mission: &Data::Mission,
    ) -> Option<Data::Position> {
        self.generic.transit_data.next_on_mission(stop, mission)
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


    fn new<Departures, Arrivals, D, A>
    (
        model: &transit_model::Model,
        transit_data: & 'data Data,
        request_input : traits::RequestInput<Departures, Arrivals, D, A>
    ) -> Result<Self, BadRequest>
    where
        Arrivals : Iterator<Item = (A, PositiveDuration)>,
        Departures : Iterator<Item = (D, PositiveDuration)>,
        A : AsRef<str>,
        D : AsRef<str>,
        Self: Sized
    {
        let generic_result = GenericRequest::new(
            model,
            transit_data,
            request_input
        );
        generic_result.map(|generic| Self { generic })
    }

    pub fn data(&self) -> & Data {
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

}

impl<'data, 'outer, Data> GenericLoadsDepartAfter<'data, Data>
where
    Data: traits::Data + traits::DataIters<'outer>,
{

    fn departures(&'outer self) -> Departures {
        self.generic.departures()
    }

 
    fn arrivals(&'outer self) -> Arrivals {
        self.generic.arrivals()
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

