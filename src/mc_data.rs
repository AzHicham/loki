
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
    missions : Vec<Mission>,
}

struct Mission {
    stops : Vec<Idx<StopPoint>>,
    trips : Vec<Trip>,
}

struct Stop {
    uri : String
}

struct Trip {
    arrival_times : Vec<Time>,
    departure_times : Vec<Time>,
    transit_model_vehicle_journey : Idx<VehicleJourney>,
}

// /
// / stop_points_with_transit_to_explore : List<StopPoint>
// / init stop_points_with_transit_to_explore with reachable stop_points
// / while stop_points_with_transit_to_explore is not empty {
// /    paths_by_stop_points : Map<StopPoint, ParetoFront<Path> >
// /    for stop_point : stop_points_with_transit_to_explore {
// /        stop_points_with_transfers_to_explore : List<StopPoint>
// /        for path in paths_by_stop_points(stop_point) {
// /            old_objective = path.objective
// /            for route in routes(stop_point) {
// /                (vehicle_journey, objective) = embark(stop_point, old_objective)
// /                for (new_stop_point, new_objective) in ride(vehicle_journey, objective) {
// /                    old_pareto_front = paths_by_stop_points(new_stop_point)
// /                    // update old_pareto_front with a new_path with new_objective
// /                    // if old_pareto_front is updated : 
// /                        add new_stop_point to stop_points_with_a_transfer_to_explore
// /                }
// /            }
// /        }
// /    }
// /    for stop_point : stop_points_with_transfers_to_explore {
// /        perform all transfer from stop_point
// /        update pareto front at arrival
// /        push arrival_stop_point to stop_points_with_transit_to_explore if pareto front has been updated
// /    }
// / }
// / 
// / path : sequence of (StopPoint, Trip, Transfer)
// / 
// / 
// / 
// / 


