pub mod init;


pub use transit_model::{
    objects::{StopPoint, VehicleJourney, Transfer as TransitModelTransfer},

}; 
pub use transit_model::objects::Time as TransitModelTime;
pub use typed_index_collection::{Idx};


use crate::time::{Calendar, PositiveDuration};

use std::collections::HashMap;

use crate::timetables::{Timetables as TimetablesTrait};


pub struct TransitData<Timetables : TimetablesTrait> {
    pub (super) stop_point_idx_to_stop : HashMap< Idx<StopPoint>, Stop  >,

    pub (super) stops_data : Vec<StopData<Timetables>>,
    pub (super) timetables : Timetables,

    pub calendar : Calendar,

}
pub struct StopData<Timetables : TimetablesTrait> {
    pub (super) stop_point_idx : Idx<StopPoint>,
    pub (super) position_in_timetables : Vec<Timetables::Position>,
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





impl<Timetables : TimetablesTrait> TransitData<Timetables> {
    pub fn stop_data<'a>(& 'a self, stop : & Stop) -> & 'a StopData<Timetables> {
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

    pub fn vehicle_journey_idx(&self, trip : & Timetables::Trip) -> Idx<VehicleJourney> {
        self.timetables.vehicle_journey_idx(trip)
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






