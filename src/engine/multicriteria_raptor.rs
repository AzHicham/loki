use crate::engine::public_transit::{PublicTransit, PublicTransitIters};
use crate::engine::journeys_tree::{JourneysTree};
use crate::engine::pareto_front::{OnboardFront, DebarkedFront, WaitingFront, ArrivedFront};
use std::collections::HashMap;


pub struct MultiCriteriaRaptor<'pt, PT : PublicTransit> {
    pt : & 'pt PT,
    journeys_tree : JourneysTree<PT>,

    waiting_fronts : Vec<WaitingFront<PT>>,    // map a `stop` to a pareto front
    new_waiting_fronts : Vec<WaitingFront<PT>>,// map a `stop` to a pareto front
    stops_with_new_waiting : Vec<PT::Stop>,  // list of Stops

    missions_with_new_waiting : HashMap::<PT::Mission, PT::Position>, // list of Missions

    // map a `stop` to the pareto front of Pathes which
    // ends at `stop` with a Transit 
    debarked_fronts : Vec<DebarkedFront<PT>>,    // map a `stop` to a pareto front
    new_debarked_fronts : Vec<DebarkedFront<PT>>,// map a `stop` to a pareto front
    stops_with_new_debarked :  Vec<PT::Stop>,  // list of Stops

    onboard_front : OnboardFront<PT>,
    new_onboard_front : OnboardFront<PT>,

    arrived_front : ArrivedFront<PT>,

}


impl<'pt, PT : PublicTransit + PublicTransitIters<'pt> > MultiCriteriaRaptor<'pt, PT> {

    pub fn new(pt : &'pt PT ) -> Self {
        let nb_of_stops = pt.nb_of_stops();
        Self {
            pt,
            journeys_tree : JourneysTree::new(),

            waiting_fronts : vec![WaitingFront::<PT>::new(); nb_of_stops],
            new_waiting_fronts : vec![WaitingFront::<PT>::new(); nb_of_stops],
            stops_with_new_waiting : Vec::new(),

            missions_with_new_waiting : HashMap::new(),

            debarked_fronts : vec![DebarkedFront::<PT>::new(); nb_of_stops],
            new_debarked_fronts : vec![DebarkedFront::<PT>::new(); nb_of_stops],
            stops_with_new_debarked : Vec::new(),

            onboard_front : OnboardFront::<PT>::new(),
            new_onboard_front : OnboardFront::<PT>::new(),

            arrived_front : ArrivedFront::<PT>::new()
        }
    }

    pub fn compute(& mut self) {
        self.clear();

        self.init_with_departures();

        self.identify_missions_with_new_waitings();

        debug_assert!( ! self.missions_with_new_waiting.is_empty());

        while ! self.missions_with_new_waiting.is_empty() {
            self.save_and_clear_new_debarked();

            self.ride();

            self.save_and_clear_new_waitings();

            self.perform_transfers_and_arrivals();

            self.identify_missions_with_new_waitings();
            
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
        self.stops_with_new_waiting.clear();

        self.missions_with_new_waiting.clear();

        for front in & mut self.debarked_fronts {
            front.clear();
        }
        for front in & mut self.new_debarked_fronts {
            front.clear();
        }
        self.stops_with_new_debarked.clear();

        self.onboard_front.clear();
        self.new_onboard_front.clear();

        self.arrived_front.clear();

    }

    // fill new_waiting_fronts with journeys departures
    fn init_with_departures(&mut self) {
        debug_assert!(self.journeys_tree.is_empty());
        debug_assert!(self.new_waiting_fronts.iter().all(|front| { front.is_empty()}));
        debug_assert!(self.stops_with_new_waiting.is_empty());

        // TODO : check that there is at least one departure
        // TODO : check that all departure stops are distincts
        for departure in self.pt.departures() {
            let (stop, criteria) = self.pt.depart(&departure);
            let journey = self.journeys_tree.depart(&departure);
            let stop_id = self.pt.stop_id(&stop);
    
            let new_waiting_front = & mut self.new_waiting_fronts[stop_id];
            if new_waiting_front.is_empty() {
                self.stops_with_new_waiting.push(stop.clone());
            }
    
            new_waiting_front.add(journey, criteria, self.pt);
            
     
        }   
    }

    // copy `new_waiting_fronts` in `waiting_fronts`
    // - update `waiting_fronts`
    // - reads `stops_with_new_waiting` and `new_waiting_fronts`
    fn save_new_waiting_fronts(&mut self) {
        debug_assert!(!self.stops_with_new_waiting.is_empty());
        // TODO : check that new_waiting_fronts[stop] is empty for all
        //     stops not in stops_with_new_waiting

        for stop in self.stops_with_new_waiting.iter() {
            let stop_id = self.pt.stop_id(stop);
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

    // identify missions that can be boarded from the new waiting pathes
    // - fill `mission_has_new_waiting` and `missions_with_new_waiting`
    // - reads `stops_with_new_waiting`
    fn identify_missions_with_new_waitings(& mut self) {
        debug_assert!(!self.stops_with_new_waiting.is_empty());
        debug_assert!(self.missions_with_new_waiting.is_empty());

        for stop in self.stops_with_new_waiting.iter() {
            // TODO : check that the same mission is not returned twice
            for (mission, position) in self.pt.boardable_missions_at(&stop) {

                let current_mission_has_new_waiting = self.missions_with_new_waiting.entry(mission.clone());
                use std::collections::hash_map::Entry;
                match current_mission_has_new_waiting {
                    Entry::Vacant(entry) => {
                        entry.insert(position);
                    },
                    Entry::Occupied(mut entry) => {
                        let saved_position = entry.get_mut();
                        if self.pt.is_upstream(&position, saved_position, &mission) {
                            * saved_position = position;
                        }
                    }
                }
            }  
        }
    }

    // ride all `missions_with_new_waiting`, boarding all new waiting pathes,
    // propagating theses new pathes, and perform debarkments along the way
    // - update `new_debarked_fronts` and `stops_with_new_debarked`
    // - uses `onboard_front` and `new_onboard_front` as local buffers
    // - reads `missions_with_new_waiting`, `mission_has_new_waiting`, 
    //         `new_waiting_fronts`, `debarked_fronts` 
    fn ride(& mut self) {
        debug_assert!( ! self.missions_with_new_waiting.is_empty() );
        debug_assert!(self.stops_with_new_debarked.is_empty());
        debug_assert!(self.new_debarked_fronts.iter().all(|front| { front.is_empty() } ));

        for (mission, first_position) in self.missions_with_new_waiting.iter() {

            let mut has_position = Some(first_position.clone());

            self.onboard_front.clear();

            while let Some(position) = has_position {
                let stop = self.pt.stop_of(&position, mission);
                let stop_id = self.pt.stop_id(&stop);
                // update debarked front at this stop with elements from
                //   onboard front
                { 
                    let debarked_front = & mut self.debarked_fronts[stop_id];
                    let new_debarked_front = & mut self.new_debarked_fronts[stop_id];
                    let current_stop_has_new_debarked = ! new_debarked_front.is_empty();

                    for ((ref onboard, ref trip), ref onboard_criteria) in self.onboard_front.iter() {

                        let new_debarked_criteria = self.pt.debark(trip, &position, onboard_criteria);

                        if debarked_front.dominates(&new_debarked_criteria, self.pt) {
                            continue;
                        }
                        if new_debarked_front.dominates(&new_debarked_criteria, self.pt) {
                            continue;
                        }
                        let new_debarked = self.journeys_tree.debark(onboard, &stop);
                        debarked_front.remove_elements_dominated_by( &new_debarked_criteria, self.pt);                         
                        new_debarked_front.add_and_remove_elements_dominated(new_debarked, new_debarked_criteria, self.pt);
                        if  ! current_stop_has_new_debarked {
                            self.stops_with_new_debarked.push(stop.clone());
                        }
                        

                    }
                }


                // we update has_stop to the next stop on the route
                has_position = self.pt.next_on_mission(&position, mission);

                // if there is no next stop on the route
                // there is no need to the update onboard front
                if has_position.is_none() {
                    continue;
                }

                // board and ride new waitings and put them into new_onboard_front
                {
                    self.new_onboard_front.clear();
                    let new_waiting_front = & self.new_waiting_fronts[stop_id];
                    for (ref waiting, ref waiting_criteria) in new_waiting_front.iter() {
                        if let Some((trip, new_onboard_criteria)) = self.pt.best_trip_to_board(&position, &mission, &waiting_criteria) {                      
                            if self.new_onboard_front.dominates(&new_onboard_criteria, self.pt) {
                                continue;
                            }
                            let new_onboard = self.journeys_tree.board(&waiting, &trip);
                            self.new_onboard_front.add_and_remove_elements_dominated((new_onboard, trip), new_onboard_criteria, self.pt);
                        
                        }
                    }
                }

                // ride to the next stop point and update onboard
                //   pareto front along the way
                {
                    for ((onboard, trip), criteria) in self.onboard_front.iter() {
                        let new_criteria = self.pt.ride(&trip, &position, &criteria);
                        if self.new_onboard_front.dominates(&new_criteria, self.pt) {
                            continue;
                        }
                        self.new_onboard_front.add((onboard.clone(), trip.clone()), new_criteria, self.pt);

                    }
                    
                }
                self.onboard_front.replace_with(& mut self.new_onboard_front);
            }
        }

    }

    // tranfer `new_debarked_fronts` into `debarked_fronts`
    // - update `debarked_fronts` and clear `new_debarked_fronts`
    // - reads `stops_with_new_debarked` and `new_debarked_fronts`
    fn save_and_clear_new_debarked(&mut self) {
        debug_assert!(!self.stops_with_new_debarked.is_empty());
        // TODO : check that new_debarked_front[stop] is empty for all
        //     stops not in stops_with_new_debarked
        for stop in & self.stops_with_new_debarked {
            let stop_id = self.pt.stop_id(&stop);
            let debarked_front = & mut self.debarked_fronts[stop_id];
            let new_debarked_front = & mut self.new_debarked_fronts[stop_id];
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
            new_debarked_front.clear();
        }
        self.stops_with_new_debarked.clear();
    }

    // perform transfers and arrivals from newly debarked path 
    // - update `new_waiting_fronts` and `arrived_front`
    // - reads `stops_with_new_debarked`, `new_debarked_fronts`
    //         `waiting_fronts`, `new_waiting_fronts`
    fn perform_transfers_and_arrivals(&mut self) {
        debug_assert!(self.new_waiting_fronts.iter().all(|front| front.is_empty()));
        debug_assert!(self.stops_with_new_waiting.is_empty());
        for stop in self.stops_with_new_debarked.iter() {
            let stop_id = self.pt.stop_id(stop);
            let new_debarked_front = & mut self.new_debarked_fronts[stop_id];
            debug_assert!( ! new_debarked_front.is_empty() );
            for (debarked, criteria) in new_debarked_front.iter() {
                // we perform arrival from the `debarked` path
                if let Some(arrived_criteria) = self.pt.journey_arrival(stop, &criteria) {
                    let arrived = self.journeys_tree.arrive(&debarked);
                    self.arrived_front.add(arrived, arrived_criteria, self.pt);
                }
                // we perform all transfers from the `debarked` path
                for transfer in self.pt.transfers_at(&stop) {
                    let (arrival_stop, arrival_criteria) = self.pt.transfer(&stop, &transfer, &criteria);
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
                        self.stops_with_new_waiting.push(arrival_stop.clone());
                    }

                    let waiting = self.journeys_tree.transfer(&debarked, &transfer);
                    waiting_front.remove_elements_dominated_by( &arrival_criteria, self.pt);
                    new_waiting_front.add_and_remove_elements_dominated(waiting, arrival_criteria, self.pt);
                    
                }
            }
        }
    }

    // tranfer `new_waiting_fronts` into `waiting_fronts`
    // - update `waiting_fronts` and clear `new_waiting_fronts`
    // - reads `stops_with_new_waiting` and `new_waiting_fronts`
    fn save_and_clear_new_waitings(&mut self) {
        debug_assert!(!self.stops_with_new_waiting.is_empty());
        // TODO : check that new_waiting_fronts[stop] is empty for all
        //     stops not in stops_with_new_waiting

        for stop in self.stops_with_new_waiting.iter() {
            let stop_id = self.pt.stop_id(stop);
            let waiting_front = & mut self.waiting_fronts[stop_id];
            let new_waiting_front = & mut self.new_waiting_fronts[stop_id];
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
            new_waiting_front.clear();
        }
        self.stops_with_new_waiting.clear();

        self.missions_with_new_waiting.clear();
    }
}
