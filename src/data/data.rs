
use transit_model;
use transit_model::{
    objects::{StopPoint, VehicleJourney, Transfer, Time},
}; 
use std::path::PathBuf;
use std::collections::{BTreeMap};
use crate::data::chain_decomposition::ChainDecomposition;
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


pub struct StopPattern {
    pub (super) stops : Vec<StopIdx>,
    pub (super) departure_chains : ChainDecomposition<DailyTripData, Time>,
    pub (super) arrival_chains : ChainDecomposition<DailyTripData, Time>

}

#[derive(Debug, Clone)]
pub struct DailyTripData {
    pub (super) vehicle_journey_idx : Idx<VehicleJourney>,

}

pub struct Stop {
    pub (super) stop_point_idx : Idx<StopPoint>,
    pub (super) position_in_stop_patterns : Vec<(StopPatternIdx, Position)>,
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


pub struct TransitData {
    pub (super) stop_point_array_to_stop_pattern : BTreeMap< StopPointArray, StopPatternIdx>,
    pub (super) stop_point_idx_to_stops_idx : BTreeMap< Idx<StopPoint>, Vec< StopIdx > >,

    pub (super) stops : Vec<Stop>,
    pub (super) stop_patterns : Vec<StopPattern>
}




