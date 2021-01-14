

use crate::time::{PositiveDuration, SecondsSinceDatasetUTCStart};

use crate::traits::{ TransitTypes, TransitIters, NetworkStructure, Request as RequestTrait, RequestIters, Indices, Response, TimeQueries};

use super::Request;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DepartureIdx {
    pub (super) idx: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ArrivalIdx {
    pub (super) idx: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Criteria {
    arrival_time: SecondsSinceDatasetUTCStart,
    nb_of_legs: u8,
    fallback_duration: PositiveDuration,
    transfers_duration: PositiveDuration,
}


impl<'data, 'model, Data : TransitTypes> TransitTypes for Request<'data, 'model, Data> {
    type Stop = Data::Stop;
    type Mission = Data::Mission;
    type Trip = Data::Trip;
    type Transfer = Data::Transfer;
    type Position = Data::Position;

}

impl<'data, 'model, Data : TransitTypes + NetworkStructure> NetworkStructure for Request<'data, 'model, Data> {
    fn is_upstream(
        &self,
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool {
        self.transit_data
            .is_upstream(upstream, downstream, mission)
    }

    fn next_on_mission(
        &self,
        stop: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.transit_data.next_on_mission(stop, mission)
    }

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission {
        self.transit_data.mission_of(trip)
    }

    fn stop_of(&self, position: &Self::Position, mission: &Self::Mission) -> Self::Stop {
        self.transit_data
            .stop_of(position, mission)
    }
}
impl<'data, 'model, Data : TransitTypes > RequestTrait for Request<'data, 'model, Data> 
where Data : TimeQueries + NetworkStructure

{
    type Departure = DepartureIdx;
    type Arrival = ArrivalIdx;
    type Criteria = Criteria;


    

    fn is_lower(&self, lower: &Self::Criteria, upper: &Self::Criteria) -> bool {
        // let two_hours = PositiveDuration{ seconds: 2*60*60};
        // if lower.arrival_time.clone() + two_hours < upper.arrival_time.clone() {
        //     return true;
        // }
        // lower.arrival_time <= upper.arrival_time
        lower.arrival_time + self.leg_arrival_penalty * (lower.nb_of_legs as u32) 
            <= upper.arrival_time + self.leg_arrival_penalty * (upper.nb_of_legs as u32)
        // && lower.nb_of_transfers <= upper.nb_of_transfers
        && 
        lower.fallback_duration + lower.transfers_duration  + self.leg_walking_penalty * (lower.nb_of_legs as u32) 
            <=  upper.fallback_duration + upper.transfers_duration + self.leg_walking_penalty * (upper.nb_of_legs as u32) 

        // &&
        // lower.arrival_time.clone() + lower.fallback_duration + lower.transfers_duration 
        //      <= upper.arrival_time.clone() + upper.fallback_duration + upper.transfers_duration 
        

        // && lower.arrival_time.clone() + lower.fallback_duration + lower.transfers_duration <= upper.arrival_time.clone() + upper.fallback_duration + upper.transfers_duration
        // && lower.fallback_duration + lower.transfers_duration <= upper.fallback_duration + upper.transfers_duration
    }

    // fn is_lower(&self, lower : & Self::Criteria, upper : & Self::Criteria) -> bool {
    //     lower.arrival_time <= upper.arrival_time
    //     && lower.nb_of_legs <= upper.nb_of_legs
    //     && lower.fallback_duration + lower.transfers_duration <=  upper.fallback_duration + upper.transfers_duration
    // }

    // fn is_lower(&self, lower : & Self::Criteria, upper : & Self::Criteria) -> bool {
    //     lower.arrival_time <= upper.arrival_time
    // }

    fn is_valid(&self, criteria: &Self::Criteria) -> bool {
        criteria.arrival_time <= self.max_arrival_time && criteria.nb_of_legs <= self.max_nb_legs
    }

    fn board_and_ride(
        &self,
        position: &Self::Position,
        trip: &Self::Trip,
        waiting_criteria: &Self::Criteria,
    ) -> Option<Self::Criteria> {
        let has_board_time = self.transit_data.board_time_of(trip, position);
        if let Some(board_time) = has_board_time {
            if waiting_criteria.arrival_time > board_time {
                return None;
            }
        }
        else {
            return None;
        }
        let mission = self.transit_data.mission_of(trip);
        let next_position = self.transit_data.next_on_mission(position, &mission)?;
        let arrival_time_at_next_stop = self.transit_data.arrival_time_of(trip, &next_position);
        let new_criteria = Criteria {
            arrival_time: arrival_time_at_next_stop,
            nb_of_legs: waiting_criteria.nb_of_legs + 1,
            fallback_duration: waiting_criteria.fallback_duration,
            transfers_duration: waiting_criteria.transfers_duration,
        };
        Some(new_criteria)
    }

    fn best_trip_to_board(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
        waiting_criteria: &Self::Criteria,
    ) -> Option<(Self::Trip, Self::Criteria)> {
        let waiting_time = &waiting_criteria.arrival_time;
        self.transit_data
            .earliest_trip_to_board_at(waiting_time, mission, position)
            .map(|(trip, arrival_time)| {
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
        trip: &Self::Trip,
        position: &Self::Position,
        onboard_criteria: &Self::Criteria,
    ) -> Option<Self::Criteria> {
        debug_assert!({
            let arrival_time = &onboard_criteria.arrival_time;
            self.transit_data.arrival_time_of(trip, position) == *arrival_time
        });
        self.transit_data
            .debark_time_of(trip, position)
            .map(|debark_time| Criteria {
                arrival_time: debark_time,
                nb_of_legs: onboard_criteria.nb_of_legs,
                fallback_duration: onboard_criteria.fallback_duration,
                transfers_duration: onboard_criteria.transfers_duration,
            })
    }

    fn ride(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
        criteria: &Self::Criteria,
    ) -> Self::Criteria {
        let mission = self.transit_data.mission_of(trip);
        let next_position = self.transit_data.next_on_mission(position, &mission).unwrap();
        let arrival_time_at_next_position = self.transit_data.arrival_time_of(trip, &next_position);
        Criteria {
            arrival_time: arrival_time_at_next_position,
            nb_of_legs: criteria.nb_of_legs,
            fallback_duration: criteria.fallback_duration,
            transfers_duration: criteria.transfers_duration,
        }
    }

    fn transfer(
        &self,
        _from_stop: &Self::Stop,
        transfer: &Self::Transfer,
        criteria: &Self::Criteria,
    ) -> (Self::Stop, Self::Criteria) {
        let (arrival_stop, transfer_duration) = self.transit_data.transfer(transfer);
        let new_criteria = Criteria {
            arrival_time: criteria.arrival_time + transfer_duration,
            nb_of_legs: criteria.nb_of_legs,
            fallback_duration: criteria.fallback_duration,
            transfers_duration: criteria.transfers_duration + transfer_duration,
        };
        (arrival_stop, new_criteria)
    }

    fn depart(&self, departure: &Self::Departure) -> (Self::Stop, Self::Criteria) {
        let (stop, fallback_duration) =
            &self.departures_stop_point_and_fallback_duration[departure.idx];
        let arrival_time = self.departure_datetime + fallback_duration.clone();
        let criteria = Criteria {
            arrival_time,
            nb_of_legs: 0,
            fallback_duration : * fallback_duration,
            transfers_duration: PositiveDuration::zero(),
        };
        (stop.clone(), criteria)
    }

    fn arrival_stop(&self, arrival: &Self::Arrival) -> Self::Stop {
        (&self.arrivals_stop_point_and_fallbrack_duration[arrival.idx])
            .0.clone()
    }

    fn arrive(&self, arrival: &Self::Arrival, criteria: &Self::Criteria) -> Self::Criteria {
        let arrival_duration = &self.arrivals_stop_point_and_fallbrack_duration[arrival.idx].1;
        Criteria {
            arrival_time: criteria.arrival_time + *arrival_duration,
            nb_of_legs: criteria.nb_of_legs,
            fallback_duration: criteria.fallback_duration + *arrival_duration,
            transfers_duration: criteria.transfers_duration,
        }
    }


}

impl<'data, 'model, Data : TransitTypes> Indices for Request<'data, 'model, Data> 
where Data : Indices
{
    fn nb_of_stops(&self) -> usize {
        self.transit_data.nb_of_stops()
    }

    fn stop_id(&self, stop: &Self::Stop) -> usize {
        self.transit_data.stop_id(stop)
    }
}

impl<'data, 'outer, 'model, Data : TransitTypes>  RequestIters<'outer> for Request<'data, 'model, Data> 
where Data : TransitIters<'outer> + NetworkStructure + TimeQueries
{


    type Departures = Departures;
    fn departures(&'outer self) -> Self::Departures {
        let nb_of_departures = self.departures_stop_point_and_fallback_duration.len();
        Departures {
            inner: 0..nb_of_departures,
        }
    }

    type Arrivals = Arrivals;
    fn arrivals(&'outer self) -> Self::Arrivals {
        let nb_of_arrivals = self.arrivals_stop_point_and_fallbrack_duration.len();
        Arrivals {
            inner: 0..nb_of_arrivals,
        }
    }
}

impl<'data, 'outer, 'model, Data : TransitTypes> TransitIters<'outer> for Request<'data, 'model, Data>  
where Data : TransitIters<'outer>
{
    type MissionsAtStop = Data::MissionsAtStop;

    fn boardable_missions_at(&'outer self, stop: &Self::Stop) -> Self::MissionsAtStop {
        self.transit_data.boardable_missions_at(stop)
    }
    type TransfersAtStop = Data::TransfersAtStop;
    fn transfers_at(&'outer self, from_stop: &Self::Stop) -> Self::TransfersAtStop {
        self.transit_data.transfers_at(from_stop)
    }

    type TripsOfMission = Data::TripsOfMission;
    fn trips_of(&'outer self, mission: &Self::Mission) -> Self::TripsOfMission {
        self.transit_data.trips_of(mission)
    }


}

pub struct Departures {
    inner: std::ops::Range<usize>,
}

impl Iterator for Departures {
    type Item = DepartureIdx;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|idx| DepartureIdx { idx })
    }
}

pub struct Arrivals {
    inner: std::ops::Range<usize>,
}

impl Iterator for Arrivals {
    type Item = ArrivalIdx;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|idx| ArrivalIdx { idx })
    }
}

impl<'data, 'model, Data> TimeQueries for Request<'data, 'model, Data>
where Data : TimeQueries
{
    fn board_time_of(&self, trip : &Self::Trip, position  : & Self::Position) -> Option<SecondsSinceDatasetUTCStart> {
        self.transit_data.board_time_of(trip, position)
    }

    fn debark_time_of(&self, trip : &Self::Trip, position  : & Self::Position) -> Option<SecondsSinceDatasetUTCStart> {
        self.transit_data.debark_time_of(trip, position)
    }

    fn arrival_time_of(&self, trip : &Self::Trip, position  : & Self::Position) -> SecondsSinceDatasetUTCStart {
        self.transit_data.arrival_time_of(trip, position)
    }

    fn transfer(&self, transfer : & Self::Transfer) -> (Self::Stop, PositiveDuration) {
        self.transit_data.transfer(transfer)
    }

    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart)> {
        self.transit_data.earliest_trip_to_board_at(waiting_time, mission, position)
    }


}

impl<'data, 'model, Data> Response for Request<'data, 'model, Data> 
where Data : Response
{
    fn to_naive_datetime(&self, seconds : &SecondsSinceDatasetUTCStart) -> chrono::NaiveDateTime {
        self.transit_data.to_naive_datetime(seconds)
    }

    fn vehicle_journey_idx(&self, trip : & Self::Trip) -> typed_index_collection::Idx<transit_model::objects::VehicleJourney> {
        self.transit_data.vehicle_journey_idx(trip)
    }

    fn stop_point_idx(&self, stop : & Self::Stop) -> typed_index_collection::Idx<transit_model::objects::StopPoint> {
        self.transit_data.stop_point_idx(stop)
    }

    fn stoptime_idx(&self, position  : & Self::Position, trip : & Self::Trip) -> usize {
        self.transit_data.stoptime_idx(&position, &trip)
    }

    fn transfer_idx(&self, transfer : & Self::Transfer) -> typed_index_collection::Idx<transit_model::objects::Transfer> {
        self.transit_data.transfer_idx(transfer)
    }

    fn day_of(&self, trip : & Self::Trip) -> chrono::NaiveDate {
        self.transit_data.day_of(trip)
    }

    fn transfer_start_stop(&self, transfer : & Self::Transfer) -> Self::Stop {
        self.transit_data.transfer_start_stop(transfer)
    }

    fn is_same_stop(&self, stop_a : & Self::Stop, stop_b : & Self::Stop) -> bool {
        self.transit_data.is_same_stop(stop_a, stop_b)
    }
}