#[allow(dead_code)]

use crate::public_transit::{PublicTransit};

type Id = usize;

const MAX_ID : Id = std::usize::MAX;

#[derive(Clone, Copy, Debug)]
struct Departure {
    id : Id
}

#[derive(Clone, Copy, Debug)]
struct Board {
    id : Id
}

#[derive(Clone, Copy, Debug)]
struct Alight {
    id : Id
}

#[derive(Clone, Copy, Debug)]
struct Transfer {
    id : Id
}

#[derive(Clone, Copy, Debug)]
struct Arrival {
    id : Id
}

enum BoardParent{
    Departure(Departure),
    Transfer(Transfer),
}

/// A complete journey is a sequence of the form
///  Departure, (Board, Aligth, Transfer)* , Board, Aligth, Arrival
/// i.e. it always starts with a Departure, followed by zero or more (Board, Aligth, Transfer)
///      and then finished by a Board, Aligth, Arrival
/// 
/// We associate the minimum amount of data to each leg type so as to be able to reconstruct
/// the whole journey :
///  - Departure -> a RouteStop 
///  - Board     -> a Trip  
///      the specific RouteStop at which this Trip is boarded is given by the RouteStop
///      associated to the leg before the Board : either a Departure or a Transfer
///  - Alight    -> a RouteStop
///      the specific RouteStop where we alight. The specific Trip that is alighted is
///      given by the Trip associated to the Board leg that comes before this Alight
///  - Transfer  -> a RouteStop
///      the specific RouteStop where the Transfer ends. The specific RouteStop at which
///      this Transfer ends is given by the RouteStop associated to the Aligth leg
///      that comes before this Transfer
///  - Arrival -> nothing
///      the specific RouteStop where this Arrival occurs is given by the RouteStop
///      associated to the Alight leg that comes before this Arrival

struct JourneysTree<PT : PublicTransit> {
    // data associated to each leg
    departures : Vec<PT::RouteStop>,  
    boards     : Vec<PT::Trip>,
    alights    : Vec<PT::RouteStop>,
    transfers  : Vec<PT::RouteStop>,

    // parents 
    alight_parents  : Vec<Board>,
    transfer_parents : Vec<Alight>,
    arrival_parents : Vec<Alight>,
    board_parents   : Vec<BoardParent>,

}

impl<PT : PublicTransit> JourneysTree<PT> {

    fn new() -> Self {
        Self {
            departures : Vec::new(),
            boards : Vec::new(),
            alights : Vec::new(),
            transfers : Vec::new(),

            alight_parents   : Vec::new(),
            transfer_parents : Vec::new(),
            arrival_parents  : Vec::new(),
            board_parents    : Vec::new()
        }
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

