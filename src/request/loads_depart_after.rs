use std::cmp::Ordering;

use crate::traits;
use crate::{
    loads_data::Load,
    time::{PositiveDuration, SecondsSinceDatasetUTCStart},
};

use chrono::NaiveDateTime;
use traits::{BadRequest, RequestIO};

use super::generic_request::{Arrivals, Departures, GenericRequest};

pub struct LoadsDepartAfter<'data, Data: traits::Data> {
    generic: GenericRequest<'data, Data>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct LoadsCount {
    high: usize,
    medium: usize,
    low: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Criteria {
    arrival_time: SecondsSinceDatasetUTCStart,
    nb_of_legs: u8,
    fallback_duration: PositiveDuration,
    transfers_duration: PositiveDuration,
    loads_count: LoadsCount,
}

impl LoadsCount {
    fn zero() -> Self {
        Self {
            high: 0,
            medium: 0,
            low: 0,
        }
    }

    fn add(&self, load: Load) -> Self {
        let mut high = self.high;
        let mut medium = self.medium;
        let mut low = self.low;
        match load {
            Load::High => {
                high += 1;
            }
            Load::Medium => {
                medium += 1;
            }
            Load::Low => {
                low += 1;
            }
        }
        Self { high, medium, low }
    }

    fn is_lower(&self, other: &Self) -> bool {
        use Ordering::{Equal, Greater, Less};
        match self.high.cmp(&other.high) {
            Less => true,
            Greater => false,
            Equal => match self.medium.cmp(&other.medium) {
                Less => true,
                Greater => false,
                Equal => match self.low.cmp(&other.low) {
                    Less | Equal => true,
                    Greater => false,
                },
            },
        }
    }
}

impl<'data, Data: traits::Data> traits::TransitTypes for LoadsDepartAfter<'data, Data> {
    type Stop = Data::Stop;
    type Mission = Data::Mission;
    type Trip = Data::Trip;
    type Transfer = Data::Transfer;
    type Position = Data::Position;
}

impl<'data, Data: traits::Data> traits::RequestTypes for LoadsDepartAfter<'data, Data> {
    type Departure = Departure;
    type Arrival = Arrival;
    type Criteria = Criteria;
}

impl<'data, 'model, Data: traits::Data> traits::Request for LoadsDepartAfter<'data, Data> {
    fn is_lower(&self, lower: &Self::Criteria, upper: &Self::Criteria) -> bool {
        let arrival_penalty = self.generic.leg_arrival_penalty;
        let walking_penalty = self.generic.leg_walking_penalty;
        lower.arrival_time + arrival_penalty * (lower.nb_of_legs as u32) 
            <= upper.arrival_time + arrival_penalty * (upper.nb_of_legs as u32)
        // && lower.nb_of_transfers <= upper.nb_of_transfers
        && 
        lower.fallback_duration + lower.transfers_duration  + walking_penalty * (lower.nb_of_legs as u32) 
            <=  upper.fallback_duration + upper.transfers_duration + walking_penalty * (upper.nb_of_legs as u32)
        && lower.loads_count.is_lower(&upper.loads_count)
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
        criteria.arrival_time <= self.generic.max_arrival_time
            && criteria.nb_of_legs <= self.generic.max_nb_legs
    }

    fn board_and_ride(
        &self,
        position: &Self::Position,
        trip: &Self::Trip,
        waiting_criteria: &Self::Criteria,
    ) -> Option<Self::Criteria> {
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

    fn best_trip_to_board(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
        waiting_criteria: &Self::Criteria,
    ) -> Option<(Self::Trip, Self::Criteria)> {
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

    fn debark(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
        onboard_criteria: &Self::Criteria,
    ) -> Option<Self::Criteria> {
        debug_assert!({
            let arrival_time = &onboard_criteria.arrival_time;
            self.generic.transit_data.arrival_time_of(trip, position).0 == *arrival_time
        });
        self.generic
            .transit_data
            .debark_time_of(trip, position)
            .map(|(debark_time, load)| Criteria {
                arrival_time: debark_time,
                nb_of_legs: onboard_criteria.nb_of_legs,
                fallback_duration: onboard_criteria.fallback_duration,
                transfers_duration: onboard_criteria.transfers_duration,
                loads_count: onboard_criteria.loads_count.add(load),
            })
    }

    fn ride(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
        criteria: &Self::Criteria,
    ) -> Self::Criteria {
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
        _from_stop: &Self::Stop,
        transfer: &Self::Transfer,
        criteria: &Self::Criteria,
    ) -> (Self::Stop, Self::Criteria) {
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

    fn depart(&self, departure: &Self::Departure) -> (Self::Stop, Self::Criteria) {
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

    fn arrival_stop(&self, arrival: &Self::Arrival) -> Self::Stop {
        self.generic.arrivals_stop_point_and_fallbrack_duration[arrival.idx]
            .0
            .clone()
    }

    fn arrive(&self, arrival: &Self::Arrival, criteria: &Self::Criteria) -> Self::Criteria {
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
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool {
        self.generic
            .transit_data
            .is_upstream(upstream, downstream, mission)
    }

    fn next_on_mission(
        &self,
        stop: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.generic.transit_data.next_on_mission(stop, mission)
    }

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission {
        self.generic.transit_data.mission_of(trip)
    }

    fn stop_of(&self, position: &Self::Position, mission: &Self::Mission) -> Self::Stop {
        self.generic.transit_data.stop_of(position, mission)
    }

    fn nb_of_stops(&self) -> usize {
        self.generic.transit_data.nb_of_stops()
    }

    fn stop_id(&self, stop: &Self::Stop) -> usize {
        self.generic.transit_data.stop_id(stop)
    }

    fn nb_of_missions(&self) -> usize {
        self.generic.transit_data.nb_of_missions()
    }

    fn mission_id(&self, mission: &Self::Mission) -> usize {
        self.generic.transit_data.mission_id(mission)
    }
}

impl<'data, 'outer, Data> traits::RequestIters<'outer> for LoadsDepartAfter<'data, Data>
where
    Data: traits::Data + traits::DataIters<'outer>,
{
    type Departures = Departures;
    fn departures(&'outer self) -> Self::Departures {
        self.generic.departures()
    }

    type Arrivals = Arrivals;
    fn arrivals(&'outer self) -> Self::Arrivals {
        self.generic.arrivals()
    }
}

impl<'data, 'outer, Data> traits::DataIters<'outer> for LoadsDepartAfter<'data, Data>
where
    Data: traits::Data + traits::DataIters<'outer>,
{
    type MissionsAtStop = Data::MissionsAtStop;

    fn boardable_missions_at(&'outer self, stop: &Self::Stop) -> Self::MissionsAtStop {
        self.generic.transit_data.boardable_missions_at(stop)
    }
    type TransfersAtStop = Data::TransfersAtStop;
    fn transfers_at(&'outer self, from_stop: &Self::Stop) -> Self::TransfersAtStop {
        self.generic.transit_data.transfers_at(from_stop)
    }

    type TripsOfMission = Data::TripsOfMission;
    fn trips_of(&'outer self, mission: &Self::Mission) -> Self::TripsOfMission {
        self.generic.transit_data.trips_of(mission)
    }
}

impl<'data, Data> traits::RequestWithIters for LoadsDepartAfter<'data, Data> where
    Data: traits::DataWithIters
{
}

use crate::response;
use crate::traits::Journey as PTJourney;

use super::generic_request::{Arrival, Departure};

impl<'data, Data> RequestIO<'data, Data> for LoadsDepartAfter<'data, Data>
where
    Data: traits::Data,
{
    fn new<'a, 'b>(
        model: &transit_model::Model,
        transit_data: &'data Data,
        departure_datetime: NaiveDateTime,
        departures_stop_point_and_fallback_duration: impl Iterator<Item = (&'a str, PositiveDuration)>,
        arrivals_stop_point_and_fallback_duration: impl Iterator<Item = (&'b str, PositiveDuration)>,
        leg_arrival_penalty: PositiveDuration,
        leg_walking_penalty: PositiveDuration,
        max_duration_to_arrival: PositiveDuration,
        max_nb_legs: u8,
    ) -> Result<Self, BadRequest> {
        let generic_result = GenericRequest::new(
            model,
            transit_data,
            departure_datetime,
            departures_stop_point_and_fallback_duration,
            arrivals_stop_point_and_fallback_duration,
            leg_arrival_penalty,
            leg_walking_penalty,
            max_duration_to_arrival,
            max_nb_legs,
        );
        generic_result.map(|generic| Self { generic })
    }
    fn create_response(
        &self,
        data: &Data,
        pt_journey: &PTJourney<Self>,
    ) -> Result<response::Journey<Data>, response::BadJourney<Data>> {
        self.generic.create_response(data, pt_journey)
    }
}
