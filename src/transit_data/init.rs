
use transit_model;
use transit_model::{
    model::{Model},
    objects::{StopPoint, VehicleJourney, Transfer, StopTime},
}; 
use std::collections::{BTreeMap};
use typed_index_collection::{Idx};
use super::data::{ 
    EngineData, 
    StopIdx,  
    StopPatternIdx, 
    StopPointArray, 
    VehicleData, 
    Stop, 
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
            arrival_stop_point_array_to_stop_pattern : BTreeMap::new(),
            stop_point_idx_to_stops : BTreeMap::new(),
            stops : Vec::with_capacity(nb_of_stop_points),
            arrival_stop_patterns : Vec::new(),
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

        //create transfer between all stops representing the same stop_point
        for stops_idx in self.stop_point_idx_to_stops.values() {
            let from_stops_idx = stops_idx.clone();
            let to_stops_idx = stops_idx.clone();
            for from_stop_idx in from_stops_idx {
                let from_stop = & mut self.stops[from_stop_idx.idx];
                for to_stop_idx in to_stops_idx.clone() {
                    from_stop.transfers.push((to_stop_idx, default_transfer_duration, None));
                }
            }
        }

    }

    fn insert_transfer(& mut self, from_stop_point_idx : Idx<StopPoint>
                                , to_stop_point_idx : Idx<StopPoint>
                                , transfer_idx : Idx<Transfer>
                                , duration : PositiveDuration ) 
    { 

        let empty_vec : Vec<StopIdx> = Vec::new();

        let from_stops_idx = self.stop_point_idx_to_stops.get(&from_stop_point_idx).map_or_else(|| empty_vec.iter(), |vec| vec.iter());
        let to_stops_idx = self.stop_point_idx_to_stops.get(&to_stop_point_idx).map_or_else(|| empty_vec.iter(), |vec| vec.iter());

        for from_stop_idx in from_stops_idx {
            let from_stop = & mut self.stops[from_stop_idx.idx];
            for to_stop_idx in to_stops_idx.clone() {
                from_stop.transfers.push((*to_stop_idx, duration, Some(transfer_idx)));
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
        let arrival_stop_point_array : StopPointArray = arrival_stop_times_iter.clone()
                                                        .map(|stop_time| stop_time.stop_point_idx)
                                                        .collect();


        let has_arrival_pattern = self.arrival_stop_point_array_to_stop_pattern.get(&arrival_stop_point_array);
        let arrival_stop_pattern_idx = if let Some(stop_pattern_idx) = has_arrival_pattern {
            *stop_pattern_idx
        }
        else {
            self.create_new_arrival_stop_pattern(arrival_stop_point_array)
        };
        let arrival_stop_pattern = & mut self.arrival_stop_patterns[arrival_stop_pattern_idx.idx];

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

        let insert_error = arrival_stop_pattern.insert(arrival_times_iter, departure_times_iter, daily_trip_data);
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




    fn create_new_arrival_stop_pattern(& mut self, arrival_stop_point_array : StopPointArray) -> StopPatternIdx {
        debug_assert!( ! self.arrival_stop_point_array_to_stop_pattern.contains_key(&arrival_stop_point_array));

        let nb_of_positions = arrival_stop_point_array.len();
        let stop_pattern_idx = StopPatternIdx {
            idx : self.arrival_stop_patterns.len()
        };


        let mut stops = Vec::with_capacity(nb_of_positions);
        for stop_point_idx in arrival_stop_point_array.iter() {
            let has_stops = self.stop_point_idx_to_stops.get(stop_point_idx);
            let stop_idx = match has_stops {
                None => {
                    let new_stop_idx = self.add_new_stop_point(*stop_point_idx);
                    new_stop_idx
                },
                Some(stops_idx) => {
                    debug_assert!(stops_idx.len() >=1 );
                    let has_suitable_stop_idx = stops_idx.iter().find(|&&stop_idx| {
                        let stop = & self.stops[stop_idx.idx];
                        ! stop.arrival_patterns.contains(&stop_pattern_idx)
                    });
                    if let Some(&stop_idx) = has_suitable_stop_idx {
                        stop_idx
                    }
                    // all stops associated to this stop_point_idx
                    // are already been used in this stop_pattern
                    // hence we create a new copy
                    else {
                        let new_stop_idx = self.add_new_stop_point(*stop_point_idx);
                        new_stop_idx
                    }
                }
            };
            let stop = & mut self.stops[stop_idx.idx];
            stop.arrival_patterns.push(stop_pattern_idx);

            stops.push(stop_idx);
        }


        let stop_pattern = StopPatternTimetables::new(stops);

        self.arrival_stop_patterns.push(stop_pattern);

        self.arrival_stop_point_array_to_stop_pattern.insert(arrival_stop_point_array, stop_pattern_idx);

        stop_pattern_idx
    }

    fn add_new_stop_point(&mut self, stop_point_idx : Idx<StopPoint>) -> StopIdx {
        let stop = Stop{ 
            stop_point_idx,
            arrival_patterns : Vec::new(),
            transfers : Vec::new() 
        };
        let stop_idx = StopIdx {
            idx : self.stops.len()
        };
        self.stops.push(stop);
        let mut stops_idx = self.stop_point_idx_to_stops.entry(stop_point_idx).or_insert(Vec::new());
        stops_idx.push(stop_idx);
        stop_idx
    }

}

fn departure_time(stop_time : & StopTime) -> TransitModelTime {
    stop_time.departure_time - TransitModelTime::new(0,0, stop_time.boarding_duration.into())
}

fn arrival_time(stop_time : & StopTime) -> TransitModelTime {
    stop_time.arrival_time + TransitModelTime::new(0, 0, stop_time.alighting_duration.into())
}
