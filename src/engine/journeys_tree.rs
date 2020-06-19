
use crate::engine::public_transit::{PublicTransit, DepartureLeg, ConnectionLeg, Journey};

type Id = usize;



#[derive(Clone, Copy, Debug)]
pub struct Board {
    id : Id
}

#[derive(Clone, Copy, Debug)]
pub struct Debark {
    id : Id
}

#[derive(Clone, Copy, Debug)]
pub struct Wait {
    id : Id
}

#[derive(Clone, Copy, Debug)]
pub struct Arrive {
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


enum WaitData<PT : PublicTransit> {
    Transfer(PT::Transfer, Debark),
    Departure(PT::Departure)
}

pub struct JourneysTree<PT : PublicTransit> {
    // data associated to each moment
    boards  : Vec<(PT::Trip, Wait)>,
    debarks  : Vec<(PT::Stop, Board)>,
    waits   : Vec<WaitData<PT>>,
    arrives   : Vec<(PT::Arrival, Debark)>

}

impl<PT : PublicTransit> JourneysTree<PT> {

    pub fn new() -> Self {
        Self {
            boards : Vec::new(),
            debarks : Vec::new(),
            waits : Vec::new(),
            arrives : Vec::new(),


        }
    }

    pub fn depart(& mut self, departure : & PT::Departure) -> Wait {
        let id = self.waits.len();
        self.waits.push(WaitData::Departure(departure.clone()));

        Wait{ id }
    }


    pub fn board(& mut self, waiting : & Wait, trip : & PT::Trip) -> Board {
        let id = self.boards.len();
        self.boards.push((trip.clone(), waiting.clone()));

        Board{ id }
    }

    pub fn debark(& mut self, board : & Board, route_stop : & PT::Stop) -> Debark {
        let id = self.debarks.len();
        self.debarks.push((route_stop.clone(), board.clone()));
        Debark{ id }
    }

    pub fn transfer(& mut self, debark : & Debark, transfer : & PT::Transfer) -> Wait {
        let id = self.waits.len();
        self.waits.push(WaitData::Transfer(transfer.clone(), debark.clone()));

        Wait{ id }
    }

    pub fn arrive(& mut self, debark : & Debark, arrival : & PT::Arrival) -> Arrive {
        let id = self.arrives.len();
        self.arrives.push((arrival.clone(), debark.clone()));

        Arrive{ id }
    }

    pub fn fill_journey(&self, arrive : & Arrive, journey : & mut Journey<PT>) {
        journey.arrival = (&self.arrives[arrive.id].0).clone();
        let  connections = & mut journey.connections;
        let new_departure_leg = self.fill_journey_data(arrive,  connections);
        journey.departure_leg = new_departure_leg;
    }

    pub fn create_journey(&self, arrive : & Arrive) -> Journey<PT> {
        let arrival = (&self.arrives[arrive.id].0).clone();
        let mut connections : Vec<ConnectionLeg<PT>> = Vec::new();
        let departure_leg = self.fill_journey_data(arrive, & mut connections);
        Journey {
            departure_leg,
            connections,
            arrival
        }
    }

    fn fill_journey_data(&self, 
        arrive : & Arrive,
        connections : & mut Vec<ConnectionLeg<PT>>,
        ) -> DepartureLeg<PT>
        {
            connections.clear();
            let mut debark = &self.arrives[arrive.id].1;

            loop {
                let (debark_stop, board) = self.debarks[debark.id].clone();
                let (trip, wait) = self.boards[board.id].clone();
                let  prev_wait = & self.waits[wait.id];
                match prev_wait {
                    WaitData::Departure(departure) => {
                        let departure_leg = DepartureLeg::<PT> {
                            departure : departure.clone(),
                            trip : trip,
                            debark_stop : debark_stop
                        };
                        connections.reverse();
                        return departure_leg;
                    },
                    WaitData::Transfer(transfer, prev_debark) => {
                        let connection_leg = ConnectionLeg {
                            transfer : transfer.clone(),
                            trip,
                            debark_stop
                        };
                        connections.push(connection_leg);
                        debark = prev_debark;
                        
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
        self.boards.clear();
        self.debarks.clear();
        self.waits.clear();
        self.arrives.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.boards.is_empty()
        && self.debarks.is_empty()
        && self.waits.is_empty()
        && self.arrives.is_empty()
    }
}
