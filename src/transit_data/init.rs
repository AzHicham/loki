
use transit_model;
use transit_model::{
    model::{Model},
    objects::{StopPoint, VehicleJourney, Transfer as TransitModelTransfer, StopTime},
}; 
use std::collections::{BTreeMap};
use typed_index_collection::{Idx};
use super::data::{ 
    EngineData, 
    Stop,  
    StopPattern, 
    StopPoints, 
    VehicleData, 
    StopData, 
    TransitModelTime,
    TransitData,

};
use super::ordered_timetable::{
    StopPatternTimetables,
    VehicleTimesError
};
use super::calendars::Calendars;
use super::time::{SecondsSinceDayStart, PositiveDuration};

use log::warn;


impl TransitData {
    pub fn new(transit_model : Model, default_transfer_duration : PositiveDuration) -> Self {
        let engine_data = EngineData::new(&transit_model, default_transfer_duration);
        Self {
            engine_data,
            transit_model,
        }
    }
}


impl EngineData {
    fn new(transit_model : & Model, default_transfer_duration : PositiveDuration) -> Self {

        let nb_of_stop_points = transit_model.stop_points.len();

        let (start_date, end_date) = transit_model.calculate_validity_period().expect("Unable to calculate a validity period.");

        let mut engine_data = Self {
            arrival_stop_points_to_forward_pattern : BTreeMap::new(),
            stop_point_idx_to_stop : std::collections::HashMap::new(),
            stops_data : Vec::with_capacity(nb_of_stop_points),
            forward_patterns : Vec::new(),
            calendars : Calendars::new(start_date, end_date),
        };

        engine_data.init(transit_model, default_transfer_duration);
 
        engine_data
    }

    fn init(&mut self, transit_model : & Model, default_transfer_duration : PositiveDuration) {
       for (vehicle_journey_idx, vehicle_journey) in transit_model.vehicle_journeys.iter() {
            self.insert_vehicle_journey(vehicle_journey_idx, vehicle_journey, transit_model);
        }

        for (transfer_idx, transfer) in transit_model.transfers.iter() {
            let has_from_stop_point_idx = transit_model.stop_points.get_idx(&transfer.from_stop_id);
            let has_to_stop_point_idx = transit_model.stop_points.get_idx(&transfer.to_stop_id);
            match (has_from_stop_point_idx, has_to_stop_point_idx) {
                (Some(from_stop_point_idx), Some(to_stop_point_idx)) => {
                    let duration = transfer.real_min_transfer_time.map_or(default_transfer_duration, |seconds| { PositiveDuration{seconds} });
                    self.insert_transfer(from_stop_point_idx, to_stop_point_idx, transfer_idx, duration)
                }
                _ => {
                    //TODO : log some error
                    continue;
                }
            }

        }


    }

    fn insert_transfer(& mut self, from_stop_point_idx : Idx<StopPoint>
                                , to_stop_point_idx : Idx<StopPoint>
                                , transfer_idx : Idx<TransitModelTransfer>
                                , duration : PositiveDuration ) 
    { 

        let has_from_stop = self.stop_point_idx_to_stop.get(&from_stop_point_idx);
        let has_to_stop = self.stop_point_idx_to_stop.get(&to_stop_point_idx);

        match (has_from_stop, has_to_stop) {
            (Some(from_stop), Some(to_stop)) => {
                let from_stop_data = & mut self.stops_data[from_stop.idx];
                from_stop_data.transfers.push((*to_stop, duration, Some(transfer_idx)));
            },
            _ => {
                warn!("Transfer {:?} is between stops which does not appears in the data.", transfer_idx);
            }
        }                          
    }
    

    fn insert_vehicle_journey(& mut self, vehicle_journey_idx : Idx<VehicleJourney>
                                        , vehicle_journey : & VehicleJourney
                                        , transit_model : & Model
                                    ) {
        
        let arrival_stop_times_iter = vehicle_journey.stop_times
                                            .iter()
                                            .filter(|stop_time| {
                                                stop_time.drop_off_type == 0
                                                // == 1 means the drop off is not permitter
                                                // TODO == 2 means an On Demand Transport 
                                                //         see what should be done here
                                            });
        let arrival_stop_points : StopPoints = arrival_stop_times_iter.clone()
                                                        .map(|stop_time| stop_time.stop_point_idx)
                                                        .collect();


        let has_forward_pattern = self.arrival_stop_points_to_forward_pattern.get(&arrival_stop_points);
        let forward_pattern = if let Some(pattern) = has_forward_pattern {
            *pattern
        }
        else {
            self.create_new_forward_pattern(arrival_stop_points)
        };
        let forward_pattern_data = & mut self.forward_patterns[forward_pattern.idx];

        let arrival_times_iter  = arrival_stop_times_iter.clone()
                                .map(|stop_time| {
                                    let arrival_time  = arrival_time(stop_time);
                                    SecondsSinceDayStart {
                                        seconds : arrival_time.total_seconds()
                                    }
                                });
        let departure_times_iter = arrival_stop_times_iter.clone()
                                    .map(|stop_time|
                                        if stop_time.pickup_type == 0 {
                                            let departure_time = departure_time(stop_time);
                                            let result = SecondsSinceDayStart {
                                                seconds : departure_time.total_seconds()
                                            };
                                            Some(result)
                                        }
                                        //  == 1 it means that boarding is not allowed
                                        //   at this stop_point
                                        // TODO : == 2 means an On Demand Transport 
                                        //        see what should be done here
                                        else {
                                            None
                                        }
                                        
                                    );


        let transit_model_calendar = transit_model.calendars
                                .get(&vehicle_journey.service_id)
                                .unwrap_or_else(|| 
                                    panic!(format!("Calendar {} needed for vehicle journey {} not found", 
                                                    vehicle_journey.service_id, 
                                                    vehicle_journey.id)
                                                )
                                            );

        let calendar_idx = self.calendars.get_or_insert(transit_model_calendar.dates.iter());
        
        let daily_trip_data = VehicleData{
            vehicle_journey_idx ,
            calendar_idx  
        };

        let insert_error = forward_pattern_data.insert(arrival_times_iter, departure_times_iter, daily_trip_data);
        if let Err(err) = insert_error {
            match err {
                VehicleTimesError::BoardBeforeDebark(idx) => {
                    let stop_time = &vehicle_journey.stop_times[idx];
                    let arrival_time = arrival_time(stop_time);
                    let departure_time = departure_time(stop_time);
                    warn!("Skipping vehicle journey {} in arrival pattern because at position {} its 
                            departure time {} is earlier than its arrival time {} ", 
                            vehicle_journey.id,
                            idx,
                            departure_time,
                            arrival_time
                        );
                },
                VehicleTimesError::NextDebarkIsBeforeBoard(idx) => {
                    let stop_time = &vehicle_journey.stop_times[idx];
                    let departure_time = departure_time(stop_time);
                    let next_stop_time = &vehicle_journey.stop_times[idx+1];
                    let next_arrival_time = arrival_time(next_stop_time);
                    warn!("Skipping vehicle journey {} in arrival pattern because its 
                            departure time {} at position {} is after its arrival time {} 
                            at the next position",
                            vehicle_journey.id,
                            departure_time,
                            idx,
                            next_arrival_time
                        );
                },
                VehicleTimesError::NextDebarkIsBeforePrevDebark(idx) => {
                    let stop_time = &vehicle_journey.stop_times[idx];
                    let arrival_time_ = arrival_time(stop_time);
                    let next_stop_time = &vehicle_journey.stop_times[idx+1];
                    let next_arrival_time = arrival_time(next_stop_time);
                    warn!("Skipping vehicle journey {} in arrival pattern because its 
                            arrival time {} at position {} is after its arrival time {} 
                            at the next position",
                            vehicle_journey.id,
                            arrival_time_,
                            idx,
                            next_arrival_time
                        );
                }
            }
        }

    }




    fn create_new_forward_pattern(& mut self, arrival_stop_points : StopPoints) -> StopPattern {
        debug_assert!( ! self.arrival_stop_points_to_forward_pattern.contains_key(&arrival_stop_points));

        let nb_of_positions = arrival_stop_points.len();
        let pattern = StopPattern {
            idx : self.forward_patterns.len()
        };


        let mut stops = Vec::with_capacity(nb_of_positions);
        for stop_point_idx in arrival_stop_points.iter() {
            let has_stop = self.stop_point_idx_to_stop.get(stop_point_idx);
            let stop = match has_stop {
                None => {
                    let new_stop = self.add_new_stop_point(*stop_point_idx);
                    new_stop
                },
                Some(&stop) => {
                    stop
                }
            };
            stops.push(stop);
        }


        let pattern_data = StopPatternTimetables::new(stops);



        self.arrival_stop_points_to_forward_pattern.insert(arrival_stop_points, pattern);

        for (stop, position) in pattern_data.stops_and_positions() {
            let stop_data = & mut self.stops_data[stop.idx]; 
            stop_data.position_in_forward_patterns.push((pattern, position));
        }

        self.forward_patterns.push(pattern_data);

        pattern
    }

    fn add_new_stop_point(&mut self, stop_point_idx : Idx<StopPoint>) -> Stop {
        debug_assert!(!self.stop_point_idx_to_stop.contains_key(&stop_point_idx));
        let stop_data = StopData{ 
            stop_point_idx,
            position_in_forward_patterns : Vec::new(),
            transfers : Vec::new() 
        };
        let stop = Stop {
            idx : self.stops_data.len()
        };
        self.stops_data.push(stop_data);
        self.stop_point_idx_to_stop.insert(stop_point_idx, stop);
        stop
    }

}

fn departure_time(stop_time : & StopTime) -> TransitModelTime {
    stop_time.departure_time - TransitModelTime::new(0,0, stop_time.boarding_duration.into())
}

fn arrival_time(stop_time : & StopTime) -> TransitModelTime {
    stop_time.arrival_time + TransitModelTime::new(0, 0, stop_time.alighting_duration.into())
}
