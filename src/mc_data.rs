
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
struct Mission {
    id : usize,
}

struct Trip {
    mission : Mission,
    id : usize
}
#[derive(Debug, PartialEq, Copy, Clone)]
struct Stop {
    id : usize
}



struct MissionData {
    stops : Vec<Stop>,
    trips : Vec<TripData>,
}

struct TripData {
    vehicle_journey_idx : Idx<VehicleJourney>,
}

struct StopData {
    stop_point_idx : Idx<StopPoint>,
    position_in_missions : Vec<(Mission, usize)>,
    transfers : Vec<(Stop, Duration, Idx<Transfer>)>
}

type StopPointSequence = Vec< Idx<StopPoint> >;

struct TransitData {
    stop_point_sequence_to_mission : BTreeMap< StopPointSequence, Mission>,
    stop_point_idx_to_main_stop : BTreeMap< Idx<StopPoint>, Stop >,
    // when a stop_point appears more than once in a mission, we create
    // another stop associated with the same stop_point for each extra occurence
    stop_point_idx_to_extra_stops : BTreeMap< Idx<StopPoint>, Vec<Stop> >,

    stops_data : Vec<StopData>,
    missions_data : Vec<MissionData>
}



impl TransitData {
    pub fn new(transit_model : & Model, default_transfer_duration : Duration) -> TransitData {

        // let mut stop_sequence_to_mission : BTreeMap< StopSequence, Mission> = BTreeMap::new();
        // let mut stop_point_idx_to_stops : BTreeMap< Idx<StopPoint>, Vec<Stop> > = BTreeMap::new();
        // let mut stops_data : Vec<StopData> = Vec::with_capacity(transit_model.stop_points.len());
        // let mut missions_data : Vec<MissionData> = Vec::new();


        let mut transit_data = TransitData {
            stop_point_sequence_to_mission : BTreeMap::new(),
            stop_point_idx_to_main_stop : BTreeMap::new(),
            stop_point_idx_to_extra_stops : BTreeMap::new(),
            stops_data : Vec::with_capacity(transit_model.stop_points.len()),
            missions_data : Vec::new(),
        };

        for (vehicle_journey_idx, vehicle_journey) in transit_model.vehicle_journeys.iter() {
            transit_data.insert_vehicle_journey(vehicle_journey_idx, vehicle_journey);
        }

        for (transfer_idx, transfer) in transit_model.transfers.iter() {
            let has_from_stop_point_idx = transit_model.stop_points.get_idx(&transfer.from_stop_id);
            let has_to_stop_point_idx = transit_model.stop_points.get_idx(&transfer.to_stop_id);
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
        let stop_point_sequence : StopPointSequence = vehicle_journey.stop_times
                                                .iter()
                                                .map(|stop_time| stop_time.stop_point_idx)
                                                .collect(); 

        let has_mission = self.stop_point_sequence_to_mission.get(&stop_point_sequence);
        let mission = if let Some(mission) = has_mission {
            *mission
        }
        else {
            self.create_new_mission(stop_point_sequence)
        };
        let mission_data = & mut self.missions_data[mission.id];
        let trip_data = TripData{
            vehicle_journey_idx : vehicle_journey_idx
        };
        mission_data.trips.push(trip_data);

    }

    fn create_new_mission(& mut self, stop_point_sequence : StopPointSequence) -> Mission {
        debug_assert!( ! self.stop_point_sequence_to_mission.contains_key(&stop_point_sequence));
        let mission = Mission { id : self.missions_data.len() };

        let mut stops : Vec<Stop> = Vec::with_capacity(stop_point_sequence.len());
        for (position, stop_point_idx) in stop_point_sequence.iter().enumerate() {
            let has_main_stop = self.stop_point_idx_to_main_stop.get(stop_point_idx);
            let stop = match has_main_stop {
                None => {
                    let new_stop = self.add_new_main_stop(*stop_point_idx);
                    new_stop
                },
                Some(main_stop) => {
                    let main_stop_data = & self.stops_data[main_stop.id];
                    let has_last_mission = main_stop_data.position_in_missions.last();
                    match has_last_mission {
                        // if this `stop_point_idx` already appeared in `stop_point_sequence`,
                        // then `(mission, _)` is the last element of 
                        //   main_stop_data.position_in_mission
                        // in this case, we create a new extra stop to ensure that each `Stop`
                        // appears only once in a `Mission`
                        Some((last_mission, _)) if *last_mission == mission => {
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
            stop_data.position_in_missions.push((mission, position));
            stops.push(stop);
        }

        let mission_data = MissionData {
            stops,
            trips : Vec::new()
        };
        self.missions_data.push(mission_data);

        self.stop_point_sequence_to_mission.insert(stop_point_sequence, mission);

        mission
    }

    fn add_new_main_stop(&mut self, stop_point_idx : Idx<StopPoint>) -> Stop {
        debug_assert!( ! self.stop_point_idx_to_main_stop.contains_key(&stop_point_idx));
        let stop = Stop{ id : self.stops_data.len()};
        let stop_data = StopData {
            stop_point_idx,
            position_in_missions : Vec::new(),
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
            position_in_missions : Vec::new(),
            transfers : Vec::new()
        };
        self.stops_data.push(stop_data);
        let extra_stops = self.stop_point_idx_to_extra_stops.entry(stop_point_idx).or_insert(Vec::new());
        extra_stops.push(stop);
        stop
    }
}

// read all stops and put them in MCData.stops
// read all transfers, and fill MCData.stops.transfers 

// read all vehicle_journeys to identify mission patterns 
//   based on sequence of stop_point_idx in vj.stop_times
// read again all vehicle_journeys and fill MCData.missions with a new tripdata for each vj