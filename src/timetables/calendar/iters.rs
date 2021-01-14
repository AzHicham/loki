
use std::iter::{Chain, Map};
use std::ops::Range;
use crate::time::SecondsSinceTimezonedDayStart as Time;
use super::timetables_data::{TimetableData, Timetables, Timetable, Position, Vehicle};

pub type TimetablesIter = Map<Range<usize>, fn(usize) -> Timetable>;

impl Timetables {
    pub fn timetables(&self) -> TimetablesIter {
        (0..self.nb_of_timetables()).map(|idx| {
            Timetable{ idx}
        })
    }

    pub fn vehicles(&self, timetable : & Timetable) -> VehiclesIter {
        let timetable_data = self.timetable_data(timetable);
        let nb_of_vehicles = timetable_data.nb_of_vehicles();
        VehiclesIter::new(timetable.clone(), 0..nb_of_vehicles)
    }

    pub fn positions(&self, timetable : & Timetable) -> PositionsIter {
        let nb_of_position = self.timetable_data(timetable).nb_of_positions();
        PositionsIter::new(timetable.clone(), 0..nb_of_position)
    }


}


pub struct PositionsIter {
    timetable : Timetable,
    position_idxs : Range<usize>,
}

impl PositionsIter {
    fn new(timetable : Timetable, position_idxs : Range<usize>) -> Self {
        Self {
            timetable,
            position_idxs,
        }
    }
}

impl Iterator for PositionsIter {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        self.position_idxs.next().map(|idx| {
            Position {
                timetable : self.timetable.clone(),
                idx 
            }
        })
    }
}

pub struct VehiclesIter {
    timetable : Timetable,
    vehicle_idxs : Range<usize>,
}

impl VehiclesIter {
    fn new(timetable : Timetable, vehicle_idxs : Range<usize>) -> Self {
        Self {
            timetable,
            vehicle_idxs,
        }
    }
}

impl Iterator for VehiclesIter {
    type Item = Vehicle;

    fn next(&mut self) -> Option<Self::Item> {
        self.vehicle_idxs.next().map(|idx| {
            Vehicle {
                timetable : self.timetable.clone(),
                idx 
            }
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
