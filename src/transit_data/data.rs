pub use transit_model::{
    objects::{StopPoint, VehicleJourney, Transfer as TransitModelTransfer},

}; 
pub use transit_model::objects::Time as TransitModelTime;
pub use typed_index_collection::{Idx};

use std::{collections::{BTreeMap}};
use super::ordered_timetable::{StopPatternData, Position, Timetable, Vehicle};
use super::calendar::{Calendar, DaysPattern};
use super::time::{PositiveDuration, DaysSinceDatasetStart};



use std::collections::HashMap;



#[derive(Debug, Copy, Clone)]
pub struct Duration {
    pub (super) seconds : u32
}


#[derive(Debug, Clone)]
pub struct VehicleData {
    pub (super) vehicle_journey_idx : Idx<VehicleJourney>,
    pub (super) days_pattern : DaysPattern,

}

pub struct StopData {
    pub (super) stop_point_idx : Idx<StopPoint>,
    pub (super) position_in_patterns : Vec<(StopPattern, Position)>,
    pub (super) transfers : Vec<(Stop, PositiveDuration, Idx<TransitModelTransfer>)>
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Ord, PartialOrd)]
pub enum FlowDirection{
    BoardOnly,
    DebarkOnly,
    BoardAndDebark,
}
pub type StopPoints = Vec< (Idx<StopPoint>, FlowDirection) >;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct StopPattern {
    pub (super) idx : usize
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Stop {
    pub (super) idx : usize
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Transfer {
    pub (super) stop : Stop,
    pub (super) idx_in_stop_transfers : usize,
}


pub struct TransitData {
    pub (super) stop_points_to_pattern : BTreeMap< StopPoints, StopPattern>,
    pub (super) stop_point_idx_to_stop : HashMap< Idx<StopPoint>, Stop  >,

    pub (super) stops_data : Vec<StopData>,
    pub (super) patterns : Vec<StopPatternData>,

    pub calendar : Calendar,


}


#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Mission {
    pub stop_pattern : StopPattern,
    pub timetable : Timetable,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Trip {
    pub mission : Mission,
    pub vehicle : Vehicle,
    pub day : DaysSinceDatasetStart,
}



impl TransitData {
    pub fn stop_data<'a>(& 'a self, stop : & Stop) -> & 'a StopData {
        & self.stops_data[stop.idx]
    }

    pub fn pattern<'a>(& 'a self, pattern : & StopPattern) -> & 'a StopPatternData {
        & self.patterns[pattern.idx]
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

    pub fn nb_of_patterns(&self) -> usize {
        self.patterns.len()
    }

    pub fn nb_of_timetables(&self) -> usize {
        self.patterns.iter().map(|pattern| {
            pattern.nb_of_timetables()
        }).sum()
    }

    pub fn nb_of_vehicles(&self) -> usize {
        self.patterns.iter().map(|pattern| {
            pattern.nb_of_vehicles()
        }).sum()
    }

    pub fn vehicle_journey_idx(&self, trip : & Trip) -> Idx<VehicleJourney> {
        let pattern = &trip.mission.stop_pattern;
        let timetable = &trip.mission.timetable;
        let vehicle = &trip.vehicle;
        self.patterns[pattern.idx].vehicle_data(timetable, vehicle).vehicle_journey_idx
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






