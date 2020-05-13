
use transit_model;
use transit_model::{
    model::{Model, Idx},
    objects::{StopPoint, VehicleJourney, Transfer, Time},
}; 
use std::path::PathBuf;
use std::collections::{HashMap, BTreeMap};

fn run() {
    let input_dir = PathBuf::from("tests/fixtures/small_ntfs/");
    let model = transit_model::ntfs::read(input_dir).unwrap();
    let collections = model.into_collections();
    dbg!(collections.vehicle_journeys);

    println!("Hello, world!");
}

// TODO : group collections.vehicle_journeys by mission (same sequence of stop_points)
//    info : each VehicleJourney.stop_time is sorted by stop_time.sequence in ntfs::read::manage_stop_times
#[derive(Debug, Copy, Clone)]
struct Duration {
    seconds : u32
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
struct StopPattern {
    id : usize,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
struct Position {
    id : usize,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
struct Chain {
    stop_pattern : StopPattern,
    id : usize,
}



#[derive(Debug, PartialEq, Eq, Copy, Clone)]
struct DailyTrip {
    chain : Chain,
    id : usize
    
}

struct Trip {
    daily_trip : DailyTrip,
    day : u32,
}


#[derive(Debug, PartialEq, Copy, Clone)]
struct Stop {
    id : usize
}

// Trip :
//   DailyTrip (aka VehicleJourney)
//   Day

// in each Mission, sort all DailyTrip by increasing departure data
// for each stop in a mission, compute the minimum duration from the first stop of the mission
//    among all DailyTrips

// In order to find the earliest trip leaving after a given DateTime :
//  - find the 

// In Mission : 
// store, for each position, a BTreeMap : departure_time -> DailyTrip
//  -> earliest departure after some time is a search in the map
//  -> lastest arrival before some time is also a search in the map

struct StopPatternData {
    stops : Vec<Stop>,
    departure_chains : Vec<ChainData>,

}

struct ChainData {
    // daily_trips, ordered by increasing times
    daily_trips : Vec<DailyTripData>,

    // times_by_position[position][daily_trip]
    // is the time at which `daily_trip` is present at `position`
    // so for each `position`, `times_by_position[position]`
    // is a vector of size `daily_trips.len()`
    times_by_position : Vec<Vec<Time>>  
}

struct DailyTripData {
    vehicle_journey_idx : Idx<VehicleJourney>,

}

struct StopData {
    stop_point_idx : Idx<StopPoint>,
    position_in_stop_patterns : Vec<(StopPattern, Position)>,
    transfers : Vec<(Stop, Duration, Idx<Transfer>)>
}

type StopPointArray = Vec< Idx<StopPoint> >;

struct TransitData {
    transit_model : Model,
    stop_point_array_to_stop_pattern : BTreeMap< StopPointArray, StopPattern>,
    stop_point_idx_to_main_stop : BTreeMap< Idx<StopPoint>, Stop >,
    // when a stop_point appears more than once in a mission, we create
    // another stop associated with the same stop_point for each extra occurence
    stop_point_idx_to_extra_stops : BTreeMap< Idx<StopPoint>, Vec<Stop> >,

    stops_data : Vec<StopData>,
    stop_patterns_data : Vec<StopPatternData>
}



impl TransitData {
    pub fn new(transit_model :  Model, default_transfer_duration : Duration) -> TransitData {


        let mut transit_data = TransitData {
            transit_model,
            stop_point_array_to_stop_pattern : BTreeMap::new(),
            stop_point_idx_to_main_stop : BTreeMap::new(),
            stop_point_idx_to_extra_stops : BTreeMap::new(),
            stops_data : Vec::with_capacity(transit_model.stop_points.len()),
            stop_patterns_data : Vec::new(),
        };

        for (vehicle_journey_idx, vehicle_journey) in transit_data.transit_model.vehicle_journeys.iter() {
            transit_data.insert_vehicle_journey(vehicle_journey_idx, vehicle_journey);
        }

        for (transfer_idx, transfer) in transit_data.transit_model.transfers.iter() {
            let has_from_stop_point_idx = transit_data.transit_model.stop_points.get_idx(&transfer.from_stop_id);
            let has_to_stop_point_idx = transit_data.transit_model.stop_points.get_idx(&transfer.to_stop_id);
            match (has_from_stop_point_idx, has_to_stop_point_idx) {
                (Some(from_stop_point_idx), Some(to_stop_point_idx)) => {
                    let duration = transfer.real_min_transfer_time.map_or(default_transfer_duration, |seconds| { Duration{seconds} });
                    transit_data.insert_transfer(from_stop_point_idx, to_stop_point_idx, transfer_idx, duration)
                }
                _ => {
                    //TODO : log some error
                    continue;
                }
            }

        }

        transit_data
    }

    fn insert_transfer(& mut self, from_stop_point_idx : Idx<StopPoint>
                                , to_stop_point_idx : Idx<StopPoint>
                                , transfer_idx : Idx<Transfer>
                                , duration : Duration ) 
    { 
        let has_main_from_stop = self.stop_point_idx_to_main_stop.get(&from_stop_point_idx).copied();
        let has_main_to_stop = self.stop_point_idx_to_main_stop.get(&to_stop_point_idx).copied();
        let empty_vec : Vec<Stop> = Vec::new();
        let extra_from_stops = self.stop_point_idx_to_extra_stops.get(&from_stop_point_idx).map_or_else(|| empty_vec.iter(), |vec| vec.iter());
        let extra_to_stops = self.stop_point_idx_to_extra_stops.get(&to_stop_point_idx).map_or_else(|| empty_vec.iter(), |vec| vec.iter());

        for from_stop in has_main_from_stop.iter().chain(extra_from_stops) {
            let from_stop_data = & mut self.stops_data[from_stop.id];
            for to_stop in has_main_to_stop.iter().chain(extra_to_stops.clone()) {
                from_stop_data.transfers.push((*to_stop, duration, transfer_idx));
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
        let stop_pattern = if let Some(stop_pattern) = has_stop_pattern {
            *stop_pattern
        }
        else {
            self.create_new_stop_pattern(stop_point_array)
        };
        let stop_pattern_data = & mut self.stop_patterns_data[stop_pattern.id];

        // TODO : insert the vehicle_journey stop_times in stop_pattern_data
        //       make a function that computes the right chain based on times,
        //       and insert the vehicle_journey in the right chain

        let departure_times : Vec<Time> = self.transit_model.vehicle_journeys[vehicle_journey_idx]
                                                        .stop_times.iter()
                                                        .map(|stop_time| stop_time.departure_time)
                                                        .collect();
        let arrival_times : Vec<Time> = self.transit_model.vehicle_journeys[vehicle_journey_idx]
                                            .stop_times.iter()
                                            .map(|stop_time| stop_time.arrival_time)
                                            .collect();
        let departure_daily_trip_data = DailyTripData{
            vehicle_journey_idx : vehicle_journey_idx,
        };

        for departure_chain_data in stop_pattern_data.departure_chains.iter() {
            if departure_chain_data.accept(&departure_times) {
                departure_chain_data.insert(&departure_times, departure_daily_trip_data);
            }
        }
        mission_data.daily_trips.push(daily_trip_data);

        for (position, stop_time) in vehicle_journey.stop_times.iter().enumerate() {
            mission_data.departure_times_by_position[position].insert(key, value)
        }
 

    }


    fn create_new_stop_pattern(& mut self, stop_point_array : StopPointArray) -> StopPattern {
        debug_assert!( ! self.stop_point_array_to_stop_pattern.contains_key(&stop_point_array));
        let stop_pattern = StopPattern { id : self.stop_patterns_data.len() };

        let mut stops : Vec<Stop> = Vec::with_capacity(stop_point_array.len());
        for (position_id, stop_point_idx) in stop_point_array.iter().enumerate() {
            let has_main_stop = self.stop_point_idx_to_main_stop.get(stop_point_idx);
            let stop = match has_main_stop {
                None => {
                    let new_stop = self.add_new_main_stop(*stop_point_idx);
                    new_stop
                },
                Some(main_stop) => {
                    let main_stop_data = & self.stops_data[main_stop.id];
                    let has_last_pattern = main_stop_data.position_in_stop_patterns.last();
                    match has_last_pattern {
                        // if this `stop_point_idx` already appeared in `stop_point_array`,
                        // then `(mission, _)` is the last element of 
                        //   main_stop_data.position_in_stop_patterns
                        // in this case, we create a new extra stop to ensure that each `Stop`
                        // appears only once in a `StopPattern`
                        Some((last_pattern, _)) if *last_pattern == stop_pattern => {
                            let new_extra_stop = self.add_new_extra_stop(*stop_point_idx);
                            new_extra_stop
                        },
                        // here we know that  `stop_point_idx` did not appear in `stop_point_sequence`
                        // hence we can add `mission` to the main_stop
                        _ => {
                            *main_stop
                        }
                    }

                }
            };
            let stop_data = & mut self.stops_data[stop.id];
            let position = Position { id : position_id};
            stop_data.position_in_stop_patterns.push((stop_pattern, position));
            stops.push(stop);
        }

        let stop_pattern_data = StopPatternData {
            stops,
            departure_chains : Vec::new()
        };
        self.stop_patterns_data.push(stop_pattern_data);

        self.stop_point_array_to_stop_pattern.insert(stop_point_array, stop_pattern);

        stop_pattern
    }

    fn add_new_main_stop(&mut self, stop_point_idx : Idx<StopPoint>) -> Stop {
        debug_assert!( ! self.stop_point_idx_to_main_stop.contains_key(&stop_point_idx));
        let stop = Stop{ id : self.stops_data.len()};
        let stop_data = StopData {
            stop_point_idx,
            position_in_stop_patterns : Vec::new(),
            transfers : Vec::new()
        };
        self.stops_data.push(stop_data);
        self.stop_point_idx_to_main_stop.insert(stop_point_idx, stop);
        stop
    }

    fn add_new_extra_stop(&mut self, stop_point_idx : Idx<StopPoint>) -> Stop {
        debug_assert!( self.stop_point_idx_to_main_stop.contains_key(&stop_point_idx));
        let stop = Stop{ id : self.stops_data.len()};
        let stop_data = StopData {
            stop_point_idx,
            position_in_stop_patterns : Vec::new(),
            transfers : Vec::new()
        };
        self.stops_data.push(stop_data);
        let extra_stops = self.stop_point_idx_to_extra_stops.entry(stop_point_idx).or_insert(Vec::new());
        extra_stops.push(stop);
        stop
    }
}


impl ChainData {

    fn new(nb_of_positions : usize) -> Self {
        assert!( nb_of_positions >= 1);
        ChainData{
            daily_trips : Vec::new(),
            times_by_position : vec![Vec::new(); nb_of_positions]
        }
    }

    fn nb_of_positions(&self) -> usize {
        self.times_by_position.len()
    }

    fn daily_trip_times<'a>(& 'a self, daily_trip_idx : usize) -> DailyTripTimesIter<'a> {
        debug_assert!( daily_trip_idx < self.daily_trips.len() );
        DailyTripTimesIter {
            chain_data : & self,
            daily_trip_idx,
            position : 0
        }
    }

    //
    // test whether, for all daily_trip_times vector 
    // we have either 
    //    - times[pos] <= daily_trip_times[pos] for all pos
    //    - times[pos] >= daily_trip_times[pos] for all pos
    // if this happens, we say that the two vectors are "comparable"
    fn accept(& self, times :&[Time]) -> bool {
        use std::cmp::Ordering;
        debug_assert!( times.len() == self.nb_of_positions() );
        for daily_trip_idx in 0..self.daily_trips.len() {
            let daily_trip_times = self.daily_trip_times(daily_trip_idx);
            let zip_iter = times.iter().zip(daily_trip_times);
            let first_not_equal_iter = zip_iter.skip_while(|&(left, right)| *left == right);
            let has_first_not_equal = first_not_equal_iter.next();
            if let Some(first_not_equal) = has_first_not_equal {
                let ordering = first_not_equal.0.cmp(&first_not_equal.1);
                assert!( ordering != Ordering::Equal);
                // let's see if there is a position where the ordering is not the same
                // as first_ordering
                let found = first_not_equal_iter.find(|&(left, right)| {
                    let cmp = left.cmp(&right);
                    cmp != ordering && cmp != Ordering::Equal
                });
                if found.is_some() {
                    return false;
                }
                // if found.is_none(), it means that 
                // all elements are ordered the same, so the two times vectors are comparable

            }
            // if has_first_not_equal == None
            // then times == daily_trip_times
            // the two  vector are comparable
        }
        true
    }

    fn insert(& mut self,  times :&[Time],  daily_trip_data : DailyTripData)
    {
        debug_assert!(self.accept(times));
        //let's find where to insert our new times vector
        let first_time = times[0];
        let times_at_first_pos = self.times_by_position[0];
        debug_assert!(self.is_sorted());
        let search_insert_idx = self.times_by_position[0].binary_search(&first_time);
        let insert_idx = match search_insert_idx {
            Result::Ok(idx) => idx,
            Result::Err(idx) => idx 
        };
        for pos in 0..self.nb_of_positions() {
            self.times_by_position[pos].insert(insert_idx, times[pos]);
        }
        self.daily_trips.insert(insert_idx, daily_trip_data);
    }

    fn is_sorted(&self) -> bool {
        for pos in 0..self.nb_of_positions() {
            let pos_times = self.times_by_position[pos];
            let pos_sorted = (0..self.daily_trips.len() - 1).all(|i| pos_times[i] <= pos_times[i + 1]);
            if ! pos_sorted {
                return false;
            }
        }
        true
    }
}

struct DailyTripTimesIter<'a> {
    chain_data : & 'a ChainData,
    daily_trip_idx : usize,
    position : usize,
}

impl<'a> Iterator for DailyTripTimesIter<'a> {
    type Item = Time;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.chain_data.times_by_position.len() {
            None
        }
        else {
            let result = self.chain_data.times_by_position[self.position][self.daily_trip_idx];
            self.position = self.position + 1;
            Some(result)
            
        }
    }
}
// read all stops and put them in MCData.stops
// read all transfers, and fill MCData.stops.transfers 

// read all vehicle_journeys to identify mission patterns 
//   based on sequence of stop_point_idx in vj.stop_times
// read again all vehicle_journeys and fill MCData.missions with a new tripdata for each vj