use std::hash::Hash;
pub trait PublicTransit {
    /// A location where a vehicle can be boarded into or debarked from
    type Stop: Clone;

    /// A `Mission` is an ordered sequence of `Position`
    type Mission: Clone + Hash + Eq;

    /// Identify a point along a `Mission`
    type Position: Clone;

    /// A trip of a vehicle along a `Mission`
    type Trip: Clone;

    /// Identify a foot transfer between two `Stop`s
    type Transfer: Clone;

    /// Identify a possible departure of a journey
    type Departure: Clone;

    /// Identify a possible arrival of a journey
    type Arrival: Clone;

    /// Stores data used to determine if a journey is better than another
    type Criteria: Clone;

    /// Returns `true` if `upstream` is positionned strictly before `downstream`
    /// in `mission`.
    ///
    /// Panics if `upstream` or `downstream` does not belongs to `mission`.
    fn is_upstream(
        &self,
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool;

    /// Returns `Some(next_position)` if `next_position` is after `position` on `mission`.
    ///
    /// Returns `None` if `position` is the last on `mission`.
    ///
    /// Panics if `position` does not belongs to `mission`.
    fn next_on_mission(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position>;

    /// Returns the `Mission` that `trip` belongs to.
    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission;

    /// Returns the `Stop` at `position` in `mission`
    ///
    /// Panics if `position` does not belongs to `mission`
    fn stop_of(&self, position: &Self::Position, mission: &Self::Mission) -> Self::Stop;

    /// Returns `true` if `lower` is better or equivalent to `upper`
    fn is_lower(&self, lower: &Self::Criteria, upper: &Self::Criteria) -> bool;

    /// Returns `false` when `criteria` corresponds to an invalid journey.
    ///
    /// For example if we want to have at most 5 transfer, and `criteria` have 6 transfers
    ///  then `is_valid(criteria)` should returns false.
    ///
    /// Similarly, if we want our journey to arrive at most 24h after the given departure time
    ///  and `criteria` have an arrival time more than 24h after, then `is_valid(criteria)` should returns false.
    ///
    /// The more `criteria` you can eliminate in this way, the better the engine will perform.
    fn is_valid(&self, criteria: &Self::Criteria) -> bool;

    /// Returns `Some(arrival_criteria)` if `trip` can be boarded
    /// when being at `position` with `waiting_criteria`.
    /// In this case, `arrival_criteria` is the criteria obtained by :
    ///  - boarding `trip` at `position` when waiting with `waiting_criteria`.
    ///  - ride `trip` until arrival at the next stop
    ///
    /// Returns None if `trip` cannot be boarded when being at `position` with `waiting_criteria`
    ///
    /// Panics if `position` is the last on the `mission_of_(trip)`.
    ///
    /// Panics if `position` does not belongs to `mission_of_(trip)`.
    fn board_and_ride(
        &self,
        position: &Self::Position,
        trip: &Self::Trip,
        waiting_criteria: &Self::Criteria,
    ) -> Option<Self::Criteria>;

    /// Returns `Some((best_trip, best_crit))` where `best_trip` is
    /// the "best" `Trip` of `mission` that can be be boarded while
    /// being at `position` with `waiting_criteria`, and
    ///
    /// `best_crit = board_and_ride(position, best_trip, waiting_criteria)`.
    ///
    /// Here "best" means the following.
    ///
    /// Let `position_1, ..., position_n` be the sequence of positions after `position` on the `mission_of(trip)`, i.e. :
    ///
    ///  - `Some(position_1) = next_on_mission(position, mission_of_(trip))`
    ///  - `Some(position_2) = next_on_mission(position_1, mission_of_(trip))`
    ///  -  ...
    ///  - `Some(position_n) = next_on_mission(position_{n-1}, mission_of_(trip))`
    ///  - `None = next_on_mission(position_n, mission_of_(trip))`
    ///
    /// Let `best_crit_2, ..., best_crit_n` be the sequence of criteria obtained by boarding and riding `best_trip`, i.e. :
    ///
    ///  - `best_crit_2 = ride(best_trip, position_1, best_crit)`
    ///  - `best_crit_3 = ride(best_trip, position_2, best_crit_2)`
    ///  - ...
    ///  - `best_crit_n =  ride(best_trip, position_{n-1}, best_crit_{n-1})`
    ///
    /// Consider any `trip` in `trips_of(mission)` that can be boarded, i.e. such that :
    ///
    ///  `Some(crit) = board_and_ride(position, trip, waiting_criteria)`
    ///
    /// And let `crit_2, ..., crit_n` be the sequence of criteria obtained by boarding and riding `best_trip`, i.e. :
    ///
    ///  - `crit_2 = ride(trip, position_1, crit)`
    ///  - `crit_3 = ride(trip, position_2, crit_2)`
    ///  - ...
    ///  - `crit_n =  ride(trip, position_{n-1}, crit_{n-1})`
    ///
    /// Then we must have :
    ///
    ///  - `true = is_lower(best_crit, crit)`
    ///  - `true = is_lower(best_crit_2, crit_2)`
    ///  - ...
    ///  - `true = is_lower(best_crit_n, crit_n)`
    ///
    ///
    /// For example, consider the arrival time as a criteria. Then, the above properties means that `best_trip`
    /// arrives earlier than any other trip at ALL subsequent positions.
    ///
    /// Returns None if `mission` cannot be boarded at `position` with `waiting_criteria`.
    ///
    /// Panics if `position` does not belongs to `mission`.
    fn best_trip_to_board(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
        waiting_criteria: &Self::Criteria,
    ) -> Option<(Self::Trip, Self::Criteria)>;

    /// Returns `Some(debarked_criteria)`,
    /// where `derbarked_criteria` is the criteria obtained by debarking from `trip` at `position`
    /// when being onboard at the arrival at `position` with `onboard_criteria`.
    ///
    /// Returns None if a passenger cannot debark from `trip` at `position` with `onboard_criteria`.
    ///
    /// Panics if `position` does not belong to `mission_of_(trip)`.
    fn debark(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
        onboard_criteria: &Self::Criteria,
    ) -> Option<Self::Criteria>;

    /// Returns the `new_criteria` obtained when riding along `trip`
    /// to the arrival to next position of `mission_of(trip)`, when being onboard at
    /// the arrival of `trip` at `position` with `criteria`.
    ///
    /// Panics if `position` does not belongs to `mission_of(trip)`
    ///
    /// Panics if `position` is the last position of `mission_of(trip)`
    fn ride(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
        criteria: &Self::Criteria,
    ) -> Self::Criteria;

    /// Performs `transfer` when being at `from_stop` with `criteria`
    /// and returns the arrival `Stop` along with the `Criteria`
    /// obtained after performing the transfer.
    ///
    /// Panics if `transfer` cannot be performed from `from_stop`.
    fn transfer(
        &self,
        from_stop: &Self::Stop,
        transfer: &Self::Transfer,
        criteria: &Self::Criteria,
    ) -> (Self::Stop, Self::Criteria);

    /// Returns the `Stop` at which this departure occurs
    /// along with the initial `Criteria`
    fn depart(&self, departure: &Self::Departure) -> (Self::Stop, Self::Criteria);

    /// The stop at which this arrival can be made
    fn arrival_stop(&self, arrival: &Self::Arrival) -> Self::Stop;

    /// Returns the criteria obtained after performing `arrival`
    /// while being at `arrival_stop(arrival)` with `criteria`.
    fn arrive(&self, arrival: &Self::Arrival, criteria: &Self::Criteria) -> Self::Criteria;

    //TODO : document monotonicity on board, debark, ride, tranfer, arrival

    /// An upper bound on the total number of `Stop`s.
    fn nb_of_stops(&self) -> usize;
    
    /// Returns an usize between 0 and nb_of_stops().
    ///
    /// Returns a different value for two different `stop`s.
    fn stop_id(&self, stop: &Self::Stop) -> usize;

    // // An upper bound on the total number of `Mission`s
    // fn nb_of_missions(&self) -> usize;
    // // Returns an usize between 0 and nb_of_misions()
    // // Returns a different value for two different `mission`s
    // fn mission_id(&self, mission : & Self::Mission) -> usize;
}

pub trait PublicTransitIters<'a>: PublicTransit {

    /// Iterator for the `Mission`s that can be boarded at a `stop`
    /// along with the `Position` of `stop` on each `Mission`
    type MissionsAtStop: Iterator<Item = (Self::Mission, Self::Position)>;
    /// Returns all the `Mission`s that can be boarded at `stop`.
    ///
    /// Should not return twice the same `Mission`.
    fn boardable_missions_at(&'a self, stop: &Self::Stop) -> Self::MissionsAtStop;

    /// Iterator for all possible departures of a journey
    type Departures: Iterator<Item = Self::Departure>;
    /// Returns the identifiers of all possible departures of a journey
    fn departures(&'a self) -> Self::Departures;

    /// Iterator for all `Transfer`s that can be taken at a `Stop`
    type TransfersAtStop: Iterator<Item = Self::Transfer>;
    /// Returns all `Transfer`s that can be taken at `from_stop`
    ///
    /// Should not return twice the same `Transfer`.
    fn transfers_at(&'a self, from_stop: &Self::Stop) -> Self::TransfersAtStop;

    /// Iterator for all `Trip`s belonging to a `Mission`.
    type TripsOfMission: Iterator<Item = Self::Trip>;
    /// Returns all `Trip`s belonging to `mission`
    fn trips_of(&'a self, mission: &Self::Mission) -> Self::TripsOfMission;

    /// Iterator for all possible arrivals of a journey
    type Arrivals: Iterator<Item = Self::Arrival>;
    /// Returns the identifiers of all possible arrivals of a journey
    fn arrivals(&'a self) -> Self::Arrivals;
}

pub struct DepartureLeg<PT: PublicTransit> {
    pub departure: PT::Departure,
    pub trip: PT::Trip,
    pub board_position: PT::Position,
    pub debark_position: PT::Position,
}

pub struct ConnectionLeg<PT: PublicTransit> {
    pub transfer: PT::Transfer,
    pub trip: PT::Trip,
    pub board_position: PT::Position,
    pub debark_position: PT::Position,
}

pub struct Journey<PT: PublicTransit> {
    pub departure_leg: DepartureLeg<PT>,
    pub connection_legs: Vec<ConnectionLeg<PT>>,
    pub arrival: PT::Arrival,
    pub criteria_at_arrival: PT::Criteria,
}
