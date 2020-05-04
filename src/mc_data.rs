
use transit_model;
use transit_model::{
    model::Idx,
    objects::{StopPoint, VehicleJourney, Time},
}; 
use std::path::PathBuf;

fn run() {
    let input_dir = PathBuf::from("tests/fixtures/small_ntfs/");
    let model = transit_model::ntfs::read(input_dir).unwrap();
    let collections = model.into_collections();
    dbg!(collections.vehicle_journeys);

    println!("Hello, world!");
}

// TODO : group collections.vehicle_journeys by mission (same sequence of stop_points)
//    info : each VehicleJourney.stop_time is sorted by stop_time.sequence in ntfs::read::manage_stop_times


struct Route {
    id : usize
}

struct Mission {
    route : Route,
    id : usize,
}

struct Trip {
    mission : Mission,
    id : usize
}

struct Stop {
    id : usize
}



struct RouteData {
    missions : Vec<MissionData>,
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

struct StopData {
    transit_model_id : Idx<StopPoint>,
    boardable_missions : Vec<Mission>,
    position_in_mission : Vec<(Mission, usize)>,
    transfers : Vec<(Stop, Time)>
}

struct MCData {
    routes : Vec<RouteData>,
    stops : Vec<StopData>,
}

// read all stops and put them in MCData.stops
// read all transfers, and fill MCData.stops.transfers 

// read all vehicle_journeys to identify mission patterns 
//   based on sequence of stop_point_idx in vj.stop_times
// read again all vehicle_journeys and fill MCData.missions with a new tripdata for each vj