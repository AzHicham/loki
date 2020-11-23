pub use transit_model::{
    objects::{StopPoint, VehicleJourney, Transfer as TransitModelTransfer},

}; 
pub use transit_model::objects::Time as TransitModelTime;
pub use typed_index_collection::{Idx};


use super::timetables::timetables_data::{Timetables, Position, Timetable, Vehicle};
use crate::time::{Calendar, PositiveDuration};

use std::collections::HashMap;



pub struct TransitData {
    pub (super) stop_point_idx_to_stop : HashMap< Idx<StopPoint>, Stop  >,

    pub (super) stops_data : Vec<StopData>,
    pub (super) timetables : Timetables,

    pub calendar : Calendar,

}
pub struct StopData {
    pub (super) stop_point_idx : Idx<StopPoint>,
    pub (super) position_in_timetables : Vec<Position>,
    pub (super) transfers : Vec<(Stop, PositiveDuration, Idx<TransitModelTransfer>)>
}


#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Ord, PartialOrd)]
pub struct Stop {
    pub (super) idx : usize
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Transfer {
    pub (super) stop : Stop,
    pub (super) idx_in_stop_transfers : usize,
}






#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Mission {
    pub timetable : Timetable,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Trip {
    pub vehicle : Vehicle,
}



impl TransitData {
    pub fn stop_data<'a>(& 'a self, stop : & Stop) -> & 'a StopData {
        & self.stops_data[stop.idx]
    }

    pub fn transfer(&self, transfer : & Transfer) -> (Stop, PositiveDuration) {
        let stop_data = self.stop_data(&transfer.stop);
        let result = stop_data.transfers[transfer.idx_in_stop_transfers];
        (result.0, result.1)
    }

    pub fn transfer_start_stop(&self, transfer : & Transfer) -> Stop {
        transfer.stop
    }

    pub fn nb_of_stops(&self) -> usize {
        self.stops_data.len()
    }

    pub fn stop_to_usize(&self, stop : & Stop) -> usize {
        stop.idx
    }

    pub fn stop_point_idx_to_stop(&self, stop_point_idx : & Idx<StopPoint>) -> Option<&Stop> {
        self.stop_point_idx_to_stop.get(stop_point_idx)
    }

    pub fn nb_of_timetables(&self) -> usize {
        self.timetables.nb_of_timetables()
    }

    pub fn nb_of_vehicles(&self) -> usize {
        self.timetables.nb_of_vehicles()
    }

    pub fn vehicle_journey_idx(&self, trip : & Trip) -> Idx<VehicleJourney> {
        let vehicle = &trip.vehicle;
        self.timetables.vehicle_journey_idx(vehicle)
    }

    pub fn stop_point_idx(&self, stop : & Stop) -> Idx<StopPoint> {
        self.stops_data[stop.idx].stop_point_idx
    }

    pub fn transfer_idx(&self, transfer : & Transfer) -> Idx<TransitModelTransfer> {
        let stop_data = self.stop_data(&transfer.stop);
        let result = stop_data.transfers[transfer.idx_in_stop_transfers];
        result.2
    }

    
}






