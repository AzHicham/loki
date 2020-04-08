#[allow(dead_code, unused_imports)]

mod public_transit;
mod journeys_tree;
mod pareto_front;
#[allow(dead_code)]
mod multicriteria_raptor;

use public_transit::PublicTransit;
use journeys_tree::{ JourneysTree};
use pareto_front::{OnboardFront, DebarkedFront, WaitingFront, ArrivedFront};



fn main() {
    println!("Hello, world!");
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


