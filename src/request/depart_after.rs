
use crate::transit_data::{
    data::{
        TransitData,
        Stop,
        StopData,
        StopPattern,
        Transfer,
        Mission,
        Trip,
    },

    ordered_timetable::Position,
    iters::{
        TransfersOfStop,
        MissionsOfStop,
        TripsOfMission,
    },
    time::{
        SecondsSinceDatasetStart, 
        PositiveDuration,
        DaysSinceDatasetStart,
    }
};

use crate::engine::public_transit::{
    PublicTransit,
    PublicTransitIters,
    Journey as PTJourney,
};

use super::response:: {
    DepartureSection,
    VehicleSection,
    WaitingSection,
    TransferSection,
    ArrivalSection,
    Journey,
};

use typed_index_collection::{Idx};
use transit_model::{
    objects::{StopPoint,},
}; 

use std::cmp::Ordering;

use log::{error};

pub struct Request<'a> {
    transit_data : & 'a TransitData,
    departure_datetime : SecondsSinceDatasetStart,
    departures_stop_point_and_fallback_duration : Vec<(Stop, PositiveDuration)>,
    arrivals_stop_point_and_fallbrack_duration : Vec<(Stop, PositiveDuration)>,
    transfer_arrival_penalty : PositiveDuration,
    transfer_walking_penalty : PositiveDuration,
    max_arrival_time : SecondsSinceDatasetStart,
    max_nb_transfer : u8,
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DepartureIdx {
    idx : usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ArrivalIdx {
    idx : usize,
}





#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Criteria {
    arrival_time : SecondsSinceDatasetStart,
    nb_of_transfers : u8,
    fallback_duration : PositiveDuration,
    transfers_duration : PositiveDuration,
}


impl<'a> Request<'a> {

    pub fn new(transit_data : & 'a TransitData,
        departure_datetime : SecondsSinceDatasetStart,
        departures_stop_point_and_fallback_duration : Vec<(Stop, PositiveDuration)>,
        arrivals_stop_point_and_fallbrack_duration : Vec<(Stop, PositiveDuration)>,
        transfer_arrival_penalty : PositiveDuration,
        transfer_walking_penalty : PositiveDuration,
        max_arrival_time : SecondsSinceDatasetStart,
        max_nb_transfer : u8,
    ) -> Self
    {
        Self {
            transit_data,
            departure_datetime,
            departures_stop_point_and_fallback_duration,
            arrivals_stop_point_and_fallbrack_duration,
            transfer_arrival_penalty,
            transfer_walking_penalty,
            max_arrival_time,
            max_nb_transfer
        }
    }

    pub fn create_response_from_engine_result(&self, 
        pt_journey : & PTJourney<Self>
    ) ->  Result<Journey, ()>
    {
        let departure_section =  {
            let from_datetime = self.departure_datetime.clone();
            let departure_idx = pt_journey.departure_leg.departure.idx;
            let (stop, duration) = &self.departures_stop_point_and_fallback_duration[departure_idx];
            let to_datetime = self.departure_datetime.clone() + duration.clone();
            DepartureSection {
                from_datetime,
                to_datetime,
                to_stop : stop.clone()
            }
        };
        let first_vehicle = {
            let trip = &pt_journey.departure_leg.trip;
            let board_position = &pt_journey.departure_leg.board_position;
            let debark_position = &pt_journey.departure_leg.debark_position;
            let from_datetime = self.transit_data.board_time_of(trip, board_position).unwrap();
            let to_datetime = self.transit_data.debark_time_of(trip, debark_position).unwrap();
            let mission = self.transit_data.mission_of(trip);
            if self.transit_data.is_upstream_in_mission(debark_position, board_position, &mission) {
                return Err(());
            }
            let from_stop = self.transit_data.stop_at_position_in_mission(board_position, &mission);
            let to_stop = self.transit_data.stop_at_position_in_mission(debark_position, &mission);
            if from_stop != departure_section.to_stop {
                return Err(());
            }
            VehicleSection{
                from_datetime,
                to_datetime,
                from_stop,
                to_stop,
                trip : trip.clone()
            }
        };


        let first_waiting = match first_vehicle.from_datetime.cmp(&departure_section.to_datetime) {
            Ordering::Less => {
                return Err(());
            },
            Ordering::Equal => {
                None
            },
            Ordering::Greater => {
                let from_datetime = departure_section.to_datetime.clone();
                let to_datetime = first_vehicle.from_datetime.clone();
                let stop = departure_section.to_stop;
                let section = WaitingSection {
                    from_datetime,
                    to_datetime,
                    stop
                };
                Some(section)
            }
        };
        


        let mut prev_stop = first_vehicle.to_stop.clone();
        let mut prev_datetime = first_vehicle.to_datetime.clone();
        let mut connections = Vec::new();
        
        for connection_leg in pt_journey.connection_legs.iter() {
            let transfer_section = {
                let from_datetime = prev_datetime;
                let from_stop = prev_stop;
                let transfer = connection_leg.transfer.clone();
                let (to_stop, duration) = self.transit_data.transfer(&from_stop, &transfer);
                let to_datetime = from_datetime.clone() + duration;
                TransferSection {
                    from_datetime,
                    to_datetime,
                    from_stop,
                    to_stop,
                    transfer,
                }
            };

            let vehicle_section = {
                let trip = &connection_leg.trip;
                let board_position = &connection_leg.board_position;
                let debark_position = &connection_leg.debark_position;
                let from_datetime = self.transit_data.board_time_of(trip, board_position).unwrap();
                let to_datetime = self.transit_data.debark_time_of(trip, debark_position).unwrap();
                let mission = self.transit_data.mission_of(trip);
                if self.transit_data.is_upstream_in_mission(debark_position, board_position, &mission) {
                    return Err(());
                }
                let from_stop = self.transit_data.stop_at_position_in_mission(board_position, &mission);
                let to_stop = self.transit_data.stop_at_position_in_mission(debark_position, &mission);
                if from_stop != transfer_section.to_stop {
                    return Err(());
                }
                VehicleSection{
                    from_datetime,
                    to_datetime,
                    from_stop,
                    to_stop,
                    trip : trip.clone()
                }
            };

            let waiting_section = match vehicle_section.from_datetime.cmp(&transfer_section.to_datetime) {
                Ordering::Less => {
                    return Err(());
                },
                Ordering::Equal => {
                    None
                },
                Ordering::Greater => {
                    let from_datetime = transfer_section.to_datetime.clone();
                    let to_datetime = vehicle_section.from_datetime.clone();
                    let stop = transfer_section.to_stop.clone();
                    let section = WaitingSection {
                        from_datetime,
                        to_datetime,
                        stop
                    };
                    Some(section)
                }
            };


            prev_stop = vehicle_section.to_stop.clone();
            prev_datetime = vehicle_section.to_datetime.clone();

            connections.push((transfer_section, waiting_section, vehicle_section));


        }

        let arrival_section = {
            let from_datetime = prev_datetime.clone();
            let from_stop = prev_stop;
            let (arrival_stop, duration) = &self.arrivals_stop_point_and_fallbrack_duration[pt_journey.arrival.idx];
            if from_stop != *arrival_stop {
                return Err(());
            }
            let to_datetime = prev_datetime.clone() + *duration;
            ArrivalSection {
                from_datetime,
                to_datetime,
                from_stop,
            }
        };

        let journey = Journey {
            departure_section,
            first_waiting,
            first_vehicle,
            connections,
            arrival_section,
        };

        Ok(journey)

    }

}

impl<'a> PublicTransit for Request<'a> {
    type Stop = Stop;
    type Mission = Mission;
    type Trip = Trip;
    type Transfer = Transfer;
    type Departure = DepartureIdx;
    type Arrival = ArrivalIdx;
    type Criteria = Criteria;
    type Position = Position;

    fn is_upstream(&self, upstream : & Self::Position, downstream : & Self::Position, mission : & Self::Mission) -> bool {
        self.transit_data.is_upstream_in_mission(upstream, downstream, mission)
    }

    fn next_on_mission(&self, stop : & Self::Position, mission : & Self::Mission) -> Option<Self::Position> {
        self.transit_data.next_position_in_mission(stop, mission)
    }

    fn mission_of(&self, trip : & Self::Trip) -> Self::Mission {
        trip.mission.clone()
    }

    fn stop_of(&self, position: & Self::Position, mission: & Self::Mission) -> Self::Stop {
        self.transit_data.stop_at_position_in_mission(position, mission)
    }

    fn is_lower(&self, lower : & Self::Criteria, upper : & Self::Criteria) -> bool {
        // let two_hours = PositiveDuration{ seconds: 2*60*60};
        // if lower.arrival_time.clone() + two_hours < upper.arrival_time.clone() {
        //     return true;
        // }
        // lower.arrival_time <= upper.arrival_time
        lower.arrival_time.clone() + self.transfer_arrival_penalty * (lower.nb_of_transfers as u32) <= upper.arrival_time.clone() + self.transfer_arrival_penalty * (upper.nb_of_transfers as u32)
        // && lower.nb_of_transfers <= upper.nb_of_transfers
        && lower.fallback_duration + lower.transfers_duration  + self.transfer_walking_penalty * (lower.nb_of_transfers as u32) <=  upper.fallback_duration + upper.transfers_duration + self.transfer_walking_penalty * (upper.nb_of_transfers as u32)
        // && lower.arrival_time.clone() + lower.fallback_duration + lower.transfers_duration <= upper.arrival_time.clone() + upper.fallback_duration + upper.transfers_duration
        // && lower.fallback_duration + lower.transfers_duration <= upper.fallback_duration + upper.transfers_duration
    }

    // fn is_lower(&self, lower : & Self::Criteria, upper : & Self::Criteria) -> bool {
    //     lower.arrival_time <= upper.arrival_time
    //     && lower.nb_of_transfers <= upper.nb_of_transfers
    //     && lower.fallback_duration + lower.transfers_duration <=  upper.fallback_duration + upper.transfers_duration
    // }

    // fn is_lower(&self, lower : & Self::Criteria, upper : & Self::Criteria) -> bool {
    //     lower.arrival_time <= upper.arrival_time
    // }

    fn is_valid(&self, criteria: & Self::Criteria) -> bool {
        criteria.arrival_time <= self.max_arrival_time
        && criteria.nb_of_transfers <= self.max_nb_transfer
    }

    fn board_and_ride(&self, position : & Position, trip : & Self::Trip, waiting_criteria : & Self::Criteria) -> Option<Self::Criteria> {

        let has_board_time = self.transit_data.board_time_of(trip, position);
        if has_board_time.is_none() {
            return None;
        }
        if let Some(board_time) = has_board_time {
            if waiting_criteria.arrival_time > board_time {
                return None;
            }
        }
        let next_position = self.next_on_mission(position, &trip.mission).unwrap();
        let arrival_time_at_next_stop = self.transit_data.arrival_time_of(trip, &next_position);
        let new_criteria = Criteria {
            arrival_time : arrival_time_at_next_stop,
            nb_of_transfers : waiting_criteria.nb_of_transfers + 1,
            fallback_duration : waiting_criteria.fallback_duration,
            transfers_duration : waiting_criteria.transfers_duration
        };
        Some(new_criteria)
        
        
    }

    fn best_trip_to_board(&self, position : & Self::Position, mission : & Self::Mission, waiting_criteria : & Self::Criteria) -> Option<(Self::Trip, Self::Criteria)> {
        let waiting_time = &waiting_criteria.arrival_time;
        self.transit_data.earliest_trip_to_board_at(waiting_time, mission, position)
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

    fn debark(&self, trip : & Self::Trip, position : & Self::Position, onboard_criteria : & Self::Criteria) -> Option<Self::Criteria> {
        debug_assert!( {
            let arrival_time = & onboard_criteria.arrival_time;
            self.transit_data.arrival_time_of(trip, position) == *arrival_time
        });
        self.transit_data.debark_time_of(trip, position).map(|debark_time| {
            Criteria {
                arrival_time : debark_time,
                nb_of_transfers : onboard_criteria.nb_of_transfers,
                fallback_duration : onboard_criteria.fallback_duration,
                transfers_duration : onboard_criteria.transfers_duration
            }
        })   
    }

    fn ride(&self, trip : & Self::Trip, position : & Self::Position, criteria : & Self::Criteria) -> Self::Criteria {
        let next_position = self.next_on_mission(position, &trip.mission).unwrap();
        let arrival_time_at_next_position = self.transit_data.arrival_time_of(trip, &next_position);
        let new_criteria = Criteria {
            arrival_time : arrival_time_at_next_position,
            nb_of_transfers : criteria.nb_of_transfers,
            fallback_duration : criteria.fallback_duration,
            transfers_duration : criteria.transfers_duration
        };
        new_criteria

    }

    fn transfer(&self, from_stop : & Self::Stop, transfer : & Self::Transfer, criteria : & Self::Criteria) -> (Self::Stop, Self::Criteria) {
        let (arrival_stop, transfer_duration) = self.transit_data.transfer(from_stop, transfer);
        let new_criteria = Criteria {
            arrival_time : criteria.arrival_time.clone() + transfer_duration,
            nb_of_transfers : criteria.nb_of_transfers,
            fallback_duration : criteria.fallback_duration,
            transfers_duration : criteria.transfers_duration + transfer_duration
        };
        (arrival_stop, new_criteria)
    }

    fn depart(&self, departure : & Self::Departure) -> (Self::Stop, Self::Criteria) {
        let (stop, fallback_duration) = self.departures_stop_point_and_fallback_duration[departure.idx];
        let arrival_time = self.departure_datetime.clone() + fallback_duration;
        let criteria = Criteria{
            arrival_time,
            nb_of_transfers : 0,
            fallback_duration,
            transfers_duration : PositiveDuration::zero()
        };
        (stop, criteria)
    }


    fn arrival_stop(&self, arrival: & Self::Arrival) -> Self::Stop {
        (&self.arrivals_stop_point_and_fallbrack_duration[arrival.idx]).0.clone()
    }

    fn arrive(&self, arrival: & Self::Arrival, criteria: & Self::Criteria) -> Self::Criteria {
        let arrival_duration = &self.arrivals_stop_point_and_fallbrack_duration[arrival.idx].1;
        Criteria {
            arrival_time : criteria.arrival_time.clone() + *arrival_duration,
            nb_of_transfers : criteria.nb_of_transfers,
            fallback_duration : criteria.fallback_duration + *arrival_duration,
            transfers_duration : criteria.transfers_duration
        }
    }

 
    fn nb_of_stops(&self) -> usize {
        self.transit_data.nb_of_stops()
    }

    fn stop_id(&self, stop : & Self::Stop) -> usize {
        self.transit_data.stop_to_usize(stop)
    }



}

impl<'inner, 'outer> PublicTransitIters<'outer> for Request<'inner> {
    type MissionsAtStop = MissionsOfStop< 'outer >;

    fn boardable_missions_at(& 'outer self, stop : & Self::Stop) -> Self::MissionsAtStop {
        self.transit_data.missions_of(stop)
    }

    type Departures = Departures;
    fn departures(& 'outer self) -> Self::Departures {
        let nb_of_departures = self.departures_stop_point_and_fallback_duration.len();
        Departures {
            inner : 0..nb_of_departures
        }
    }

    type TransfersAtStop = TransfersOfStop;
    fn transfers_at(& 'outer self, from_stop : & Self::Stop) -> Self::TransfersAtStop {
        self.transit_data.transfers_of(from_stop)
    }

    type TripsOfMission = TripsOfMission;
    fn trips_of(&'outer self, mission : & Self::Mission) -> Self::TripsOfMission {
        self.transit_data.trips_of(mission)
    }

    type Arrivals = Arrivals;
    fn arrivals(&'outer self) -> Self::Arrivals {
        let nb_of_arrivals = self.arrivals_stop_point_and_fallbrack_duration.len();
        Arrivals{
            inner : 0..nb_of_arrivals
        }
    }
}

pub struct Departures {
    inner : std::ops::Range<usize>
}

impl Iterator for Departures {
    type Item = DepartureIdx;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|idx| {
            DepartureIdx{
                idx
            }
        })
    }
}

pub struct Arrivals {
    inner : std::ops::Range<usize>
}

impl Iterator for Arrivals {
    type Item = ArrivalIdx;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|idx| {
            ArrivalIdx{
                idx
            }
        })
    }
}

