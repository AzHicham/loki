
use crate::public_transit::{PublicTransit};

type Id = usize;

const MAX_ID : Id = std::usize::MAX;


#[derive(Clone, Copy, Debug)]
pub struct Onboard {
    id : Id
}

#[derive(Clone, Copy, Debug)]
pub struct Debarked {
    id : Id
}

#[derive(Clone, Copy, Debug)]
pub struct Waiting {
    id : Id
}

#[derive(Clone, Copy, Debug)]
pub struct Arrived {
    id : Id
}



/// A complete journey is a sequence of moments the form
///  Waiting, (Onboard, Debarked, Waiting)* , Onboard, Debarked, Arrived
/// i.e. it always starts with a Waiting, followed by zero or more (Onboard, Debarked, Waiting)
///      and then finished by a Onboard, Debarked, Arrived
/// 
/// We associate the minimum amount of data to each moment so as to be able to reconstruct
/// the whole journey :
///  - Onboard     -> a Trip  
///      the specific RouteStop at which this Trip is boarded is given by the RouteStop
///      associated to the Waiting before the Onboard 
///  - Debarked    -> a RouteStop
///      the specific RouteStop where we alight. The specific Trip that is alighted is
///      given by the Trip associated to the Onboard moment that comes before this Debarked
///  - Waiting  -> a RouteStop
///      the specific RouteStop where we are waiting. 
///      A Waiting can occurs either :
///         - at the beginning of the journey, 
///         - or between a Debarked and a Onboard, which means we are making a transfer
///           between two vehicles. 
///      In the second case, the the specific RouteStop at which
///      this transfer begins is given by the RouteStop associated to the Debarked moment
///      that comes before this Waiting
///  - Arrived -> nothing
///      the specific RouteStop where this Arrival occurs is given by the RouteStop
///      associated to the Debarked leg that comes before this Arrived

pub struct JourneysTree<PT : PublicTransit> {
    // data associated to each leg
    onboards  : Vec<PT::Trip>,
    debarkeds  : Vec<PT::RouteStop>,
    waitings   : Vec<PT::RouteStop>,

    // parents 
    onboard_parents   : Vec<Waiting>,
    debarked_parents  : Vec<Onboard>,
    waiting_parents : Vec<Option<Debarked>>,
    arrived_parents : Vec<Debarked>,


}

impl<PT : PublicTransit> JourneysTree<PT> {

    pub fn new() -> Self {
        Self {
            onboards : Vec::new(),
            debarkeds : Vec::new(),
            waitings : Vec::new(),

            onboard_parents    : Vec::new(),
            debarked_parents   : Vec::new(),
            waiting_parents : Vec::new(),
            arrived_parents  : Vec::new(),

        }
    }

    pub fn depart(& mut self, route_stop : & PT::RouteStop) -> Waiting {
        debug_assert!(self.waitings.len() < MAX_ID);
        debug_assert!(self.waitings.len() == self.waiting_parents.len());
        let id = self.waitings.len();
        self.waitings.push(route_stop.clone());
        self.waiting_parents.push(None);

        Waiting{ id }
    }


    pub fn board(& mut self, waiting : & Waiting, trip : & PT::Trip) -> Onboard {
        debug_assert!(self.onboards.len() < MAX_ID);
        debug_assert!(self.onboards.len() == self.onboard_parents.len());
        let id = self.onboards.len();
        self.onboards.push(trip.clone());
        self.onboard_parents.push(waiting.clone());

        Onboard{ id }
    }

    pub fn debark(& mut self, onboard : & Onboard, route_stop : & PT::RouteStop) -> Debarked {
        debug_assert!(self.debarkeds.len() < MAX_ID);
        debug_assert!(self.debarkeds.len() == self.debarked_parents.len());
        let id = self.debarkeds.len();
        self.debarkeds.push(route_stop.clone());
        self.debarked_parents.push(onboard.clone());

        Debarked{ id }
    }

    pub fn transfer(& mut self, debarked : & Debarked, route_stop : & PT::RouteStop) -> Waiting {
        debug_assert!(self.waitings.len() < MAX_ID);
        debug_assert!(self.waitings.len() == self.waiting_parents.len());
        let id = self.waitings.len();
        self.waitings.push(route_stop.clone());
        self.waiting_parents.push(Some(debarked.clone()));

        Waiting{ id }
    }

    pub fn arrive(& mut self, debarked : & Debarked) -> Arrived {
        debug_assert!(self.arrived_parents.len() < MAX_ID);
        let id = self.arrived_parents.len();
        self.arrived_parents.push(debarked.clone());

        Arrived{ id }
    }

    pub fn onboard_trip(&self, onboard : & Onboard) -> & PT::Trip {
        &self.onboards[onboard.id]
    }

    pub fn debarked_stop(&self, debarked : & Debarked) -> & PT::RouteStop {
        &self.debarkeds[debarked.id]
    }

    pub fn waiting_stop(&self, waiting : & Waiting ) -> & PT::RouteStop {
        &self.waitings[waiting.id]
    }
}

