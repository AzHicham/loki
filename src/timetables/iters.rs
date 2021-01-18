
use std::iter::{Chain, Map};
use std::ops::Range;
use super::generic_timetables::{TimetableData, Timetables, Timetable, Position, Vehicle};

pub type TimetableIter = Map<Range<usize>, fn(usize) -> Timetable>;

impl<Time, TimezoneData, TripData>  Timetables<Time, TimezoneData, TripData>  
where 
Time : Ord,
TimezoneData : PartialEq + Clone
{
    pub fn timetables(&self) -> TimetableIter {
        (0..self.nb_of_timetables()).map(|idx| {
            Timetable{ idx}
        })
    }

    pub fn vehicles(&self, timetable : & Timetable) -> VehicleIter {
        let timetable_data = self.timetable_data(timetable);
        let nb_of_vehicles = timetable_data.nb_of_vehicle();
        VehicleIter::new(timetable.clone(), 0..nb_of_vehicles)
    }

    pub fn positions(&self,  timetable : & Timetable) -> PositionsIter {
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

pub struct VehicleIter {
    timetable : Timetable,
    vehicle_idxs : Range<usize>,
}

impl VehicleIter {
    fn new(timetable : Timetable, vehicle_idxs : Range<usize>) -> Self {
        Self {
            timetable,
            vehicle_idxs,
        }
    }
}

impl Iterator for VehicleIter {
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

impl<Time, TimezoneData, TripData>  TimetableData<Time, TimezoneData, TripData>  
// where Time 
{
    
    pub (super) fn vehicle_debark_times(&self, vehicle_idx: usize) -> VehicleTimes<Time> {
        debug_assert!(vehicle_idx < self.vehicle_datas.len());
        VehicleTimes {
            times_by_position: &self.debark_times_by_position,
            position_idx: 0,
            vehicle_idx,
        }
    }

    pub (super) fn vehicle_board_times(& self, vehicle_idx: usize) -> VehicleTimes<Time>  {
        debug_assert!(vehicle_idx < self.vehicle_datas.len());
        VehicleTimes {
            times_by_position: &self.board_times_by_position,
            position_idx: 0,
            vehicle_idx,
        }
    }

    pub (super) fn vehicle_board_then_debark_times<'a>(
        &'a self,
        vehicle_idx: usize,
    ) -> Chain<VehicleTimes<'a, Time>, VehicleTimes<'a, Time>> {
        self.vehicle_board_times(vehicle_idx)
            .chain(self.vehicle_debark_times(vehicle_idx))
    }


   

}




pub (super) struct VehicleTimes<'a, Time> {
    times_by_position: &'a [Vec<Time>],
    position_idx : usize,
    vehicle_idx : usize,
}

impl<'a, Time> Clone for VehicleTimes<'a, Time> {
    fn clone(&self) -> Self {
       Self {
           times_by_position : self.times_by_position,
           position_idx : self.position_idx,
            vehicle_idx : self.vehicle_idx
       }
    }
}

impl<'a, Time> Iterator for VehicleTimes<'a, Time>
// where Time 
 {
    type Item = & 'a Time;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self
            .times_by_position
            .get(self.position_idx)
            .map(|time_by_vehicles| &time_by_vehicles[self.vehicle_idx]);
        if result.is_some() {
            self.position_idx += 1;
        }
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.times_by_position.len() - self.position_idx;
        (remaining, Some(remaining))
    }
}

impl<'a, Time> ExactSizeIterator for VehicleTimes<'a, Time>
where Time : Clone
 {}
