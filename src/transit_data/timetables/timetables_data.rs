use crate::transit_data::data::{Stop};
use crate::transit_data::calendar::{DaysPattern};
use std::cmp::Ordering;
use std::iter::{Chain, Map};
use std::ops::Range;

use transit_model::objects::{VehicleJourney};
use typed_index_collection::{Idx};

use crate::transit_data::time::SecondsSinceDayStart as Time;
use chrono_tz::Tz as TimeZone;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Ord, PartialOrd)]
pub enum FlowDirection{
    BoardOnly,
    DebarkOnly,
    BoardAndDebark,
}
pub type StopFlows = Vec< (Stop, FlowDirection) >;


#[derive(Debug)]
pub struct Timetables {
    pub (super) stop_flows_to_timetables : BTreeMap< StopFlows, Vec<Timetable> >,
    pub (super) timetable_datas : Vec<TimetableData>,
}

#[derive(Debug)]
// TODO : document more explicitely !
pub (super) struct TimetableData {

    pub (super) stop_flows: StopFlows,

    pub (super) timezone : TimeZone,

    // vehicles data, ordered by increasing times
    // meaning that is v1 is before v2 in this vector,
    // then for all `position` we have
    //    debark_time_by_vehicle[v1][position] <= debark_time_by_vehicle[v2][position]
    pub (super) vehicles_data: Vec<VehicleData>,

    // `board_times_by_position[position][vehicle]`
    //   is the time at which a traveler waiting
    //   at `position` can board `vehicle`
    // Vehicles are ordered by increasing time
    //  so for each `position` the vector
    //  board_times_by_position[position] is sorted by increasing times
    pub (super) board_times_by_position: Vec<Vec<Time>>,

    // `debark_times_by_position[position][vehicle]`
    //   is the time at which a traveler being inside `vehicle`
    //   will debark at `position`
    // Vehicles are ordered by increasing time
    //  so for each `position` the vector
    //  debark_times_by_position[position] is sorted by increasing times
    pub (super) debark_times_by_position: Vec<Vec<Time>>,

    pub (super) latest_board_time_by_position: Vec<Time>,
}
#[derive(Debug, Clone)]
pub (super) struct VehicleData {
    pub (super) vehicle_journey_idx : Idx<VehicleJourney>,
    pub (super) days_pattern : DaysPattern,
}


#[derive(Debug, PartialEq, Eq, Copy, Clone, Ord, PartialOrd)]
pub struct Position {
    pub (super) idx: usize,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Timetable {
    pub (super) idx: usize,
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Vehicle {
    pub (super) idx: usize,
}



impl Timetables {
    pub fn new() -> Self {
        Self {
            stop_flows_to_timetables : BTreeMap::new(),
            timetable_datas : Vec::new(),
        }
    }

    
}
pub type TimetablesIter = Map<Range<usize>, fn(usize) -> Timetable>;

pub type VehiclesIter = Map<Range<usize>, fn(usize) -> Vehicle>;

impl TimetableData {
    fn new(stop_flows : StopFlows, timezone : & TimeZone) -> Self {
        let nb_of_positions = stop_flows.len();
        assert!(nb_of_positions >= 2);
        Self {
            stop_flows,
            timezone : timezone.clone(),
            vehicles_data: Vec::new(),
            debark_times_by_position: vec![Vec::new(); nb_of_positions],
            board_times_by_position: vec![Vec::new(); nb_of_positions],
            latest_board_time_by_position: vec![Time::zero(); nb_of_positions],
        }
    }

    fn debark_time_at(&self, vehicle: &Vehicle, position: &Position) -> &Time {
        &self.debark_times_by_position[position.idx][vehicle.idx]
    }

    fn board_time_at(&self, vehicle: &Vehicle, position: &Position) -> &Time {
        &self.board_times_by_position[position.idx][vehicle.idx]
    }

    fn latest_board_time_at(&self, position: &Position) -> &Time {
        &self.latest_board_time_by_position[position.idx]
    }

    pub (super) fn nb_of_positions(&self) -> usize {
        self.board_times_by_position.len()
    }

    pub (super) fn nb_of_vehicles(&self) -> usize {
        self.vehicles_data.len()
    }

   
}

// Retuns
//    - Some(Equal)   if lower[i] == upper[i] for all i
//    - Some(Less)    if lower[i] <= upper[i] for all i
//    - Some(Greater) if lower[i] >= upper[i] for all i
//    - None otherwise (the two vector are not comparable)
pub (super) fn partial_cmp<Lower, Upper, Value>(lower: Lower, upper: Upper) -> Option<Ordering>
where
    Lower: Iterator<Item = Value> + Clone,
    Upper: Iterator<Item = Value> + Clone,
    Value: Ord,
{
    debug_assert!(lower.clone().count() == upper.clone().count());
    let zip_iter = lower.zip(upper);
    let mut first_not_equal_iter =
        zip_iter.skip_while(|(lower_val, upper_val)| lower_val == upper_val);
    let has_first_not_equal = first_not_equal_iter.next();
    if let Some(first_not_equal) = has_first_not_equal {
        let ordering = {
            let lower_val = first_not_equal.0;
            let upper_val = first_not_equal.1;
            lower_val.cmp(&upper_val)
        };
        debug_assert!(ordering != Ordering::Equal);
        // let's see if there is an index where the ordering is not the same
        // as first_ordering
        let found = first_not_equal_iter.find(|(lower_val, upper_val)| {
            let cmp = lower_val.cmp(&upper_val);
            cmp != ordering && cmp != Ordering::Equal
        });
        if found.is_some() {
            return None;
        }
        // if found.is_none(), it means that
        // all elements are ordered the same, so the two vectors are comparable
        return Some(ordering);
    }
    // if has_first_not_equal == None
    // then values == item_values
    // the two vector are equal
    Some(Ordering::Equal)
}
#[derive(Clone)]
struct VehicleTimes<'a> {
    times_by_position: &'a [Vec<Time>],
    position: usize,
    vehicle: usize,
}

impl<'a> Iterator for VehicleTimes<'a> {
    type Item = Time;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self
            .times_by_position
            .get(self.position)
            .map(|time_by_vehicles| &time_by_vehicles[self.vehicle]);
        if result.is_some() {
            self.position += 1;
        }
        result.cloned()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.times_by_position.len() - self.position;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for VehicleTimes<'a> {}
