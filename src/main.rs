
#[allow(dead_code)]


trait PublicTransit {
    type StopPoint : Clone;

    type Route : Clone;

    type Trip;
    type TripStop;

    type Transfer;

    type Criteria : Clone;

    fn is_lower(&self, lower : & Self::Criteria, upper : & Self::Criteria) -> bool;

    type Routes : Iterator<Item = Self::Route>;
    fn routes(&self, stop_point : & Self::StopPoint) -> Self::Routes;

    // returns true if upstream is positionned strictly before downstream in  route
    // panics if upstream or downstream is not in route
    fn is_upstream(&self, route : & Self::Route, upstream : & Self::StopPoint, downstream : & Self::StopPoint) -> bool;

    fn embark(route : & Self::Route, stop_point : & Self::StopPoint, criteria : Self::Criteria) -> (Self::TripStop, Self::Criteria);

    type Ride : Iterator<Item = (Self::TripStop, Self::Criteria)>;
    fn ride(trip_stop : Self::TripStop, criteria : Self::Criteria) -> Self::Ride;

    fn stop_point_of(trip_stop : Self::TripStop) -> Self::StopPoint;
    fn trip_of(trip_stop : Self::TripStop) -> Self::Trip;
    fn route_of(trip : Self::Trip) -> Self::Route;

    type Transfers : Iterator<Item = (Self::Transfer, Self::Criteria)>;
    fn transfers(departure : Self::StopPoint, start : Self::Criteria) -> Self::Transfers;

    fn start_of(transfer : Self::Transfer) -> Self::StopPoint;
    fn end_of(transfer : Self::Transfer) -> Self::StopPoint;

    type JourneyDepartures : Iterator<Item = (Self::StopPoint, Self::Criteria)>;
    fn journey_departures(&self) -> Self::JourneyDepartures;
    fn journey_arrival(stop_point : Self::StopPoint, criteria : Self::Criteria) -> Self::Criteria;

    fn nb_of_stop_points(&self) -> usize;
    //returns an usize between 0 and nb_of_stop_points()
    fn stop_point_id(&self, stop_point : & Self::StopPoint) -> usize;

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
    Departure(Pb::StopPoint),               // leg from the departure point to a public transit stop point
    Transit(Pb::TripStop, Pb::TripStop),    // a ride of a public transit Trip between two TripStop
    Transfer(Pb::Transfer),                 //a transfer between two stop point
    Arrival(Pb::StopPoint)                  // leg from a public transit stop point to the arrival
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
}

#[allow(dead_code)]
fn compute<Pb : PublicTransit>(pb : & Pb) -> () {
    let mut path_tree = PathTree::<PathData<Pb>>::new();

    let nb_of_stop_points = pb.nb_of_stop_points();
    // map a stop_point_id to the pareto front of Pathes which
    // ends at stop_point with a Transit 
    let mut alight_pareto_fronts = vec![ParetoFront::<Pb>::new(); nb_of_stop_points];
    // map a stop_point_id to the pareto front of Pathes which
    // ends at stop_point with a Transfer or a Departure 
    let mut board_pareto_fronts = vec![ParetoFront::<Pb>::new(); nb_of_stop_points];

    let nb_of_routes = pb.nb_of_routes();
    
    let mut route_has_a_new_board_path = vec![None; nb_of_routes];
    let mut routes_with_new_board_path : Vec::<Pb::Route> = Vec::new();

    let root = path_tree.root();
    for (stop_point, criteria) in pb.journey_departures() {
        let path_data = PathData {
            journey_leg :  JourneyLeg::Departure(stop_point.clone()) ,
            criteria : criteria.clone()
        };
        let path = path_tree.extend(&root, path_data);
        let stop_point_id = pb.stop_point_id(&stop_point);
        board_pareto_fronts[stop_point_id].add(&path, &criteria, pb);
        
        for route in pb.routes(&stop_point) {
            let route_id = pb.route_id(&route);
            if let Some(old_board_point) = &route_has_a_new_board_path[route_id] {
                if pb.is_upstream(&route, &stop_point, old_board_point) {
                    route_has_a_new_board_path[route_id] = Some(stop_point.clone());
                }
            }
            else {
                route_has_a_new_board_path[route_id] = Some(stop_point.clone());
                routes_with_new_board_path.push(route);
            }

        }

    }

    while ! routes_with_new_board_path.is_empty() {
        for route in & routes_with_new_board_path {
            let route_id = pb.route_id(route);
            let board_stop_point = route_has_a_new_board_path[route_id].clone().unwrap();
            let board_stop_point_id = pb.stop_point_id(&board_stop_point);
            let pareto_front = board_pareto_fronts[board_stop_point_id].clone();
            for (path, criteria) in & pareto_front.elements {

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


