use crate::laxatips_data::{
    transit_data::{Mission, Stop, Transfer, Trip},
    iters::{MissionsOfStop, TransfersOfStop, TripsOfMission},
    timetables::timetables_data::Position,
};

use crate::time::{PositiveDuration, SecondsSinceDatasetUTCStart};



use crate::public_transit::{ PublicTransit, PublicTransitIters};

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


impl<'data> PublicTransit for Request<'data> {
    type Stop = Stop;
    type Mission = Mission;
    type Trip = Trip;
    type Transfer = Transfer;
    type Departure = DepartureIdx;
    type Arrival = ArrivalIdx;
    type Criteria = Criteria;
    type Position = Position;

    fn is_upstream(
        &self,
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool {
        self.laxatips_data.transit_data
            .is_upstream_in_mission(upstream, downstream, mission)
    }

    fn next_on_mission(
        &self,
        stop: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.laxatips_data.transit_data.next_position_in_mission(stop, mission)
    }

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission {
        Mission{ timetable : trip.vehicle.timetable.clone() }
    }

    fn stop_of(&self, position: &Self::Position, mission: &Self::Mission) -> Self::Stop {
        self.laxatips_data.transit_data
            .stop_at_position_in_mission(position, mission)
    }

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
        position: &Position,
        trip: &Self::Trip,
        waiting_criteria: &Self::Criteria,
    ) -> Option<Self::Criteria> {
        let has_board_time = self.laxatips_data.transit_data.board_time_of(trip, position);
        if let Some(board_time) = has_board_time {
            if waiting_criteria.arrival_time > board_time {
                return None;
            }
        }
        else {
            return None;
        }
        let next_position = self.laxatips_data.transit_data.next_position(position).unwrap();
        let arrival_time_at_next_stop = self.laxatips_data.transit_data.arrival_time_of(trip, &next_position);
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
        self.laxatips_data.transit_data
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
            self.laxatips_data.transit_data.arrival_time_of(trip, position) == *arrival_time
        });
        self.laxatips_data.transit_data
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
        let next_position = self.laxatips_data.transit_data.next_position(position).unwrap();
        let arrival_time_at_next_position = self.laxatips_data.transit_data.arrival_time_of(trip, &next_position);
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
        let (arrival_stop, transfer_duration) = self.laxatips_data.transit_data.transfer(transfer);
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
            self.departures_stop_point_and_fallback_duration[departure.idx];
        let arrival_time = self.departure_datetime + fallback_duration;
        let criteria = Criteria {
            arrival_time,
            nb_of_legs: 0,
            fallback_duration,
            transfers_duration: PositiveDuration::zero(),
        };
        (stop, criteria)
    }

    fn arrival_stop(&self, arrival: &Self::Arrival) -> Self::Stop {
        (self.arrivals_stop_point_and_fallbrack_duration[arrival.idx])
            .0
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

    fn nb_of_stops(&self) -> usize {
        self.laxatips_data.transit_data.nb_of_stops()
    }

    fn stop_id(&self, stop: &Self::Stop) -> usize {
        self.laxatips_data.transit_data.stop_to_usize(stop)
    }
}

impl<'data,  'outer> PublicTransitIters<'outer> for Request<'data> {
    type MissionsAtStop = MissionsOfStop<'outer>;

    fn boardable_missions_at(&'outer self, stop: &Self::Stop) -> Self::MissionsAtStop {
        self.laxatips_data.transit_data.missions_of(stop)
    }

    type Departures = Departures;
    fn departures(&'outer self) -> Self::Departures {
        let nb_of_departures = self.departures_stop_point_and_fallback_duration.len();
        Departures {
            inner: 0..nb_of_departures,
        }
    }

    type TransfersAtStop = TransfersOfStop;
    fn transfers_at(&'outer self, from_stop: &Self::Stop) -> Self::TransfersAtStop {
        self.laxatips_data.transit_data.transfers_of(from_stop)
    }

    type TripsOfMission = TripsOfMission;
    fn trips_of(&'outer self, mission: &Self::Mission) -> Self::TripsOfMission {
        self.laxatips_data.transit_data.trips_of(mission)
    }

    type Arrivals = Arrivals;
    fn arrivals(&'outer self) -> Self::Arrivals {
        let nb_of_arrivals = self.arrivals_stop_point_and_fallbrack_duration.len();
        Arrivals {
            inner: 0..nb_of_arrivals,
        }
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
