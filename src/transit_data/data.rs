
use transit_model;
use transit_model::{
    model::Model as TransitModel,
    objects::{StopPoint, VehicleJourney, Transfer},
}; 
pub(super) use transit_model::objects::Time as TransitModelTime;


use std::path::PathBuf;
use std::collections::{BTreeMap};
use super::ordered_timetable::StopPatternTimetables;
use super::calendars::{Calendars, CalendarIdx};
use super::time::{SecondsSinceDayStart, PositiveDuration};
use typed_index_collection::{Idx};


fn run() {
    let input_dir = PathBuf::from("tests/fixtures/small_ntfs/");
    let model = transit_model::ntfs::read(input_dir).unwrap();
    let collections = model.into_collections();
    dbg!(collections.vehicle_journeys);

    println!("Hello, world!");
}

#[derive(Debug, Copy, Clone)]
pub struct Duration {
    pub (super) seconds : u32
}

  
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Position {
    pub (super) idx : usize,
}


#[derive(Debug, Clone)]
pub struct VehicleData {
    pub (super) vehicle_journey_idx : Idx<VehicleJourney>,
    pub (super) calendar_idx : CalendarIdx,

}

pub struct Stop {
    pub (super) stop_point_idx : Idx<StopPoint>,
    // TODO ? : replace Vec by HashMap/BTreeMap StopPatternIdx -> Position 
    pub (super) position_in_arrival_patterns : BTreeMap<StopPatternIdx, Position>,
    pub (super) transfers : Vec<(StopIdx, PositiveDuration, Option<Idx<Transfer>>)>
}

pub type StopPointArray = Vec< Idx<StopPoint> >;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Ord, PartialOrd)]
pub struct StopPatternIdx {
    pub (super) idx : usize
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct StopIdx {
    pub (super) idx : usize
}

pub struct TransferIdx {
    pub (super) stop_idx : StopIdx,
    pub (super) idx_in_stop_transfers : usize,
}


pub struct EngineData {
    pub (super) arrival_stop_point_array_to_stop_pattern : BTreeMap< StopPointArray, StopPatternIdx>,
    pub (super) stop_point_idx_to_stops_idx : BTreeMap< Idx<StopPoint>, Vec< StopIdx > >,

    pub (super) stops : Vec<Stop>,
    pub (super) arrival_stop_patterns : Vec<StopPatternTimetables<VehicleData, SecondsSinceDayStart>>,

    pub (super) calendars : Calendars,


}

pub struct TransitData {
    pub engine_data : EngineData,
    pub transit_model : TransitModel,
}


impl EngineData {
    pub fn stop<'a>(& 'a self, stop_idx : & StopIdx) -> & 'a Stop {
        & self.stops[stop_idx.idx]
    }

    pub fn arrival_pattern<'a>(& 'a self, arrival_pattern_idx : & StopPatternIdx) -> & 'a StopPatternTimetables<VehicleData, SecondsSinceDayStart> {
        & self.arrival_stop_patterns[arrival_pattern_idx.idx]
    }
}






