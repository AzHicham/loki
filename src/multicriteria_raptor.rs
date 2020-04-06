use crate::public_transit::PublicTransit;
use crate::journeys_tree::{JourneysTree};
use crate::pareto_front::{OnboardFront, DebarkedFront, WaitingFront, ArrivedFront};



pub struct MultiCriteriaRaptor<'pt, PT : PublicTransit> {
    pt : & 'pt PT,
    journeys_tree : JourneysTree<PT>,

    waiting_fronts : Vec<WaitingFront<PT>>,    // map a `stop` to a pareto front
    new_waiting_fronts : Vec<WaitingFront<PT>>,// map a `stop` to a pareto front
    stops_with_a_new_waiting : Vec<PT::Stop>,  // list of Stops

    route_has_a_new_waiting_path :  Vec<Option<PT::Stop>>, // map a route to an Option<Stop>
    routes_with_new_waiting_path : Vec<PT::Route>,         // list of Routes

    // map a `stop` to the pareto front of Pathes which
    // ends at `stop` with a Transit 
    debarked_fronts : Vec<DebarkedFront<PT>>,    // map a `stop` to a pareto front
    new_debarked_fronts : Vec<DebarkedFront<PT>>,// map a `stop` to a pareto front
    //TODO : can be replaced with new_debarked_fronts[stop].is_empty()
    stop_has_a_new_debarked   : Vec<bool>,       // map a  `stop` to a bool
    stops_with_a_new_debarked :  Vec<PT::Stop>,  // list of Stops

    onboard_front : OnboardFront<PT>,
    new_onboard_front : OnboardFront<PT>,

    arrived_front : ArrivedFront<PT>,

}


impl<'pt, PT : PublicTransit> MultiCriteriaRaptor<'pt, PT> {

    pub fn new(pt : &'pt PT ) -> Self {
        let nb_of_stops = pt.nb_of_stops();
        let nb_of_routes = pt.nb_of_routes();
        Self {
            pt,
            journeys_tree : JourneysTree::new(),

            waiting_fronts : vec![WaitingFront::<PT>::new(); nb_of_stops],
            new_waiting_fronts : vec![WaitingFront::<PT>::new(); nb_of_stops],
            stops_with_a_new_waiting : Vec::new(),

            route_has_a_new_waiting_path : vec![None; nb_of_routes],
            routes_with_new_waiting_path : Vec::new(),

            debarked_fronts : vec![DebarkedFront::<PT>::new(); nb_of_stops],
            new_debarked_fronts : vec![DebarkedFront::<PT>::new(); nb_of_stops],
            stop_has_a_new_debarked : vec![false; nb_of_stops],
            stops_with_a_new_debarked : Vec::new(),

            onboard_front : OnboardFront::<PT>::new(),
            new_onboard_front : OnboardFront::<PT>::new(),

            arrived_front : ArrivedFront::<PT>::new()
        }
    }

    pub fn compute(& mut self) {
        self.clear();

        self.init_with_departures();

        debug_assert!( ! self.routes_with_new_waiting_path.is_empty());

        while ! self.routes_with_new_waiting_path.is_empty() {
            self.save_new_waiting_fronts();

            self.ride_routes();

            self.save_new_debarked_fronts();

            self.perform_transfers_and_arrivals();
            
        }


    }

    fn clear(& mut self) {
        self.journeys_tree.clear();
        // TODO : check which maps/lists does indeed needs clearing after compute
        //   as some of them are cleared whithin compute()
        for front in & mut self.waiting_fronts {
            front.clear();
        }
        for front in & mut self.new_waiting_fronts {
            front.clear();
        }
        self.stops_with_a_new_waiting.clear();

        for has_stop in & mut self.route_has_a_new_waiting_path {
            *has_stop = None;
        }

        for front in & mut self.debarked_fronts {
            front.clear();
        }
        for front in & mut self.new_debarked_fronts {
            front.clear();
        }
        for has in & mut self.stop_has_a_new_debarked {
            *has = false;
        }
        self.stops_with_a_new_debarked.clear();

        self.onboard_front.clear();
        self.new_onboard_front.clear();

        self.arrived_front.clear();

    }

    fn init_with_departures(&mut self) {
        debug_assert!(self.journeys_tree.is_empty());
        debug_assert!(self.new_waiting_fronts.iter().all(|front| { front.is_empty()}));
        debug_assert!(self.stops_with_a_new_waiting.is_empty());
        debug_assert!(self.route_has_a_new_waiting_path.iter().all(|has| has.is_none()));
        //TODO : check that there is at least one departure

        for (stop, criteria) in self.pt.journey_departures() {
            let journey = self.journeys_tree.depart(&stop);
            let stop_id = self.pt.stop_id(&stop);
    
            let new_waiting_front = & mut self.new_waiting_fronts[stop_id];
            if new_waiting_front.is_empty() {
                self.stops_with_a_new_waiting.push(stop.clone());
            }
    
            new_waiting_front.add(journey, criteria, self.pt);
            
            let route = self.pt.route_of(&stop);
    
            let route_id = self.pt.route_id(&route);
            let has_a_new_waiting_path = & mut self.route_has_a_new_waiting_path[route_id];
            if let Some(old_waiting_stop) = has_a_new_waiting_path {
                if self.pt.is_upstream(&stop, old_waiting_stop) {
                    *old_waiting_stop = stop;
                }
            }
            else {
                * has_a_new_waiting_path = Some(stop.clone());
                self.routes_with_new_waiting_path.push(route);
            }
    
        }   
    }

    fn save_new_waiting_fronts(&mut self) {
        debug_assert!(!self.stops_with_a_new_waiting.is_empty());
        // TODO : check that new_waiting_fronts[stop] is empty for all
        //     stops not in stops_with_a_new_waiting

        for stop in self.stops_with_a_new_waiting.drain(..) {
            let stop_id = self.pt.stop_id(&stop);
            let waiting_front = & mut self.waiting_fronts[stop_id];
            let new_waiting_front = & self.new_waiting_fronts[stop_id];
            debug_assert!( ! new_waiting_front.is_empty() );
            for (waiting, criteria) in new_waiting_front.iter() {
                // we do not need to check, because 
                //  - `new_waiting_front` is a pareto front 
                //  - we added an element to `new_waiting_front` only if it was not dominated by `waiting_front`
                //  - we removed from `waiting_front` all elements that were dominated by an element of `new_waiting_front`
                //
                // TODO : add debug_assert here to check what is written above
                waiting_front.add_unchecked(waiting.clone(), criteria.clone());

            }
        }
    }

    fn ride_routes(& mut self) {
        debug_assert!( ! self.routes_with_new_waiting_path.is_empty() );
        debug_assert!(self.stops_with_a_new_debarked.is_empty());
        debug_assert!(self.stop_has_a_new_debarked.iter().all(|has| {! has}));
        debug_assert!(self.new_debarked_fronts.iter().all(|front| { front.is_empty() } ));

        for route in self.routes_with_new_waiting_path.drain(..) {
            let route_id = self.pt.route_id(&route);
            // we recover the stop at which we start riding the route
            //  and put at None in route_has_a_new_waiting_path[route_id]
            let mut has_stop = self.route_has_a_new_waiting_path[route_id].take();  

            self.onboard_front.clear();

            while let Some(stop) = has_stop {
                let stop_id = self.pt.stop_id(&stop);
                // update debarked front at this stop with elements from
                //   onboard front
                { 
                    let debarked_front = & mut self.debarked_fronts[stop_id];
                    let new_debarked_front = & mut self.new_debarked_fronts[stop_id];

                    for ((ref onboard, ref trip), ref onboard_criteria) in self.onboard_front.iter() {

                        let new_debarked_criteria = self.pt.debark(trip, &stop, onboard_criteria);
                        if debarked_front.dominates(&new_debarked_criteria, self.pt) {
                            continue;
                        }
                        if new_debarked_front.dominates(&new_debarked_criteria, self.pt) {
                            continue;
                        }
                        let new_debarked = self.journeys_tree.debark(onboard, &stop);
                        debarked_front.remove_elements_dominated_by( &new_debarked_criteria, self.pt);                         
                        new_debarked_front.add_and_remove_elements_dominated(new_debarked, new_debarked_criteria, self.pt);
                        if  ! self.stop_has_a_new_debarked[stop_id]{
                            self.stop_has_a_new_debarked[stop_id] = true;
                            self.stops_with_a_new_debarked.push(stop.clone());
                        }

                    }
                }


                // we update has_stop to the next stop on the route
                has_stop = self.pt.next_on_route(&stop);

                // if there is no next stop on the route
                // there is no need to the update onboard front
                if has_stop.is_none() {
                    continue;
                }

                // update onboard front with boardings from new waitings
                {
                    let new_waiting_front = & mut self.new_waiting_fronts[stop_id];
                    for (waiting, waiting_criteria) in new_waiting_front.drain() {
                        let (trip, new_onboard_criteria) = self.pt.board(&stop, &waiting_criteria);
                        if self.onboard_front.dominates(&new_onboard_criteria, self.pt) {
                            continue;
                        }
                        let new_onboard = self.journeys_tree.board(&waiting, &trip);
                        self.onboard_front.add_and_remove_elements_dominated((new_onboard, trip), new_onboard_criteria, self.pt);

                    }
                }

                // ride to the next stop point and update onboard
                //   pareto front along the way
                {
                    self.new_onboard_front.clear();
                    debug_assert!(self.new_onboard_front.is_empty());
                    for ((onboard, trip), criteria) in self.onboard_front.drain() {
                        let new_criteria = self.pt.ride(&trip, &stop, &criteria);
                        self.new_onboard_front.add((onboard, trip), new_criteria, self.pt);

                    }
                    self.onboard_front.replace_with(& mut self.new_onboard_front);
                    self.new_onboard_front.clear();
                }
            }
        }

    }

    fn save_new_debarked_fronts(&mut self) {
        debug_assert!(!self.stops_with_a_new_debarked.is_empty());
        // TODO : check that new_debarked_front[stop] is empty for all
        //     stops not in stops_with_a_new_debarked
        for stop in & self.stops_with_a_new_debarked {
            let stop_id = self.pt.stop_id(&stop);
            let debarked_front = & mut self.debarked_fronts[stop_id];
            let new_debarked_front = & self.new_debarked_fronts[stop_id];
            debug_assert!( ! new_debarked_front.is_empty() );
            for (debarked, criteria) in new_debarked_front.iter() {
                // we do not need to check, because 
                //  - new_debarked_front is a pareto front 
                //  - we added an element to new_debarked_front only if it was not dominated by debarked_front
                //  - we removed from debarked_front all elements that were dominated by an element of new_debarked_front
                //
                // TODO : add debug_assert here to check what is written above
                debarked_front.add_unchecked(debarked.clone(), criteria.clone());
            }
        }
    }

    fn perform_transfers_and_arrivals(&mut self) {
        debug_assert!(self.routes_with_new_waiting_path.is_empty());
        debug_assert!(self.route_has_a_new_waiting_path.iter().all(|has| has.is_none()));
        debug_assert!(self.new_waiting_fronts.iter().all(|front| front.is_empty()));
        debug_assert!(self.stops_with_a_new_waiting.is_empty());
        for stop in self.stops_with_a_new_debarked.drain(..) {
            let stop_id = self.pt.stop_id(&stop);
            let new_debarked_front = & mut self.new_debarked_fronts[stop_id];
            debug_assert!( ! new_debarked_front.is_empty() );
            for (debarked, criteria) in new_debarked_front.drain() {
                // we perform arrival from the `debarked` path
                if let Some(arrived_criteria) = self.pt.journey_arrival(&stop, &criteria) {
                    let arrived = self.journeys_tree.arrive(&debarked);
                    self.arrived_front.add(arrived, arrived_criteria, self.pt);
                }
                // we perform all transfers from the `debarked` path
                for (arrival_stop, arrival_criteria) in self.pt.transfers(&stop, &criteria) {
                    let arrival_id = self.pt.stop_id(&arrival_stop);
                    let waiting_front = & mut self.waiting_fronts[arrival_id];
                    let new_waiting_front = & mut self.new_waiting_fronts[arrival_id];
                    if waiting_front.dominates(&arrival_criteria, self.pt) {
                        continue;
                    }
                    if new_waiting_front.dominates(&arrival_criteria, self.pt) {
                        continue;
                    }

                    if new_waiting_front.is_empty() {
                        self.stops_with_a_new_waiting.push(arrival_stop.clone());
                    }

                    let waiting = self.journeys_tree.transfer(&debarked, &arrival_stop);
                    waiting_front.remove_elements_dominated_by( &arrival_criteria, self.pt);
                    new_waiting_front.add_and_remove_elements_dominated(waiting, arrival_criteria, self.pt);



                    let route = self.pt.route_of(&arrival_stop);
                    let route_id = self.pt.route_id(&route);
                    if let Some(old_waiting_stop) = &self.route_has_a_new_waiting_path[route_id] {
                        if self.pt.is_upstream(&arrival_stop, old_waiting_stop) {
                            self.route_has_a_new_waiting_path[route_id] = Some(arrival_stop.clone());
                        }
                    }
                    else {
                        self.route_has_a_new_waiting_path[route_id] = Some(arrival_stop.clone());
                        self.routes_with_new_waiting_path.push(route);
                    }
                    
                }
            }
        }
    }
}
