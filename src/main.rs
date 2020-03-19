#[allow(dead_code, unused_imports)]

mod public_transit;
mod journeys_tree;
mod pareto_front;

use public_transit::PublicTransit;
use journeys_tree::{ JourneysTree};
use pareto_front::{OnboardFront, DebarkedFront, WaitingFront};

#[allow(dead_code)]
fn compute<PT : PublicTransit>(pt : & PT) -> () {
    let mut journeys_tree = JourneysTree::<PT>::new();

    let nb_of_route_stops = pt.nb_of_route_stops();
    // map a route_stop to the pareto front of Pathes which
    // ends at route_stop with a Transit 
    let mut debarked_fronts = vec![DebarkedFront::<PT>::new(); nb_of_route_stops];
    // map a route_stop to the pareto front of Pathes which
    // ends at route_stop with a Transfer or a Departure 
    let mut waiting_fronts = vec![WaitingFront::<PT>::new(); nb_of_route_stops];

    let nb_of_routes = pt.nb_of_routes();
    
    let mut route_has_a_new_board_path : Vec::<Option<PT::RouteStop>> = vec![None; nb_of_routes];
    let mut routes_with_new_board_path : Vec::<PT::Route> = Vec::new();


    for (route_stop, criteria) in pt.journey_departures() {

        let journey = journeys_tree.depart(&route_stop);
        let route_stop_id = pt.route_stop_id(&route_stop);
        waiting_fronts[route_stop_id].add(journey, criteria, pt);
        
        let route = pt.route_of(&route_stop);

        let route_id = pt.route_id(&route);
        if let Some(old_board_point) = &route_has_a_new_board_path[route_id] {
            if pt.is_upstream(&route_stop, old_board_point) {
                route_has_a_new_board_path[route_id] = Some(route_stop.clone());
            }
        }
        else {
            route_has_a_new_board_path[route_id] = Some(route_stop.clone());
            routes_with_new_board_path.push(route);
        }

        

    }

    let mut stops_with_a_new_debarked : Vec::<PT::RouteStop> = Vec::new();
    let mut stop_has_a_new_debarked = vec![false; nb_of_route_stops];

    while ! routes_with_new_board_path.is_empty() {
        for route in & routes_with_new_board_path {
            let route_id = pt.route_id(route);
            let mut has_stop = route_has_a_new_board_path[route_id].clone();
            let mut onboard_front = OnboardFront::new();
            while let Some(ref stop) = has_stop {
                let stop_id = pt.route_stop_id(&stop);
                // update debarked front at this stop with elements from
                //   onboard front

                let debarked_front = & mut debarked_fronts[stop_id];

                for ((ref onboard, ref trip), ref onboard_criteria) in onboard_front.iter() {

                    let new_debarked_criteria = pt.debark(trip, &stop, onboard_criteria);
                    if debarked_front.dominates(&new_debarked_criteria, pt) {
                        continue;
                    }
                    let new_debarked = journeys_tree.debark(onboard, &stop);
                    let updated = debarked_front.add_unchecked(new_debarked, new_debarked_criteria, pt);
                    if updated && ! stop_has_a_new_debarked[stop_id]{
                        stop_has_a_new_debarked[stop_id] = true;
                        stops_with_a_new_debarked.push(stop.clone());
                    }

                }

                // update onboard front with from boardings from waitings
                let waiting_front = &waiting_fronts[stop_id];
                for (ref waiting, ref waiting_criteria) in waiting_front.iter() {
                    let (trip, new_onboard_criteria) = pt.board(&stop, waiting_criteria);
                    if onboard_front.dominates(&new_onboard_criteria, pt) {
                        continue;
                    }
                    let new_onboard = journeys_tree.board(waiting, &trip);
                    onboard_front.add_unchecked((new_onboard, trip), new_onboard_criteria, pt);

                }


                // ride to the next stop point and update "working"
                //   pareto front along the way
                let mut new_onboard_front = OnboardFront::<PT>::new();
                for ((ref onboard, ref trip), ref criteria) in onboard_front.iter() {
                    // TODO : here !!

                }


                // go to next route stop and iterate
                has_stop = pt.next_on_route(stop);
            }

        }
    }

    //while (continue algorithm)
    //   explore routes with a new board path
    //   explore transfers from stop_points with a new alighting path
    

}

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


