
use crate::engine::public_transit::{PublicTransit, DepartureLeg, ConnectionLeg, Journey};

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
///  Waiting, Onboard, Debarked, (Waiting, Onboard, Debarked)*, Arrived
/// i.e. it always starts with a Waiting, Onboard, Debarked, 
///      followed by zero or more (Waiting, Onboard, Debarked)
///      and then finished by an Arrived
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
///      associated to the Debarked that comes before this Arrived


enum WaitingData<PT : PublicTransit> {
    Transfer(PT::Transfer, Debarked),
    Departure(PT::Departure)
}

pub struct JourneysTree<PT : PublicTransit> {
    // data associated to each moment
    onboards  : Vec<(PT::Trip, Waiting)>,
    debarkeds  : Vec<(PT::Stop, Onboard)>,
    waitings   : Vec<WaitingData<PT>>,
    arriveds   : Vec<(PT::Arrival, Debarked)>

}

impl<PT : PublicTransit> JourneysTree<PT> {

    pub fn new() -> Self {
        Self {
            onboards : Vec::new(),
            debarkeds : Vec::new(),
            waitings : Vec::new(),
            arriveds : Vec::new(),


        }
    }

    pub fn depart(& mut self, departure : & PT::Departure) -> Waiting {
        debug_assert!(self.waitings.len() < MAX_ID);
        let id = self.waitings.len();
        self.waitings.push(WaitingData::Departure(departure.clone()));

        Waiting{ id }
    }


    pub fn board(& mut self, waiting : & Waiting, trip : & PT::Trip) -> Onboard {
        debug_assert!(self.onboards.len() < MAX_ID);
        let id = self.onboards.len();
        self.onboards.push((trip.clone(), waiting.clone()));

        Onboard{ id }
    }

    pub fn debark(& mut self, onboard : & Onboard, route_stop : & PT::Stop) -> Debarked {
        debug_assert!(self.debarkeds.len() < MAX_ID);
        let id = self.debarkeds.len();
        self.debarkeds.push((route_stop.clone(), onboard.clone()));
        Debarked{ id }
    }

    pub fn transfer(& mut self, debarked : & Debarked, transfer : & PT::Transfer) -> Waiting {
        debug_assert!(self.waitings.len() < MAX_ID);
        let id = self.waitings.len();
        self.waitings.push(WaitingData::Transfer(transfer.clone(), debarked.clone()));

        Waiting{ id }
    }

    pub fn arrive(& mut self, debarked : & Debarked, arrival : & PT::Arrival) -> Arrived {
        debug_assert!(self.arriveds.len() < MAX_ID);
        let id = self.arriveds.len();
        self.arriveds.push((arrival.clone(), debarked.clone()));

        Arrived{ id }
    }

    pub fn fill_journey(&self, arrived : & Arrived, journey : & mut Journey<PT>) {
        journey.arrival = (&self.arriveds[arrived.id].0).clone();
        let  connections = & mut journey.connections;
        let new_departure_leg = self.fill_journey_data(arrived,  connections);
        journey.departure_leg = new_departure_leg;
    }

    pub fn create_journey(&self, arrived : & Arrived) -> Journey<PT> {
        let arrival = (&self.arriveds[arrived.id].0).clone();
        let mut connections : Vec<ConnectionLeg<PT>> = Vec::new();
        let departure_leg = self.fill_journey_data(arrived, & mut connections);
        Journey {
            departure_leg,
            connections,
            arrival
        }
    }

    fn fill_journey_data(&self, 
        arrived : & Arrived,
        connections : & mut Vec<ConnectionLeg<PT>>,
        ) -> DepartureLeg<PT>
        {
            connections.clear();
            let mut debarked = &self.arriveds[arrived.id].1;

            loop {
                let (debark_stop, onboard) = self.debarkeds[debarked.id].clone();
                let (trip, waiting) = self.onboards[onboard.id].clone();
                let  prev_waiting = & self.waitings[waiting.id];
                match prev_waiting {
                    WaitingData::Departure(departure) => {
                        let departure_leg = DepartureLeg::<PT> {
                            departure : departure.clone(),
                            trip : trip,
                            debark_stop : debark_stop
                        };
                        connections.reverse();
                        return departure_leg;
                    },
                    WaitingData::Transfer(transfer, prev_debarked) => {
                        let connection_leg = ConnectionLeg {
                            transfer : transfer.clone(),
                            trip,
                            debark_stop
                        };
                        connections.push(connection_leg);
                        debarked = prev_debarked;
                        
                    }

                }

            }
        }

    // pub fn onboard_trip(&self, onboard : & Onboard) -> & PT::Trip {
    //     &self.onboards[onboard.id]
    // }

    // pub fn debarked_stop(&self, debarked : & Debarked) -> & PT::Stop {
    //     &self.debarkeds[debarked.id]
    // }

    // pub fn waiting_stop(&self, waiting : & Waiting ) -> & PT::Stop {
    //     &self.waitings[waiting.id]
    // }

    pub fn clear(&mut self) {
        self.onboards.clear();
        self.debarkeds.clear();
        self.waitings.clear();
        self.arriveds.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.onboards.is_empty()
        && self.debarkeds.is_empty()
        && self.waitings.is_empty()
        && self.arriveds.is_empty()
    }
}
