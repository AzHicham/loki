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

use super::timetables_data::*;

impl TimetableData {
 
   

    // If we are waiting to board a vehicle at `position` at time `waiting_time`
    // return `Some(best_vehicle)`
    // where `best_vehicle` is the vehicle, among those vehicle on which `filter` returns true,
    //  to board that allows to debark at the subsequent positions at the earliest time,
    fn _best_filtered_vehicle_to_board_at_by_linear_search<Filter>(
        &self,
        waiting_time: &Time,
        position: &Position,
        filter: Filter,
    ) -> Option<Vehicle>
    where
        Filter: Fn(&VehicleData) -> bool,
    {
        self.board_times_by_position[position.idx]
            .iter()
            .zip(self.vehicles_data.iter())
            .enumerate()
            .filter(|(_, (_, vehicle_data))| filter(vehicle_data))
            .find_map(|(idx, (board_time, _))| {
                if waiting_time <= board_time {
                    let vehicle = Vehicle { idx };
                    Some(vehicle)
                } else {
                    None
                }
            })
    }

    // If we are waiting to board a vehicle at `position` at time `waiting_time`
    // return `Some(best_vehicle)`
    // where `best_vehicle` is the vehicle, among those vehicle on which `filter` returns true,
    //  to board that allows to debark at the subsequent positions at the earliest time,
    fn best_filtered_vehicle_to_board_at<Filter>(
        &self,
        waiting_time: &Time,
        position: &Position,
        filter: Filter,
    ) -> Option<Vehicle>
    where
        Filter: Fn(&VehicleData) -> bool,
    {
        let search_result = self.board_times_by_position[position.idx].binary_search(waiting_time);
        let first_boardable_vehicle = match search_result {
            // here it means that
            //    waiting_time < board_time(idx)    if idx < len
            //    waiting_time > board_time(idx -1) if idx > 0
            // so idx is indeed the first vehicle that can be boarded
            Err(idx) => idx,
            // here it means that
            //    waiting_time == board_time(idx)
            // but maybe idx is not the smallest idx such hat waiting_time == board_time(idx)
            Ok(idx) => {
                let mut first_idx = idx;
                while first_idx > 0
                    && self.board_times_by_position[position.idx][first_idx] == *waiting_time
                {
                    first_idx -=  1;
                }
                first_idx
            }
        };

        for vehicle_idx in first_boardable_vehicle..self.nb_of_vehicles() {
            let vehicle_data = &self.vehicles_data[vehicle_idx];
            let board_time = &self.board_times_by_position[position.idx][vehicle_idx];
            if filter(vehicle_data) && waiting_time <= board_time {
                let vehicle = Vehicle { idx: vehicle_idx };
                return Some(vehicle);
            }
        }
        None
    }
}


pub struct PositionPair {
    pub upstream: usize,
    pub downstream: usize,
}

pub enum VehicleTimesError {
    DebarkBeforeUpstreamBoard(PositionPair), // board_time[upstream] > debark_time[downstream]
    DecreasingBoardTime(PositionPair),       // board_time[upstream] > board_time[downstream]
    DecreasingDebarkTime(PositionPair),      // debark_time[upstream] > debark_time[downstream]
}

fn is_increasing<EnumeratedValues>(
    mut enumerated_values: EnumeratedValues,
) -> Result<(), (usize, usize)>
where
    EnumeratedValues: Iterator<Item = (usize, Time)>,
{
    let has_previous = enumerated_values.next();
    let (mut prev_position, mut prev_value) = has_previous.unwrap();
    for (position, value) in enumerated_values {
        if value < prev_value {
            return Err((prev_position, position));
        }
        prev_position = position;
        prev_value = value;
    }
    Ok(())
}

// Retuns
//    - Some(Equal)   if lower[i] == upper[i] for all i
//    - Some(Less)    if lower[i] <= upper[i] for all i
//    - Some(Greater) if lower[i] >= upper[i] for all i
//    - None otherwise (the two vector are not comparable)
fn partial_cmp<Lower, Upper, Value>(lower: Lower, upper: Upper) -> Option<Ordering>
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
