
use crate::transit_data::{
    data::{
        EngineData,
        TransitData,
        Stop,
        StopIdx,
        StopPatternIdx,
        Position,
        TransferIdx,
    },
    depart_after_queries::{
        ForwardMission,
        ForwardTrip,
        ForwardMissionsOfStop
        
    },
    time::{
        SecondsSinceDatasetStart, 
        PositiveDuration,
        DaysSinceDatasetStart,
    }
};

use crate::engine::public_transit::{
    PublicTransit,
    PublicTransitIters
};

use typed_index_collection::{Idx};
use transit_model::{
    objects::{StopPoint,},
}; 

pub struct Request<'a> {
    transit_data : & 'a TransitData,
    departure_datetime : SecondsSinceDatasetStart,
    departures_stop_point_and_fallback_duration : Vec<(StopIdx, PositiveDuration)>,
    arrivals_stop_point_and_fallbrack_duration : Vec<(StopIdx, PositiveDuration)>

}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DepartureIdx {
    idx : usize,
}




#[derive(Debug, PartialEq, Eq, Clone)]
struct Criteria {
    arrival_time : SecondsSinceDatasetStart,
    nb_of_transfers : usize,
    fallback_duration : PositiveDuration,
    transfers_duration : PositiveDuration,
}


impl<'a> Request<'a> {

    pub fn new(transit_data : & 'a TransitData,
        departure_datetime : SecondsSinceDatasetStart,
        departures_stop_point_and_fallback_duration : Vec<(StopIdx, PositiveDuration)>,
        arrivals_stop_point_and_fallbrack_duration : Vec<(StopIdx, PositiveDuration)>
    ) -> Self
    {
        Self {
            transit_data,
            departure_datetime,
            departures_stop_point_and_fallback_duration,
            arrivals_stop_point_and_fallbrack_duration,
        }
    }

}

impl<'a> PublicTransit for Request<'a> {
    type Stop = StopIdx;
    type Mission = ForwardMission;
    type Trip = ForwardTrip;
    type Transfer = TransferIdx;
    type Departure = DepartureIdx;
    type Criteria = Criteria;

    fn is_upstream(&self, upstream : & Self::Stop, downstream : & Self::Stop, mission : & Self::Mission) -> bool {
        self.transit_data.engine_data.is_upstream_in_forward_mission(upstream, downstream, mission)
    }

    fn next_on_mission(&self, stop : & Self::Stop, mission : & Self::Mission) -> Option<Self::Stop> {
        self.transit_data.engine_data.next_stop_in_forward_mission(stop, mission)
    }

    fn mission_of(&self, trip : & Self::Trip) -> Self::Mission {
        trip.mission
    }

    fn is_lower(&self, lower : & Self::Criteria, upper : & Self::Criteria) -> bool {
        lower.arrival_time <= upper.arrival_time
        && lower.nb_of_transfers <= upper.nb_of_transfers
        && lower.fallback_duration + lower.transfers_duration <= upper.fallback_duration + upper.transfers_duration
    }

    fn board_and_ride(&self, stop_idx : & StopIdx, trip : & Self::Trip, waiting_criteria : & Self::Criteria) -> Option<Self::Criteria> {

        let engine_data = self.transit_data.engine_data;
        let has_departure_time = engine_data.departure_time_of(trip, stop_idx);
        if has_departure_time.is_none() {
            return None;
        }
        if let Some(departure_time) = has_departure_time {
            if waiting_criteria.arrival_time > departure_time {
                return None;
            }
        }
        let next_stop = self.next_on_mission(stop_idx, &trip.mission).unwrap();
        let arrival_time_at_next_stop = engine_data.arrival_time_of(trip, &next_stop);
        let new_criteria = Criteria {
            arrival_time : arrival_time_at_next_stop,
            nb_of_transfers : waiting_criteria.nb_of_transfers + 1,
            fallback_duration : waiting_criteria.fallback_duration,
            transfers_duration : waiting_criteria.transfers_duration
        };
        Some(new_criteria)
        
        
    }

    fn best_trip_to_board(&self, stop_idx : & Self::Stop, mission : & Self::Mission, waiting_criteria : & Self::Criteria) -> Option<(Self::Trip, Self::Criteria)> {
        let engine_data = self.transit_data.engine_data;
        let waiting_time = &waiting_criteria.arrival_time;
        engine_data.best_trip_to_board_at_stop(waiting_time, mission, stop_idx)
            .map(|(trip, arrival_time)| {
                let new_criteria =  Criteria {
                    arrival_time,
                    nb_of_transfers : waiting_criteria.nb_of_transfers + 1,
                    fallback_duration : waiting_criteria.fallback_duration,
                    transfers_duration : waiting_criteria.transfers_duration,
                };
                (trip, new_criteria)
            })
    }

    fn debark(&self, trip : & Self::Trip, stop : & Self::Stop, onboard_criteria : & Self::Criteria) -> Self::Criteria {
        debug_assert!( {
            let arrival_time = onboard_criteria.arrival_time;
            let engine_data = self.transit_data.engine_data;
            engine_data.arrival_time_of(trip, stop) == arrival_time
        });
        onboard_criteria.clone()    
    }

    fn ride(&self, trip : & Self::Trip, stop_idx : & Self::Stop, criteria : & Self::Criteria) -> Self::Criteria {
        let engine_data = & self.transit_data.engine_data;
        let next_stop = self.next_on_mission(stop_idx, &trip.mission).unwrap();
        let arrival_time_at_next_stop = engine_data.arrival_time_of(trip, &next_stop);
        let new_criteria = Criteria {
            arrival_time : arrival_time_at_next_stop,
            nb_of_transfers : criteria.nb_of_transfers,
            fallback_duration : criteria.fallback_duration,
            transfers_duration : criteria.transfers_duration
        };
        new_criteria

    }

    fn transfer(&self, from_stop : & Self::Stop, transfer : & Self::Transfer, criteria : & Self::Criteria) -> (Self::Stop, Self::Criteria) {
        let engine_data = & self.transit_data.engine_data;
        let (arrival_stop, transfer_duration) = engine_data.transfer(from_stop, transfer);
        let new_criteria = Criteria {
            arrival_time : criteria.arrival_time + transfer_duration,
            nb_of_transfers : criteria.nb_of_transfers,
            fallback_duration : criteria.fallback_duration,
            transfers_duration : criteria.transfers_duration + transfer_duration
        };
        (arrival_stop, new_criteria)
    }

    fn depart(&self, departure : & Self::Departure) -> (Self::Stop, Self::Criteria) {
        let (stop, fallback_duration) = self.departures_stop_point_and_fallback_duration[departure.idx];
        let arrival_time = self.departure_datetime + fallback_duration;
        let criteria = Criteria{
            arrival_time,
            nb_of_transfers : 0,
            fallback_duration,
            transfers_duration : PositiveDuration::zero()
        };
        (stop, criteria)
    }

    fn journey_arrival(&self, stop : & Self::Stop, criteria : & Self::Criteria) -> Option<Self::Criteria> {
        self.arrivals_stop_point_and_fallbrack_duration
            .iter()
            .find_map(|(arrival_stop, duration)| {
                if stop == arrival_stop {
                    Some(duration)
                }
                else {
                    None
                }
            })
            .map(|duration| {
                Criteria {
                    arrival_time : criteria.arrival_time,
                    nb_of_transfers : criteria.nb_of_transfers,
                    fallback_duration : criteria.fallback_duration + *duration,
                    transfers_duration : criteria.transfers_duration
                }
            })

    }

    fn nb_of_stops(&self) -> usize {
        self.transit_data.engine_data.nb_of_stops()
    }

    fn stop_id(&self, stop : & Self::Stop) -> usize {
        self.transit_data.engine_data.stop_idx_to_usize(stop)
    }



}

impl<'inner, 'outer> PublicTransitIters<'outer> for Request<'inner> {
    type MissionsAtStop = ForwardMissionsOfStop< 'outer >;
}