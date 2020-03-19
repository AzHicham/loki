
pub trait PublicTransit {

    // A route is a sequence of stop points
    type Route : Clone;
    // A stop point in a route, along with its position in this route
    type RouteStop : Clone;

    // A trip of a vehicle along a route
    type Trip : Clone;


    type Criteria : Clone;

    fn is_lower(&self, lower : & Self::Criteria, upper : & Self::Criteria) -> bool;



    // returns true if upstream is positionned strictly before downstream in their common Route
    // panics if upstream and downstream are not in the same Route
    fn is_upstream(&self,  upstream : & Self::RouteStop, downstream : & Self::RouteStop) -> bool;


    fn board(&self,
             route_stop : & Self::RouteStop, 
             criteria : & Self::Criteria
            ) -> (Self::Trip, Self::Criteria);

    //panics if route_stop does not belong to the same route as trip
    fn debark(&self,
              trip : & Self::Trip,
              route_stop : & Self::RouteStop, 
              criteria : & Self::Criteria
             ) ->  Self::Criteria;


    fn next_on_route(&self, route_stop : & Self::RouteStop) -> Option<Self::RouteStop>;

    // panics if route_stop does not belongs to the same route as trip
    //        of if route_stop is the last stop of this route
    fn ride(&self,
            trip : & Self::Trip, 
            route_stop : & Self::RouteStop,
            criteria : & Self::Criteria
            ) -> Self::Criteria;

    fn route_of(&self, route_stop : & Self::RouteStop) -> Self::Route;

    type Transfers : Iterator<Item = (Self::RouteStop, Self::Criteria)>;
    fn transfers(&self ,
                  departure : & Self::RouteStop, 
                  start : & Self::Criteria
                ) -> Self::Transfers;



    type JourneyDepartures : Iterator<Item = (Self::RouteStop, Self::Criteria)>;
    fn journey_departures(&self) -> Self::JourneyDepartures;
    fn journey_arrival(route_stop : & Self::RouteStop, 
                        criteria : & Self::Criteria
                      ) -> Option<Self::Criteria>; //Returns None if destination is not reachable from route_stop

    fn nb_of_route_stops(&self) -> usize;
    //returns an usize between 0 and nb_of_route_stops()
    fn route_stop_id(&self, route_stop : & Self::RouteStop) -> usize;

    fn nb_of_routes(&self) -> usize;
    fn route_id(&self, route : & Self::Route) -> usize;
    
}


