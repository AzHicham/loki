use crate::laxatips_data::transit_data::{Stop};
use crate::laxatips_data::calendar::{DaysPattern};
use std::cmp::Ordering;
use std::iter::{Chain, Map};
use std::ops::Range;

use transit_model::objects::{VehicleJourney};
use typed_index_collection::{Idx};

use crate::laxatips_data::time::SecondsSinceTimezonedDayStart as Time;
use chrono_tz::Tz as TimeZone;
use std::collections::BTreeMap;

use super::timetables_data::*;

pub type TimetablesIter = Map<Range<usize>, fn(usize) -> Timetable>;

pub type VehiclesIter = Map<Range<usize>, fn(usize) -> Vehicle>;

impl Timetables {
    pub fn timetables(&self) -> TimetablesIter {
        (0..self.nb_of_timetables()).map(|idx| {
            Timetable{ idx}
        })
    }
}

impl TimetableData {
    
    pub (super) fn vehicle_debark_times(&self, vehicle_idx: usize) -> VehicleTimes {
        debug_assert!(vehicle_idx < self.vehicles_data.len());
        VehicleTimes {
            times_by_position: &self.debark_times_by_position,
            position: 0,
            vehicle: vehicle_idx,
        }
    }

    pub (super) fn vehicle_board_times(& self, vehicle_idx: usize) -> VehicleTimes {
        debug_assert!(vehicle_idx < self.vehicles_data.len());
        VehicleTimes {
            times_by_position: &self.board_times_by_position,
            position: 0,
            vehicle: vehicle_idx,
        }
    }

    pub (super) fn vehicle_board_then_debark_times<'a>(
        &'a self,
        vehicle_idx: usize,
    ) -> Chain<VehicleTimes<'a>, VehicleTimes<'a>> {
        self.vehicle_board_times(vehicle_idx)
            .chain(self.vehicle_debark_times(vehicle_idx))
    }

   

}



#[derive(Clone)]
pub (super) struct VehicleTimes<'a> {
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
