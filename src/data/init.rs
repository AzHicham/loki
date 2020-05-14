
use transit_model;
use transit_model::{
    model::{Model},
    objects::{StopPoint, VehicleJourney, Transfer, Time},
}; 
use std::collections::{BTreeMap};
use crate::data::chain_decomposition::ChainDecomposition;
use typed_index_collection::{Idx};
use crate::data::data::{ TransitData, Duration, StopIdx, StopPattern, StopPatternIdx, StopPointArray, DailyTripData, Position, Stop};

impl TransitData {
    pub fn new(transit_model : & Model, default_transfer_duration : Duration) -> TransitData {

        let nb_of_stop_points = transit_model.stop_points.len();

        let mut transit_data = TransitData {
            stop_point_array_to_stop_pattern : BTreeMap::new(),
            stop_point_idx_to_stops_idx : BTreeMap::new(),
            stops : Vec::with_capacity(nb_of_stop_points),
            stop_patterns : Vec::new(),
        };

        transit_data.init(transit_model, default_transfer_duration);
 
        transit_data
    }

    fn init(&mut self, transit_model : & Model, default_transfer_duration : Duration) {
       for (vehicle_journey_idx, vehicle_journey) in transit_model.vehicle_journeys.iter() {
            self.insert_vehicle_journey(vehicle_journey_idx, vehicle_journey);
        }

        for (transfer_idx, transfer) in transit_model.transfers.iter() {
            let has_from_stop_point_idx = transit_model.stop_points.get_idx(&transfer.from_stop_id);
            let has_to_stop_point_idx = transit_model.stop_points.get_idx(&transfer.to_stop_id);
            match (has_from_stop_point_idx, has_to_stop_point_idx) {
                (Some(from_stop_point_idx), Some(to_stop_point_idx)) => {
                    let duration = transfer.real_min_transfer_time.map_or(default_transfer_duration, |seconds| { Duration{seconds} });
                    self.insert_transfer(from_stop_point_idx, to_stop_point_idx, transfer_idx, duration)
                }
                _ => {
                    //TODO : log some error
                    continue;
                }
            }

        }

        //create transfer between all stops representing the same stop_point
        for stops_idx in self.stop_point_idx_to_stops_idx.values() {
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
                                , duration : Duration ) 
    { 

        let empty_vec : Vec<StopIdx> = Vec::new();

        let from_stops_idx = self.stop_point_idx_to_stops_idx.get(&from_stop_point_idx).map_or_else(|| empty_vec.iter(), |vec| vec.iter());
        let to_stops_idx = self.stop_point_idx_to_stops_idx.get(&to_stop_point_idx).map_or_else(|| empty_vec.iter(), |vec| vec.iter());

        for from_stop_idx in from_stops_idx {
            let from_stop = & mut self.stops[from_stop_idx.idx];
            for to_stop_idx in to_stops_idx.clone() {
                from_stop.transfers.push((*to_stop_idx, duration, Some(transfer_idx)));
            }
        }                              
    }
    

    fn insert_vehicle_journey(& mut self, vehicle_journey_idx : Idx<VehicleJourney>
                                        , vehicle_journey : & VehicleJourney) {
        let stop_point_array : StopPointArray = vehicle_journey.stop_times
                                                .iter()
                                                .map(|stop_time| stop_time.stop_point_idx)
                                                .collect(); 

        let has_stop_pattern = self.stop_point_array_to_stop_pattern.get(&stop_point_array);
        let stop_pattern_idx = if let Some(stop_pattern_idx) = has_stop_pattern {
            *stop_pattern_idx
        }
        else {
            self.create_new_stop_pattern(stop_point_array)
        };
        let stop_pattern = & mut self.stop_patterns[stop_pattern_idx.idx];

        let departure_times : Vec<Time> = vehicle_journey
                                                        .stop_times.iter()
                                                        .map(|stop_time| stop_time.departure_time)
                                                        .collect();
        let arrival_times : Vec<Time> = vehicle_journey
                                            .stop_times.iter()
                                            .map(|stop_time| stop_time.arrival_time)
                                            .collect();
        let daily_trip_data = DailyTripData{
            vehicle_journey_idx : vehicle_journey_idx,
        };

        stop_pattern.departure_chains.insert(&departure_times, daily_trip_data.clone());
        stop_pattern.arrival_chains.insert(&arrival_times, daily_trip_data);


    }


    fn create_new_stop_pattern(& mut self, stop_point_array : StopPointArray) -> StopPatternIdx {
        debug_assert!( ! self.stop_point_array_to_stop_pattern.contains_key(&stop_point_array));

        let nb_of_positions = stop_point_array.len();
        let stop_pattern_idx = StopPatternIdx {
            idx : self.stop_patterns.len()
        };


        let mut stops = Vec::with_capacity(nb_of_positions);
        for (position_id, stop_point_idx) in stop_point_array.iter().enumerate() {
            let has_stops_idx = self.stop_point_idx_to_stops_idx.get(stop_point_idx);
            let stop_idx = match has_stops_idx {
                None => {
                    let new_stop_idx = self.add_new_stop_point(*stop_point_idx);
                    new_stop_idx
                },
                Some(stops_idx) => {
                    debug_assert!(stops_idx.len() >=1 );
                    let has_suitable_stop_idx = stops_idx.iter().find(|&&stop_idx| {
                        let stop = & self.stops[stop_idx.idx];
                        debug_assert!(stop.position_in_stop_patterns.len() >= 1);
                        // if stop_point_idx already appeared in this stop_pattern
                        // then stop_pattern will be the last element 
                        // in stop.position_in_stop_patterns
                        let last_pattern = stop.position_in_stop_patterns.last().unwrap().0;
                        last_pattern != stop_pattern_idx
                    });
                    if let Some(&stop_idx) = has_suitable_stop_idx {
                        stop_idx
                    }
                    // all stops associated to this stop_point_idx
                    // are already been used in this stop_pattern
                    // hence we create a new copy
                    else {
                        self.add_new_stop_point_copy(*stop_point_idx)
                    }
                }
            };
            let position = Position { idx : position_id};
            let stop = & mut self.stops[stop_idx.idx];
            stop.position_in_stop_patterns.push((stop_pattern_idx, position));

            stops.push(stop_idx);
        }



        let stop_pattern = StopPattern {
            stops ,
            departure_chains : ChainDecomposition::new(nb_of_positions),
            arrival_chains : ChainDecomposition::new(nb_of_positions)
        };

        self.stop_patterns.push(stop_pattern);

        self.stop_point_array_to_stop_pattern.insert(stop_point_array, stop_pattern_idx);

        stop_pattern_idx
    }

    fn add_new_stop_point(&mut self, stop_point_idx : Idx<StopPoint>) -> StopIdx {
        debug_assert!( ! self.stop_point_idx_to_stops_idx.contains_key(&stop_point_idx));
        let stop = Stop{ 
            stop_point_idx,
            position_in_stop_patterns : Vec::new(),
            transfers : Vec::new() 
        };
        let stop_idx = StopIdx {
            idx : self.stops.len()
        };
        self.stops.push(stop);
        let stops_idx = vec![stop_idx];
        self.stop_point_idx_to_stops_idx.insert(stop_point_idx, stops_idx);
        stop_idx
    }

    fn add_new_stop_point_copy(&mut self, stop_point_idx : Idx<StopPoint>) -> StopIdx {
        debug_assert!( self.stop_point_idx_to_stops_idx.contains_key(&stop_point_idx));
        let stop = Stop{ 
            stop_point_idx,
            position_in_stop_patterns : Vec::new(),
            transfers : Vec::new()
        };
        let stop_idx = StopIdx {
            idx : self.stops.len()
        };
        self.stops.push(stop);
        let  stops_idx = self.stop_point_idx_to_stops_idx.get_mut(&stop_point_idx).unwrap();
        debug_assert!(stops_idx.len() >= 1);
        stops_idx.push(stop_idx);
        stop_idx

    }

}
