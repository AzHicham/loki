
pub trait PublicTransit {

    // A `Route` is a sequence of `Stop`s
    type Route : Clone;
    // A stop in a route, along with its position in this Route
    type Stop : Clone;

    // A trip of a vehicle along a `Route`
    type Trip : Clone;


    type Criteria : Clone;

    fn is_lower(&self, lower : & Self::Criteria, upper : & Self::Criteria) -> bool;



    // Returns `true` if `upstream` is positionned strictly before `downstream` 
    //    in their common `Route`
    // Panics if `upstream` and `downstream` are not in the same `Route`
    fn is_upstream(&self,  upstream : & Self::Stop, downstream : & Self::Stop) -> bool;


    fn board(&self,
             stop : & Self::Stop, 
             criteria : & Self::Criteria
            ) -> (Self::Trip, Self::Criteria);

    //Panics if `stop` does not belong to the same `Route` as `trip`
    fn debark(&self,
              trip : & Self::Trip,
              stop : & Self::Stop, 
              criteria : & Self::Criteria
             ) ->  Self::Criteria;


    //Returns `None` if `stop` is the last on its `Route`
    fn next_on_route(&self, stop : & Self::Stop) -> Option<Self::Stop>;

    // Panics if `stop` does not belongs to the same `Route` as `trip`
    //        of if `stop` is the last of its `Route`
    fn ride(&self,
            trip : & Self::Trip, 
            stop : & Self::Stop,
            criteria : & Self::Criteria
            ) -> Self::Criteria;

    fn route_of(&self, stop : & Self::Stop) -> Self::Route;

    type Transfers : Iterator<Item = (Self::Stop, Self::Criteria)>;
    fn transfers(&self ,
                  departure : & Self::Stop, 
                  start : & Self::Criteria
                ) -> Self::Transfers;



    type JourneyDepartures : Iterator<Item = (Self::Stop, Self::Criteria)>;
    fn journey_departures(&self) -> Self::JourneyDepartures;

    fn journey_arrival(&self, 
                        stop : & Self::Stop, 
                        criteria : & Self::Criteria
                      ) -> Option<Self::Criteria>; //Returns None if destination is not reachable from `stop`

    fn nb_of_stops(&self) -> usize;
    //returns an usize between 0 and nb_of_stops()
    fn stop_id(&self, stop : & Self::Stop) -> usize;

    fn nb_of_routes(&self) -> usize;
    fn route_id(&self, route : & Self::Route) -> usize;
    
}


