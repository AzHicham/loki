#[allow(dead_code)]

use crate::public_transit::{PublicTransit};

type Id = usize;

const MAX_ID : Id = std::usize::MAX;


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



/// A complete journey is a sequence of the form
///  Transfer, (Board, Aligth, Transfer)* , Board, Aligth, Arrival
/// i.e. it always starts with a Transfer, followed by zero or more (Board, Aligth, Transfer)
///      and then finished by a Board, Aligth, Arrival
/// 
/// We associate the minimum amount of data to each leg type so as to be able to reconstruct
/// the whole journey :
///  - Board     -> a Trip  
///      the specific RouteStop at which this Trip is boarded is given by the RouteStop
///      associated to the leg before the Board : either a Departure or a Transfer
///  - Alight    -> a RouteStop
///      the specific RouteStop where we alight. The specific Trip that is alighted is
///      given by the Trip associated to the Board leg that comes before this Alight
///  - Transfer  -> a RouteStop
///      the specific RouteStop where the Transfer ends. 
///      If this Transfer is not the beginning of the journey (i.e. the Departure leg), 
///      the specific RouteStop at which
///      this Transfer ends is given by the RouteStop associated to the Aligth leg
///      that comes before this Transfer
///  - Arrival -> nothing
///      the specific RouteStop where this Arrival occurs is given by the RouteStop
///      associated to the Alight leg that comes before this Arrival

struct JourneysTree<PT : PublicTransit> {
    // data associated to each leg
    boards     : Vec<PT::Trip>,
    alights    : Vec<PT::RouteStop>,
    transfers  : Vec<PT::RouteStop>,

    // parents 
    board_parents   : Vec<Transfer>,
    alight_parents  : Vec<Board>,
    transfer_parents : Vec<Option<Alight>>,
    arrival_parents : Vec<Alight>,


}

impl<PT : PublicTransit> JourneysTree<PT> {

    fn new() -> Self {
        Self {
            boards : Vec::new(),
            alights : Vec::new(),
            transfers : Vec::new(),

            board_parents    : Vec::new()
            alight_parents   : Vec::new(),
            transfer_parents : Vec::new(),
            arrival_parents  : Vec::new(),

        }
    }

    fn depart(& mut self, route_stop : & PT::RouteStop) -> Transfer {
        debug_assert!(self.transfers.len() < MAX_ID);
        debug_assert!(self.transfers.len() == self.transfer_parents.len());
        let id = self.transfers.len();
        self.transfers.push(route_stop.clone());
        self.transfer_parents.push(None);

        Transfer{ id }
    }


    fn board(& mut self, transfer : & Transfer, trip : & PT::Trip) -> Board {
        debug_assert!(self.boards.len() < MAX_ID);
        debug_assert!(self.boards.len() == self.board_parents.len());
        let id = self.boards.len();
        self.boards.push(trip.clone());
        self.board_parents.push(transfer.clone());

        Board{ id }
    }

    fn alight(& mut self, board : & Board, route_stop : & PT::RouteStop) -> Alight {
        debug_assert!(self.aligths.len() < MAX_ID);
        debug_assert!(self.alights.len() == self.alight_parents.len());
        let id = self.alights.len();
        self.alights.push(route_stop.clone());
        self.alight_parents.push(board.clone());

        Alight{ id }
    }

    fn transfer(& mut self, alight : & Alight, route_stop : & PT::RouteStop) -> Transfer {
        debug_assert!(self.transfers.len() < MAX_ID);
        debug_assert!(self.transfers.len() == self.transfer_parents.len());
        let id = self.transfers.len();
        self.transfers.push(route_stop.clone());
        self.transfer_parents.push(alight.clone());

        Transfer{ id }
    }

    fn arrive(& mut self, alight : & Alight) -> Arrival {
        debug_assert!(self.arrival_parents.len() < MAX_ID);
        let id = self.arrival_parents.len();
        self.arrival_parents.push(alight.clone());

        Arrival{ id }
    }
}

