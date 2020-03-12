#[allow(dead_code)]


trait PublicTransit {


    // A route is a sequence of stop points
    type Route : Clone;
    // A stop point in a route, along with its position in this route
    type RouteStop : Clone;

    // A trip of a vehicle along a route
    type Trip;


    type Criteria : Clone;

    fn is_lower(&self, lower : & Self::Criteria, upper : & Self::Criteria) -> bool;



    // returns true if upstream is positionned strictly before downstream in their common Route
    // panics if upstream and downstream are not in the same Route
    fn is_upstream(&self,  upstream : & Self::RouteStop, downstream : & Self::RouteStop) -> bool;


    fn board(&self,
             route_stop : & Self::RouteStop, 
             criteria : & Self::Criteria
            ) -> (Self::Trip, Self::Criteria);

    fn alight(&self,
              trip : & Self::Trip,
              route_stop : & Self::RouteStop, 
              criteria : & Self::Criteria
             ) -> (Self::Trip, Self::Criteria);


    fn next_on_route(&self, route_stop : & Self::RouteStop) -> Option<Self::RouteStop>;

    fn ride(trip : & Self::Trip, 
            route_stop : & Self::RouteStop,
            criteria : & Self::Criteria
            ) -> Option<Self::Criteria>;

    fn route_of(&self, route_stop : & Self::RouteStop) -> Self::Route;

    type Transfers : Iterator<Item = (Self::RouteStop, Self::Criteria)>;
    fn transfers(departure : Self::RouteStop, start : Self::Criteria) -> Self::Transfers;



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


type PathId = usize;

const MAX_PATH_ID : PathId = std::usize::MAX;

#[derive(Clone, Copy, Debug)]
struct Path {
    id : PathId
}


struct PathTree<Data> {
    parents : Vec<Path>, //store parents of all paths, except the root
    datas : Vec<Data>,
}

impl<Data> PathTree<Data> {

    fn new() -> Self {
        PathTree {
            parents : Vec::new(),
            datas : Vec::new(),
        }
    }

    fn root(&self) -> Path {
        Path{ id : MAX_PATH_ID}
    }

    fn extend(& mut self, path : & Path, data : Data) -> Path {
        assert!(self.parents.len() < MAX_PATH_ID);
        let result = Path{ id : self.parents.len() };
        self.parents.push(*path);
        self.datas.push(data);

        result
    }
}

enum JourneyLeg<Pb : PublicTransit> {
    Departure(Pb::RouteStop),               // leg from the departure point to a public transit stop point
    Board(Pb::Trip, Pb::RouteStop),
    Alight(Pb::Trip, Pb::RouteStop),
    Transfer(Pb::RouteStop, Pb::RouteStop), //a transfer between two stop point
    Arrival(Pb::RouteStop)                  // leg from a public transit stop point to the arrival
}

struct PathData<Pb : PublicTransit> {
    journey_leg : JourneyLeg<Pb>,       // the last journey leg on this path
    criteria : Pb::Criteria         // value of the Criteria at the end of this path
}


struct ParetoFront<Pb : PublicTransit> {
    elements : Vec<(Path, Pb::Criteria)>
}


impl<Pb : PublicTransit> Clone for ParetoFront<Pb> {

    fn clone(& self) -> Self {
        ParetoFront{
            elements : self.elements.clone()
        }
    }
}

impl<Pb : PublicTransit> ParetoFront<Pb> {
    fn new() -> Self {
        Self {
            elements : Vec::new()
        }
    }

    fn add(& mut self, path : & Path, criteria : & Pb::Criteria, pb : & Pb) {
        for (_, old_criteria) in & self.elements {
            if PublicTransit::is_lower(pb, old_criteria, criteria) {
                return;
            }
        }
        self.elements.retain(|(_, old_criteria)| {
            ! PublicTransit::is_lower(pb, criteria, old_criteria)
        });
        self.elements.push((path.clone(), criteria.clone()));
    }

    fn merge_with(& mut self, other : & Self, pb : & Pb) {
        for element in & other.elements {
            let path = &element.0;
            let criteria = &element.1;
            self.add(path, criteria, pb);
        }
    }
}

#[allow(dead_code)]
fn compute<Pb : PublicTransit>(pb : & Pb) -> () {
    let mut path_tree = PathTree::<PathData<Pb>>::new();

    let nb_of_route_stops = pb.nb_of_route_stops();
    // map a route_stop to the pareto front of Pathes which
    // ends at route_stop with a Transit 
    let mut alight_pareto_fronts = vec![ParetoFront::<Pb>::new(); nb_of_route_stops];
    // map a route_stop to the pareto front of Pathes which
    // ends at route_stop with a Transfer or a Departure 
    let mut board_pareto_fronts = vec![ParetoFront::<Pb>::new(); nb_of_route_stops];

    let nb_of_routes = pb.nb_of_routes();
    
    let mut route_has_a_new_board_path : Vec::<Option<Pb::RouteStop>> = vec![None; nb_of_routes];
    let mut routes_with_new_board_path : Vec::<Pb::Route> = Vec::new();

    let root = path_tree.root();
    for (route_stop, criteria) in pb.journey_departures() {
        let path_data = PathData {
            journey_leg :  JourneyLeg::Departure(route_stop.clone()) ,
            criteria : criteria.clone()
        };
        let path = path_tree.extend(&root, path_data);
        let route_stop_id = pb.route_stop_id(&route_stop);
        board_pareto_fronts[route_stop_id].add(&path, &criteria, pb);
        
        let route = pb.route_of(&route_stop);

        let route_id = pb.route_id(&route);
        if let Some(old_board_point) = &route_has_a_new_board_path[route_id] {
            if pb.is_upstream(&route_stop, old_board_point) {
                route_has_a_new_board_path[route_id] = Some(route_stop.clone());
            }
        }
        else {
            route_has_a_new_board_path[route_id] = Some(route_stop.clone());
            routes_with_new_board_path.push(route);
        }

        

    }

    while ! routes_with_new_board_path.is_empty() {
        for route in & routes_with_new_board_path {
            let route_id = pb.route_id(route);
            let mut has_stop = route_has_a_new_board_path[route_id].clone();
            let mut pareto_front = ParetoFront::new();
            while let Some(ref stop) = has_stop {
                let stop_id = pb.route_stop_id(&stop);
                // update alighting pareto front at this stop with elements from
                //   "working" pareto front

                let local_aligth_pathes = & mut alight_pareto_fronts[stop_id];
                local_aligth_pathes.merge_with(&pareto_front, pb);

                // update "working" pareto front with boarding pathes
                let local_board_pathes = board_pareto_fronts[stop_id].clone();
                pareto_front.merge_with(&local_board_pathes, pb);

                // ride to the next stop point and update "working"
                //   pareto front along the way



                // go to next route stop and iterate
                has_stop = pb.next_on_route(stop);
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


