
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
    arrival_times : Vec<Time>,
    departure_times : Vec<Time>,
    transit_model_vehicle_journey : Idx<VehicleJourney>,
}



struct TransitData {
    transit_model : Model,
    missions : Vec<MissionData>,
    stops : Vec<StopData>,

}

impl TransitData {
    pub fn new(transit_model : Model, default_transfer_duration : Duration) -> () {

        let nb_of_stops = transit_model.stop_points.len();

        let mut idx_stop_point_to_stop = HashMap::<Idx<StopPoint>, Stop>::with_capacity(nb_of_stops);

        let mut stops_data = Vec::<StopData>::with_capacity(nb_of_stops);
        for (idx, _) in transit_model.stop_points.iter() {
            let stop_id = stops_data.len();
            let stop = Stop{ id : stop_id};
            let stop_data = StopData {
                transit_model_id : idx,
                boardable_missions : Vec::new(),
                position_in_mission : Vec::new(),
                transfers : Vec::new()
            };
            stops_data.push(stop_data);
            idx_stop_point_to_stop.insert(idx, stop);
        }
        
        for (stop_point_idx, _) in transit_model.stop_points.iter() {
            let stop = &idx_stop_point_to_stop[&stop_point_idx];
            let stop_data = & mut stops_data[stop.id];
            for transfer_idx in transit_model.get_corresponding_from_idx::<_,Transfer>(stop_point_idx) {
                let transfer = &transit_model.transfers[transfer_idx];
                let arrival_stop_point_id = &transfer.to_stop_id;
                let has_arrival_stop_point_idx = transit_model.stop_points.get_idx(arrival_stop_point_id);
                if has_arrival_stop_point_idx.is_none() {
                    //TODO : log some error
                    continue;
                }
                let arrival_stop_point_idx = has_arrival_stop_point_idx.unwrap();
                let arrival_stop = & idx_stop_point_to_stop[&arrival_stop_point_idx];

                let duration = transfer.real_min_transfer_time.map_or(default_transfer_duration, |seconds| { Duration{seconds} });

                stop_data.transfers.push((*arrival_stop, duration, transfer_idx));

            }
        }


        // Identify missions by sequence of stops served
        let mut missions_data : Vec<MissionData> = Vec::new();
        type StopSequence =  Vec<Idx<StopPoint>>;
        let mut stop_sequence_to_mission : BTreeMap< StopSequence, Mission> = BTreeMap::new();
        for (vehicle_journey_idx, vehicle_journey) in transit_model.vehicle_journeys.iter() {
            let stop_sequence : StopSequence = vehicle_journey.stop_times.iter()
                                .map(|stop_time| stop_time.stop_point_idx)
                                .collect(); 
            let mission = stop_sequence_to_mission.entry(stop_sequence)
                                                .or_insert_with(|| Mission{id : missions_data.len()} );
            let has_mission = stop_sequence_to_mission.get(stop_sequence);
            let mission = match has_mission {
                Some(mission) => { mission},
                None => {
                    let mission =  Mission{id : missions_data.len()};
                    mission
                }
            };

        }

    }
}

// read all stops and put them in MCData.stops
// read all transfers, and fill MCData.stops.transfers 

// read all vehicle_journeys to identify mission patterns 
//   based on sequence of stop_point_idx in vj.stop_times
// read again all vehicle_journeys and fill MCData.missions with a new tripdata for each vj