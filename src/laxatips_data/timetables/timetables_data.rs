use crate::laxatips_data::transit_data::{Stop};
use crate::laxatips_data::days_patterns::DaysPattern;
use std::cmp::Ordering;
use crate::laxatips_data::time::{Calendar, DaysSinceDatasetStart, SecondsSinceDatasetUTCStart};

use transit_model::objects::{VehicleJourney};
use typed_index_collection::{Idx};

use crate::laxatips_data::time::SecondsSinceTimezonedDayStart as Time;
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

    pub (super) earliest_and_latest_board_time_by_position: Vec<(Time, Time)>,
}
#[derive(Debug, Clone)]
pub struct VehicleData {
    pub vehicle_journey_idx : Idx<VehicleJourney>,
    pub days_pattern : DaysPattern,
}



#[derive(Debug, PartialEq, Eq, Clone, Hash, )]
pub struct Timetable {
    pub (super) idx: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Position {
    pub timetable : Timetable,
    pub (super) idx : usize
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Vehicle {
    pub timetable : Timetable,
    pub (super) idx: usize,
}


impl Position {
    pub fn is_upstream(&self, other : & Position) -> Option<bool> {
        if self.timetable != other.timetable {
            None
        }
        else {
            Some(self.idx < other.idx)
        }
    }

    pub fn idx_in_timetable(&self) -> usize {
        self.idx
    }
}


impl Timetables {
    pub fn new() -> Self {
        Self {
            stop_flows_to_timetables : BTreeMap::new(),
            timetable_datas : Vec::new(),
        }
    }

    pub (super) fn timetable_data(&self, timetable : & Timetable) -> & TimetableData {
        & self.timetable_datas[timetable.idx]
    }

    pub fn nb_of_timetables(&self) -> usize {
        self.timetable_datas.len()
    }

    pub fn nb_of_vehicles(&self) -> usize {
        self.timetable_datas.iter()
            .map(|timetable_data| timetable_data.nb_of_vehicles())
            .sum()
    }

    pub fn next_position(&self, position : & Position) -> Option<Position> {
        if position.idx + 1 < self.timetable_data(&position.timetable).nb_of_positions() {
            let result = Position {
                timetable : position.timetable.clone(),
                idx : position.idx + 1
            };
            Some(result)
        }
        else {
            None
        }
    }

    pub fn stop_at(&self, timetable : & Timetable, position : & Position) -> & Stop {
        assert!(*timetable == position.timetable);
        self.timetable_data(&position.timetable).stop_at(position.idx)
    }

    pub fn debark_time_at(&self, vehicle: &Vehicle, position: &Position, day : & DaysSinceDatasetStart, calendar : & Calendar) -> Option<SecondsSinceDatasetUTCStart> {
        assert!(vehicle.timetable == position.timetable);
        let timetable_data = self.timetable_data(&vehicle.timetable);
        timetable_data.debark_time_at(vehicle.idx, position.idx).map(|seconds_in_day| {
            calendar.compose(day, seconds_in_day, &timetable_data.timezone)
        })
    }

    pub fn board_time_at(&self, vehicle: &Vehicle, position: &Position, day : & DaysSinceDatasetStart, calendar : & Calendar) -> Option<SecondsSinceDatasetUTCStart> {
        assert!(vehicle.timetable == position.timetable);
        let timetable_data = self.timetable_data(&vehicle.timetable);
        timetable_data.board_time_at(vehicle.idx, position.idx).map(|seconds_in_day| {
            calendar.compose(day, seconds_in_day, &timetable_data.timezone)
        })
    }

    pub fn arrival_time_at(&self, vehicle: &Vehicle, position: &Position, day : & DaysSinceDatasetStart, calendar : & Calendar) -> SecondsSinceDatasetUTCStart {
        assert!(vehicle.timetable == position.timetable);
        let timetable_data = self.timetable_data(&vehicle.timetable);
        let seconds_in_day = timetable_data.arrival_time_at(vehicle.idx, position.idx);
        calendar.compose(day, seconds_in_day, &timetable_data.timezone)
    }

    fn earliest_and_latest_board_time_at(&self, position : & Position) -> Option<&(Time, Time)> {
        let timetable_data = self.timetable_data(&position.timetable);
        timetable_data.earliest_and_latest_board_time_at(position.idx)
    }

    pub fn vehicle_journey_idx(&self, vehicle: &Vehicle) ->  Idx<VehicleJourney> {
        self.timetable_data(&vehicle.timetable).vehicle_data(vehicle.idx).vehicle_journey_idx
    }

}


impl TimetableData {

    pub (super) fn can_board_at(&self, position_idx: usize) -> bool {
        match &self.stop_flows[position_idx].1 {
            FlowDirection::BoardAndDebark
            | FlowDirection::BoardOnly => true,
            FlowDirection::DebarkOnly => false
        }
    }

    pub (super) fn can_debark_at(&self, position_idx: usize) -> bool {
        match &self.stop_flows[position_idx].1 {
            FlowDirection::BoardAndDebark
            | FlowDirection::DebarkOnly => true,
            FlowDirection::BoardOnly => false
        }
    }

    pub (super) fn arrival_time_at(&self, vehicle_idx : usize, position_idx: usize) -> &Time {
        &self.debark_times_by_position[position_idx][vehicle_idx]
 
        
    }

    pub (super) fn debark_time_at(&self, vehicle_idx : usize, position_idx: usize) -> Option<&Time> {
        if self.can_debark_at(position_idx) {
            Some(&self.debark_times_by_position[position_idx][vehicle_idx])
        }
        else {
            None
        }
        
    }

    pub (super) fn board_time_at(&self, vehicle_idx : usize, position_idx: usize)-> Option<&Time> {
        if self.can_board_at(position_idx) {
            Some(&self.board_times_by_position[position_idx][vehicle_idx])
        }
        else {
            None
        }
        
    }

    pub (super) fn earliest_and_latest_board_time_at(&self, position_idx: usize) -> Option<&(Time, Time)> {
        if self.can_board_at(position_idx) {
            Some(&self.earliest_and_latest_board_time_by_position[position_idx])
        }
        else {
            None
        }
        
    }

    pub (super) fn stop_at(&self, position_idx: usize) -> & Stop {
        &self.stop_flows[position_idx].0
    }

    pub (super) fn flow_direction_at(&self, position_idx: usize) -> & FlowDirection {
        &self.stop_flows[position_idx].1
    }

    pub (super) fn nb_of_positions(&self) -> usize {
        self.board_times_by_position.len()
    }

    pub (super) fn nb_of_vehicles(&self) -> usize {
        self.vehicles_data.len()
    }

    pub (super) fn vehicle_data(&self, vehicle_idx : usize) -> & VehicleData {
        &self.vehicles_data[vehicle_idx]
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
