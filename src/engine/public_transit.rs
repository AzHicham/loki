pub trait PublicTransit {
    // A point where a vehicle can be boarded into or debarked from
    type Stop : Clone;

    // A `Mission` is an ordered sequence of pairwise distinct `Stop`s
    type Mission : Clone;

    // A trip of a vehicle along a `Mission`
    type Trip : Clone;

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


    // Returns all the `Mission`s that can be boarded at `stop`
    // Should not return twice the same `Mission`.
    type Missions : Iterator<Item = Self::Mission>;
    fn boardable_missions_of(&self,
        stop : & Self::Stop
    ) -> Self::Missions;

    // Returns the `Mission` that `trip` belongs to.
    fn mission_of(&self,
        trip : & Self::Trip
    ) -> Self::Mission;

    // Returns `true` if `lower` is lower or equal to `upper`
    fn is_lower(&self, 
        lower : & Self::Criteria, 
        upper : & Self::Criteria
    ) -> bool;


    // Returns a pareto front of `Trip`s belonging to `mission` that can be boarded at `stop`
    //   when waiting with `waiting_criteria`.
    //  More precisely, it returns [(trip_1, crit_1), ..., (trip_n, crit_n)] such that :
    //   - trip_i and trip_j are distinct for all distinct i,j in [1, ..., n]
    //   - crit_i is the criteria obtained from boarding trip_i when waiting with `waiting_criteria`
    //   - crit_i and crit_j are not comparable for all distinct i,j in [1, ..., n], i.e.
    //       is_lower(self, crit_i, crit_j) == is_lower(self, crit_j, crit_i) == false;
    // The returned `BoardFront` is empty when `mission` cannot be boarded at `stop`
    // Panics if `stop` does not belongs to `mission` 
    type BoardFront : Iterator<Item = (Self::Trip, Self::Criteria)>;
    fn board(&self, 
        stop : & Self::Stop, 
        mission : & Self::Mission,
        waiting_criteria : & Self::Criteria
    ) -> Self::BoardFront;

    // Returns Some(debarked_criteria) if the `Mission` of `trip` allows debarkment at `stop`,
    //   where `derbarked_criteria` is the criteria obtained by debarking from `trip` at `stop`
    //   when being onboard with `onboard_criteria`
    // Returns None if the `Mission` of `trip` does not allows debarkment at `stop`
    // Panics if `stop` does not belong to the `Mission` of `trip`
    fn debark(&self,
        trip : & Self::Trip,
        stop : & Self::Stop, 
        onboard_criteria : & Self::Criteria
    ) ->  Option<Self::Criteria>;


    // Returns the `new_criteria` obtained when riding along `trip`
    // to the next stop of its `Mission`, when being onboard at `stop` with `criteria`. 
    // Panics if `stop` is the last on the `Mission` of `trip`
    // Panics if `stop` does not belongs to the `Mission` of `trip`
    fn ride(&self,
        trip : & Self::Trip,
        stop : & Self::Stop,
        criteria : & Self::Criteria
    ) -> Self::Criteria;

    // Returns the arrival `Stop`s that can be reached from `from_stop`,
    //  along with the criteria obtained at each arrival if the transfer
    //  begins at `from_stop` with `criteria`.
    // Should not return twice the same stop.
    type Transfers : Iterator<Item = (Self::Stop, Self::Criteria)>;
    fn transfers(&self,
        from_stop : & Self::Stop,
        criteria : & Self::Criteria,
    ) ->  Self::Transfers;

    // Returns the stops from which a beginning of a journey is allowed
    //  along with the starting criteria for a journey beginning at each of
    //  these stops.
    // Should return at least one stop.
    // Should not return twice the same stop.
    type JourneyDepartures : Iterator<Item = (Self::Stop, Self::Criteria)>;
    fn journey_departures(&self) -> Self::JourneyDepartures;

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
