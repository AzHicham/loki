use crate::traits::{ConnectionLeg, DepartureLeg, Journey, RequestTypes};
type Id = usize;

#[derive(Clone, Copy, Debug)]
pub struct Board {
    id: Id,
}

#[derive(Clone, Copy, Debug)]
pub struct Debark {
    id: Id,
}

#[derive(Clone, Copy, Debug)]
pub struct Wait {
    id: Id,
}

#[derive(Clone, Copy, Debug)]
pub struct Arrive {
    id: Id,
}

/// A complete journey is a sequence of moments the form
///  Wait, Board, Debark, (Wait, Board, Debark)*, Arrive
/// i.e. it always starts with a Wait, Board, Debark,
///      followed by zero or more (Wait, Board, Debark)
///      and then finished by an Arrive
///
/// We associate the minimum amount of data to each moment so as to be able to reconstruct
/// the whole journey :
///  - Board     -> a (Trip, Position)
///  - Debark   -> a Position
///      The specific Trip that is alighted is
///      given by the Trip associated to the Board moment that comes before this Debark
///  - Wait  -> either a Transfer of a Departure
///      A Wait can occurs either :
///         - at the beginning of the journey,
///         - or between a Debark and a Board, which means we are making a transfer
///           between two vehicles.
///  - Arrive -> an Arrival

enum WaitData<PT: RequestTypes> {
    Transfer(PT::Transfer, Debark),
    Departure(PT::Departure),
}

pub struct JourneysTree<PT: RequestTypes> {
    // data associated to each moment
    boards: Vec<(PT::Trip, PT::Position, Wait)>,
    debarks: Vec<(PT::Position, Board)>,
    waits: Vec<WaitData<PT>>,
    arrives: Vec<(PT::Arrival, Debark)>,
}

impl<PT: RequestTypes> JourneysTree<PT> {
    pub fn new() -> Self {
        Self {
            boards: Vec::new(),
            debarks: Vec::new(),
            waits: Vec::new(),
            arrives: Vec::new(),
        }
    }

    pub fn depart(&mut self, departure: &PT::Departure) -> Wait {
        let id = self.waits.len();
        self.waits.push(WaitData::Departure(departure.clone()));

        Wait { id }
    }

    pub fn board(&mut self, wait: &Wait, trip: &PT::Trip, position: &PT::Position) -> Board {
        let id = self.boards.len();
        self.boards.push((trip.clone(), position.clone(), *wait));

        Board { id }
    }

    pub fn debark(&mut self, board: &Board, position: &PT::Position) -> Debark {
        let id = self.debarks.len();
        self.debarks.push((position.clone(), *board));
        Debark { id }
    }

    pub fn transfer(&mut self, debark: &Debark, transfer: &PT::Transfer) -> Wait {
        let id = self.waits.len();
        self.waits
            .push(WaitData::Transfer(transfer.clone(), *debark));

        Wait { id }
    }

    pub fn arrive(&mut self, debark: &Debark, arrival: &PT::Arrival) -> Arrive {
        let id = self.arrives.len();
        self.arrives.push((arrival.clone(), *debark));

        Arrive { id }
    }

    pub fn fill_journey(
        &self,
        arrive: &Arrive,
        criteria: &PT::Criteria,
        journey: &mut Journey<PT>,
    ) {
        journey.arrival = (&self.arrives[arrive.id].0).clone();
        let connection_legs = &mut journey.connection_legs;
        let new_departure_leg = self.fill_journey_data(arrive, connection_legs);
        journey.departure_leg = new_departure_leg;
        journey.criteria_at_arrival = criteria.clone();
    }

    pub fn create_journey(&self, arrive: &Arrive, criteria: &PT::Criteria) -> Journey<PT> {
        let arrival = (&self.arrives[arrive.id].0).clone();
        let mut connection_legs: Vec<ConnectionLeg<PT>> = Vec::new();
        let departure_leg = self.fill_journey_data(arrive, &mut connection_legs);
        Journey {
            departure_leg,
            connection_legs,
            arrival,
            criteria_at_arrival: criteria.clone(),
        }
    }

    pub fn size(&self) -> usize {
        self.waits.len() + self.debarks.len() + self.boards.len() + self.arrives.len()
    }

    fn fill_journey_data(
        &self,
        arrive: &Arrive,
        connections: &mut Vec<ConnectionLeg<PT>>,
    ) -> DepartureLeg<PT> {
        connections.clear();
        let mut debark = &self.arrives[arrive.id].1;

        loop {
            let (debark_position, board) = self.debarks[debark.id].clone();
            let (trip, board_position, wait) = self.boards[board.id].clone();
            let prev_wait = &self.waits[wait.id];
            match prev_wait {
                WaitData::Departure(departure) => {
                    let departure_leg = DepartureLeg::<PT> {
                        departure: departure.clone(),
                        trip,
                        board_position,
                        debark_position,
                    };
                    connections.reverse();
                    return departure_leg;
                }
                WaitData::Transfer(transfer, prev_debark) => {
                    let connection_leg = ConnectionLeg {
                        transfer: transfer.clone(),
                        trip,
                        board_position,
                        debark_position,
                    };
                    connections.push(connection_leg);
                    debark = prev_debark;
                }
            }
        }
    }

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
