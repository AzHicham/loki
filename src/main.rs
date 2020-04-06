#[allow(dead_code, unused_imports)]

mod public_transit;
mod journeys_tree;
mod pareto_front;
#[allow(dead_code)]
mod multicriteria_raptor;

use public_transit::PublicTransit;
use journeys_tree::{ JourneysTree};
use pareto_front::{OnboardFront, DebarkedFront, WaitingFront, ArrivedFront};

#[allow(dead_code)]
fn compute<PT : PublicTransit>(pt : & PT) -> () {
    let mut journeys_tree = JourneysTree::<PT>::new();

    let nb_of_stops = pt.nb_of_stops();

    // map a `stop` to the pareto front of Pathes which
    // ends at `stop` with a Transfer or a Departure 
    let mut waiting_fronts = vec![WaitingFront::<PT>::new(); nb_of_stops];
    let mut new_waiting_fronts = vec![WaitingFront::<PT>::new(); nb_of_stops];
    let mut stops_with_a_new_waiting : Vec::<PT::Stop> = Vec::new();

    let nb_of_routes = pt.nb_of_routes();
 
    let mut route_has_a_new_waiting_path : Vec::<Option<PT::Stop>> = vec![None; nb_of_routes];
    let mut routes_with_new_waiting_path : Vec::<PT::Route> = Vec::new();

    for (stop, criteria) in pt.journey_departures() {
        let journey = journeys_tree.depart(&stop);
        let stop_id = pt.stop_id(&stop);

        let new_waiting_front = & mut new_waiting_fronts[stop_id];
        if new_waiting_front.is_empty() {
            stops_with_a_new_waiting.push(stop.clone());
        }

        new_waiting_front.add(journey, criteria, pt);
        
        let route = pt.route_of(&stop);

        let route_id = pt.route_id(&route);
        if let Some(old_waiting_stop) = &route_has_a_new_waiting_path[route_id] {
            if pt.is_upstream(&stop, old_waiting_stop) {
                route_has_a_new_waiting_path[route_id] = Some(stop.clone());
            }
        }
        else {
            route_has_a_new_waiting_path[route_id] = Some(stop.clone());
            routes_with_new_waiting_path.push(route);
        }

    }



    // map a `stop` to the pareto front of Pathes which
    // ends at `stop` with a Transit 
    let mut debarked_fronts = vec![DebarkedFront::<PT>::new(); nb_of_stops];
    let mut new_debarked_fronts = vec![DebarkedFront::<PT>::new(); nb_of_stops];

    let mut stops_with_a_new_debarked : Vec::<PT::Stop> = Vec::new();
    //TODO : can be replaced with new_debarked_fronts[stop].is_empty()
    let mut stop_has_a_new_debarked = vec![false; nb_of_stops];

    let mut onboard_front = OnboardFront::<PT>::new(); 
    let mut new_onboard_front = OnboardFront::<PT>::new();



    let mut arrived_front = ArrivedFront::<PT>::new();

    while ! routes_with_new_waiting_path.is_empty() {


        // let's store all new_waiting_fronts in waiting_fronts
        for stop in stops_with_a_new_waiting.drain(..) {
            let stop_id = pt.stop_id(&stop);
            let waiting_front = & mut waiting_fronts[stop_id];
            let new_waiting_front = & new_waiting_fronts[stop_id];
            for (waiting, criteria) in new_waiting_front.iter() {
                waiting_front.add_unchecked(waiting.clone(), criteria.clone());
            }
        }

        // let's ride the routes which have an new waiting path
        debug_assert!(stops_with_a_new_debarked.is_empty());
        debug_assert!(stop_has_a_new_debarked.iter().all(|has| {! has}));
        debug_assert!(new_debarked_fronts.iter().all(|front| { front.is_empty() } ));

        for route in  routes_with_new_waiting_path.drain(..) {
            let route_id = pt.route_id(&route);
            // we recover the stop at which we start riding the route
            //  and put at None in route_has_a_new_waiting_path[route_id]
            let mut has_stop = route_has_a_new_waiting_path[route_id].take();
            
            while let Some(stop) = has_stop {
                let stop_id = pt.stop_id(&stop);
                // update debarked front at this stop with elements from
                //   onboard front
                { 
                    let debarked_front = & mut debarked_fronts[stop_id];
                    let new_debarked_front = & mut new_debarked_fronts[stop_id];

                    for ((ref onboard, ref trip), ref onboard_criteria) in onboard_front.iter() {

                        let new_debarked_criteria = pt.debark(trip, &stop, onboard_criteria);
                        if debarked_front.dominates(&new_debarked_criteria, pt) {
                            continue;
                        }
                        if new_debarked_front.dominates(&new_debarked_criteria, pt) {
                            continue;
                        }
                        let new_debarked = journeys_tree.debark(onboard, &stop);
                        debarked_front.remove_elements_dominated_by( &new_debarked_criteria, pt);                         
                        new_debarked_front.add_and_remove_elements_dominated(new_debarked, new_debarked_criteria, pt);
                        if  ! stop_has_a_new_debarked[stop_id]{
                            stop_has_a_new_debarked[stop_id] = true;
                            stops_with_a_new_debarked.push(stop.clone());
                        }

                    }
                }


                // we update has_stop to the next stop on the route
                has_stop = pt.next_on_route(&stop);

                // if there is no next stop on the route
                // there is no need to the update onboard front
                if has_stop.is_none() {
                    continue;
                }

                // update onboard front with boardings from new waitings
                {
                    let new_waiting_front = & mut new_waiting_fronts[stop_id];
                    for (waiting, waiting_criteria) in new_waiting_front.drain() {
                        let (trip, new_onboard_criteria) = pt.board(&stop, &waiting_criteria);
                        if onboard_front.dominates(&new_onboard_criteria, pt) {
                            continue;
                        }
                        let new_onboard = journeys_tree.board(&waiting, &trip);
                        onboard_front.add_and_remove_elements_dominated((new_onboard, trip), new_onboard_criteria, pt);

                    }
                }

                // ride to the next stop point and update onboard
                //   pareto front along the way
                {
                    new_onboard_front.clear();
                    for ((ref onboard, ref trip), ref criteria) in onboard_front.iter() {
                        let new_criteria = pt.ride(trip, &stop, criteria);
                        new_onboard_front.add((onboard.clone(), trip.clone()), new_criteria, pt);

                    }
                    onboard_front.replace_with(& mut new_onboard_front);
                    new_onboard_front.clear();
                }
            }

        }

        // let's store all new_debarked_fronts in debarked_fronts
        for stop in & stops_with_a_new_debarked {
            let stop_id = pt.stop_id(&stop);
            let debarked_front = & mut debarked_fronts[stop_id];
            let new_debarked_front = & new_debarked_fronts[stop_id];
            for (debarked, criteria) in new_debarked_front.iter() {
                // we do not need to check, because 
                //  - new_debarked_front is a pareto front 
                //  - we added an element to new_debarked_front only if it was not dominated by debarked_front
                //  - we removed from debarked_front all elements that were dominated by an element of new_debarked_front
                debarked_front.add_unchecked(debarked.clone(), criteria.clone());
            }
        }

        // let's perform transfers from newly debarked pathes
        //  as well as arrivals
        debug_assert!(routes_with_new_waiting_path.is_empty());
        debug_assert!(route_has_a_new_waiting_path.iter().all(|has| has.is_none()));
        debug_assert!(new_waiting_fronts.iter().all(|front| front.is_empty()));
        debug_assert!(stops_with_a_new_waiting.is_empty());
        for stop in stops_with_a_new_debarked.drain(..) {
            let stop_id = pt.stop_id(&stop);
            let new_debarked_front = & mut new_debarked_fronts[stop_id];
            for (debarked, criteria) in new_debarked_front.drain() {
                // we perform arrival from the `debarked` path
                if let Some(arrived_criteria) = pt.journey_arrival(&stop, &criteria) {
                    let arrived = journeys_tree.arrive(&debarked);
                    arrived_front.add(arrived, arrived_criteria, &pt);
                }
                // we perform all transfers from the `debarked` path
                for (arrival_stop, arrival_criteria) in pt.transfers(&stop, &criteria) {
                    let arrival_id = pt.stop_id(&arrival_stop);
                    let waiting_front = & mut waiting_fronts[arrival_id];
                    let new_waiting_front = & mut new_waiting_fronts[arrival_id];
                    if waiting_front.dominates(&arrival_criteria, pt) {
                        continue;
                    }
                    if new_waiting_front.dominates(&arrival_criteria, pt) {
                        continue;
                    }

                    if new_waiting_front.is_empty() {
                        stops_with_a_new_waiting.push(arrival_stop.clone());
                    }

                    let waiting = journeys_tree.transfer(&debarked, &arrival_stop);
                    waiting_front.remove_elements_dominated_by( &arrival_criteria, pt);
                    new_waiting_front.add_and_remove_elements_dominated(waiting, arrival_criteria, pt);



                    let route = pt.route_of(&arrival_stop);
                    let route_id = pt.route_id(&route);
                    if let Some(old_waiting_stop) = &route_has_a_new_waiting_path[route_id] {
                        if pt.is_upstream(&arrival_stop, old_waiting_stop) {
                            route_has_a_new_waiting_path[route_id] = Some(arrival_stop.clone());
                        }
                    }
                    else {
                        route_has_a_new_waiting_path[route_id] = Some(arrival_stop.clone());
                        routes_with_new_waiting_path.push(route);
                    }
                    
                }
            }
        }



        // TODO : what about arrivals ?

        // TODO : when to stop ?
    }


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


