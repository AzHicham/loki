
use transit_model;
use transit_model::{
    objects::{StopPoint, VehicleJourney, Transfer},
}; 
pub use transit_model::objects::Time  as TimeInDay;
pub use transit_model::objects::Date;

use std::path::PathBuf;
use std::collections::{BTreeMap};
use super::ordered_timetable::StopPatternTimetables;
use super::calendars::{Calendars, CalendarIdx};
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
    pub (super) position_in_arrival_patterns : Vec<(StopPatternIdx, Position)>,
    pub (super) transfers : Vec<(StopIdx, Duration, Option<Idx<Transfer>>)>
}

pub type StopPointArray = Vec< Idx<StopPoint> >;


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct StopPatternIdx {
    pub (super) idx : usize
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct StopIdx {
    pub (super) idx : usize
}


pub struct EngineData {
    pub (super) arrival_stop_point_array_to_stop_pattern : BTreeMap< StopPointArray, StopPatternIdx>,
    pub (super) stop_point_idx_to_stops_idx : BTreeMap< Idx<StopPoint>, Vec< StopIdx > >,

    pub (super) stops : Vec<Stop>,
    pub (super) arrival_stop_patterns : Vec<StopPatternTimetables<VehicleData, TimeInDay>>,

    pub (super) calendars : Calendars,


}








