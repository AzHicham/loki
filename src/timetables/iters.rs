
use std::iter::{Chain, Map};
use std::ops::Range;
use super::generic_timetables::{TimetableData, Timetables, Mission, Position, Trip};

pub type MissionIter = Map<Range<usize>, fn(usize) -> Mission>;

impl<Time, TripData>  Timetables<Time, TripData>  
where Time : Ord
{
    pub fn missions(&self) -> MissionIter {
        (0..self.nb_of_missions()).map(|idx| {
            Mission{ idx}
        })
    }

    pub fn trips(&self, mission : & Mission) -> TripsIter {
        let timetable_data = self.timetable_data(mission);
        let nb_of_trips = timetable_data.nb_of_trips();
        TripsIter::new(mission.clone(), 0..nb_of_trips)
    }

    pub fn positions(&self,  mission : & Mission) -> PositionsIter {
        let nb_of_position = self.timetable_data(mission).nb_of_positions();
        PositionsIter::new(mission.clone(), 0..nb_of_position)
    }


}


pub struct PositionsIter {
    mission : Mission,
    position_idxs : Range<usize>,
}

impl PositionsIter {
    fn new(mission : Mission, position_idxs : Range<usize>) -> Self {
        Self {
            mission,
            position_idxs,
        }
    }
}

impl Iterator for PositionsIter {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        self.position_idxs.next().map(|idx| {
            Position {
                mission : self.mission.clone(),
                idx 
            }
        })
    }
}

pub struct TripsIter {
    mission : Mission,
    trip_idxs : Range<usize>,
}

impl TripsIter {
    fn new(mission : Mission, trip_idxs : Range<usize>) -> Self {
        Self {
            mission,
            trip_idxs,
        }
    }
}

impl Iterator for TripsIter {
    type Item = Trip;

    fn next(&mut self) -> Option<Self::Item> {
        self.trip_idxs.next().map(|idx| {
            Trip {
                mission : self.mission.clone(),
                idx 
            }
        })
    }
}

impl<Time, TripData>  TimetableData<Time, TripData>  
// where Time 
{
    
    pub (super) fn trip_debark_times(&self, trip_idx: usize) -> TripTimes<Time> {
        debug_assert!(trip_idx < self.trips_data.len());
        TripTimes {
            times_by_position: &self.debark_times_by_position,
            position: 0,
            trip: trip_idx,
        }
    }

    pub (super) fn trip_board_times(& self, trip_idx: usize) -> TripTimes<Time>  {
        debug_assert!(trip_idx < self.trips_data.len());
        TripTimes {
            times_by_position: &self.board_times_by_position,
            position: 0,
            trip: trip_idx,
        }
    }

    pub (super) fn trip_board_then_debark_times<'a>(
        &'a self,
        trip_idx: usize,
    ) -> Chain<TripTimes<'a, Time>, TripTimes<'a, Time>> {
        self.trip_board_times(trip_idx)
            .chain(self.trip_debark_times(trip_idx))
    }


   

}




pub (super) struct TripTimes<'a, Time> {
    times_by_position: &'a [Vec<Time>],
    position: usize,
    trip: usize,
}

impl<'a, Time> Clone for TripTimes<'a, Time> {
    fn clone(&self) -> Self {
       Self {
           times_by_position : self.times_by_position,
           position : self.position,
           trip : self.trip,
       }
    }
}

impl<'a, Time> Iterator for TripTimes<'a, Time>
// where Time 
 {
    type Item = & 'a Time;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self
            .times_by_position
            .get(self.position)
            .map(|time_by_trips| &time_by_trips[self.trip]);
        if result.is_some() {
            self.position += 1;
        }
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.times_by_position.len() - self.position;
        (remaining, Some(remaining))
    }
}

impl<'a, Time> ExactSizeIterator for TripTimes<'a, Time>
where Time : Clone
 {}
