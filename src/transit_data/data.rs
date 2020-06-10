
use transit_model;
use transit_model::{
    model::Model as TransitModel,
    objects::{StopPoint, VehicleJourney, Transfer as TransitModelTransfer},

}; 
pub(super) use transit_model::objects::Time as TransitModelTime;


use std::path::PathBuf;
use std::collections::{BTreeMap};
use super::ordered_timetable::{StopPatternTimetables, Position};
use super::calendars::{Calendars, CalendarIdx};
use super::time::{SecondsSinceDayStart, PositiveDuration};
use typed_index_collection::{Idx};

use std::collections::HashMap;

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


#[derive(Debug, Clone)]
pub struct VehicleData {
    pub (super) vehicle_journey_idx : Idx<VehicleJourney>,
    pub (super) calendar_idx : CalendarIdx,

}

pub struct StopData {
    pub (super) stop_point_idx : Idx<StopPoint>,
    pub (super) position_in_forward_patterns : Vec<(StopPattern, Position)>,
    pub (super) transfers : Vec<(Stop, PositiveDuration, Option<Idx<TransitModelTransfer>>)>
}

pub type StopPoints = Vec< Idx<StopPoint> >;

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
    pub (super) arrival_stop_points_to_forward_pattern : BTreeMap< StopPoints, StopPattern>,
    pub (super) stop_point_idx_to_stop : HashMap< Idx<StopPoint>, Stop  >,

    pub (super) stops_data : Vec<StopData>,
    pub (super) forward_patterns : Vec<StopPatternTimetables<VehicleData, SecondsSinceDayStart>>,

    pub (super) calendars : Calendars,


}




impl TransitData {
    pub fn stop_data<'a>(& 'a self, stop : & Stop) -> & 'a StopData {
        & self.stops_data[stop.idx]
    }

    pub fn forward_pattern<'a>(& 'a self, pattern : & StopPattern) -> & 'a StopPatternTimetables<VehicleData, SecondsSinceDayStart> {
        & self.forward_patterns[pattern.idx]
    }

    pub fn transfer(&self, stop : & Stop, transfer : & Transfer) -> (Stop, PositiveDuration) {
        debug_assert!(*stop == transfer.stop);
        let stop_data = self.stop_data(stop);
        let result = stop_data.transfers[transfer.idx_in_stop_transfers];
        (result.0, result.1)
    }

    pub fn nb_of_stops(&self) -> usize {
        self.stops_data.len()
    }

    pub fn stop_idx_to_usize(&self, stop : & Stop) -> usize {
        stop.idx
    }
}






