pub trait PublicTransit {
    // A point where a vehicle can be boarded into or debarked from
    type Stop : Clone;

    // A `Mission` is an ordered sequence of pairwise distinct `Stop`s
    type Mission : Clone;

    // A trip of a vehicle along a `Mission`
    type Trip : Clone;

    type Departure : Clone;

    type Transfer : Clone;

    type Criteria : Clone;

    // Returns `true` if `upstream` is positionned strictly before `downstream` 
    //    in `mission`
    // Panics if `upstream` or `downstream` does not belongs to `mission`
    fn is_upstream(&self,
        upstream : & Self::Stop, 
        downstream : & Self::Stop, 
        mission : & Self::Mission,
    ) -> bool;

    // Returns Some(next_stop) if `next_stop` is after `stop` on `mission`
    // Returns None if `stop` is the last on `mission`
    // Panics if `stop` does not belongs to `mission`
    fn next_on_mission(&self,
        stop : & Self::Stop,
        mission : & Self::Mission
    ) -> Option<Self::Stop>;


    // Returns the `Mission` that `trip` belongs to.
    fn mission_of(&self,
        trip : & Self::Trip
    ) -> Self::Mission;



    // Returns `true` if `lower` is lower or equal to `upper`
    fn is_lower(&self, 
        lower : & Self::Criteria, 
        upper : & Self::Criteria
    ) -> bool;


    // Returns Some(arrival_criteria) when if `trip` can be boarded 
    //   when being at `stop` with `waiting_criteria`.
    //   In this case, `arrival_criteria` is the criteria obtained by :
    //      - boarding `trip` at `stop` when waiting with 
    //      - ride `trip` until arrival at the next stop 
    // Returns None if `trip` cannot be boarded when being at `stop` with `waiting_criteria`
    // Panics if `stop` is the last on the `Mission` of `trip`
    // Panics if `trip` does not belongs to `boardable_missions_of_(stop)`
    fn board_and_ride(&self,
        stop : & Self::Stop,
        trip : & Self::Trip,
        waiting_criteria : & Self::Criteria
    ) -> Option<Self::Criteria>;



    // Returns Some((best_trip, best_crit) where `best_trip` is 
    // the "best" `Trip` of `mission` that can be be boarded while
    // being at `stop` with `waiting_criteria`, and
    // `best_crit = board_and_ride(stop, best_trip, waiting_criteria)`
    // Here "best" means that  for all `trip` in `trips_of(mission)` we have either :
    //       - `board_and_ride(stop, trip, waiting_criteria) == None`
    //       - `board_and_ride(stop, trip, waiting_criteria) == Some(crit)` and 
    //            `is_lower(best_crit, crit) == true`
    // Returns None if `mission` cannot be boarded at `stop` with `waiting_criteria`.
    // Panics if `stop` does not belongs to `mission` 
    fn best_trip_to_board(&self,
        stop : & Self::Stop, 
        mission : & Self::Mission,
        waiting_criteria : & Self::Criteria
    ) -> Option<(Self::Trip, Self::Criteria)>;

    // Returns `debarked_criteria`,
    //   where `derbarked_criteria` is the criteria obtained by debarking from `trip` at `stop`
    //   when being onboard with `onboard_criteria`
    // Panics if `stop` does not belong to the `Mission` of `trip`
    fn debark(&self,
        trip : & Self::Trip,
        stop : & Self::Stop, 
        onboard_criteria : & Self::Criteria
    ) ->  Self::Criteria;


    // Returns the `new_criteria` obtained when riding along `trip`
    // to the arrival to next stop of its `Mission`, when being onboard at 
    // the arrival of `trip` at `stop` with `criteria`. 

    // Panics if `stop` does not belongs to the `Mission` of `trip`
    fn ride(&self,
        trip : & Self::Trip,
        stop : & Self::Stop,
        criteria : & Self::Criteria
    ) -> Self::Criteria;

    // Performs `transfer` when being at `from_stop` with `criteria`
    // and returns the arrival `Stop` along with the `Criteria`
    // obtained after performing the transfer.
    // Panics if `transfer` cannot be performed from `from_stop`
    fn transfer(&self,
        from_stop : & Self::Stop,
        transfer : & Self::Transfer,
        criteria : & Self::Criteria,
    ) ->  (Self::Stop, Self::Criteria);

    // Returns the `Stop` at which this departure occurs
    // along with the initial `Criteria` 
    fn depart(&self, departure : & Self::Departure) -> (Self::Stop, Self::Criteria);

    // Returns None if destination is not reachable from `stop`
    fn journey_arrival(&self,
        stop : & Self::Stop,
        criteria : & Self::Criteria
    ) -> Option<Self::Criteria>;

    //TODO : document monotonicity on board, debark, ride, tranfer, arrival

    // An upper bound on the total number of `Stop`s
    fn nb_of_stops(&self) -> usize;
    // Returns an usize between 0 and nb_of_stops()
    // Returns a different value for two different `stop`s
    fn stop_id(&self, stop : & Self::Stop) -> usize;

    // An upper bound on the total number of `Mission`s
    fn nb_of_missions(&self) -> usize;
    // Returns an usize between 0 and nb_of_misions()
    // Returns a different value for two different `mission`s
    fn mission_id(&self, mission : & Self::Mission) -> usize;

}

pub trait PublicTransitIters<'a> : PublicTransit {

    // Returns all the `Mission`s that can be boarded at `stop`
    // Should not return twice the same `Mission`.
    type MissionsAtStop : Iterator<Item = Self::Mission>;
    fn boardable_missions_at(& 'a self,
        stop : & Self::Stop
    ) -> Self::MissionsAtStop;


    type Departures : Iterator<Item = Self::Departure>;
    fn departures(& 'a self) -> Self::Departures;

    // Returns the set of `Transfer` that can be taken at `from_stop`
    // Should not return twice the same `Transfer`.
    type TransfersAtStop : Iterator<Item = Self::Transfer>;
    fn transfers_at(& 'a self, from_stop : & Self::Stop) -> Self::TransfersAtStop;



    // Returns all `Trip`s belonging to `mission`
    type TripsOfMission : Iterator<Item = Self::Trip>;
    fn trips_of(&'a self,
        mission : & Self::Mission
    ) -> Self::TripsOfMission;

}
