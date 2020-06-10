
use super::data::{
    TransitData,
    StopData,
    Stop,
    StopPattern,
    VehicleData,
    Transfer,
};

use super::time::{ DaysSinceDatasetStart ,SecondsSinceDatasetStart, SecondsSinceDayStart};


use super::ordered_timetable::{Timetable, Vehicle, TimetableData, TimetablesIter, Position};

use std::collections::btree_map::Keys;
use std::slice::Iter as SliceIter;

type ForwardPatternsOfStop<'a> = SliceIter<'a, (StopPattern, Position)>;

impl TransitData {
    pub fn forward_patterns_of<'a>(&'a self, stop : & Stop) -> ForwardPatternsOfStop<'a> {
        let stop_data = self.stop_data(stop);
        stop_data.position_in_forward_patterns.iter()
    }

    pub fn forward_pattern_and_timetables_of<'a>(&'a self, stop : & Stop) -> ForwardTimetablesOfStop<'a> {
        ForwardTimetablesOfStop::new(&self, stop)
    }

    pub fn transfers_of(& self, stop : & Stop) -> TransfersOfStopIter {
        let stop_data = self.stop_data(stop);
        let nb_of_transfers = stop_data.transfers.len();
        TransfersOfStopIter {
            stop : * stop,
            tranfer_idx_iter : 0..nb_of_transfers
        }
    }

}

pub struct ForwardTimetablesOfStop<'a> {
    transit_data : & 'a TransitData,
    pattern_iter : ForwardPatternsOfStop<'a>, 
    curr_pattern : Option<(StopPattern, Position, TimetablesIter)>, // None when iterator has ended
}

impl<'a> ForwardTimetablesOfStop<'a> {
    pub(super) fn new(transit_data : &'a TransitData, stop : & Stop) -> Self {
        let mut pattern_iter = transit_data.forward_patterns_of(stop);
        let has_first_pattern_idx = pattern_iter.next();
        let curr_pattern = has_first_pattern_idx.map(|(pattern, position)| {
            (*pattern, *position,  transit_data.forward_pattern(&pattern).timetables())
        });
        Self {
            transit_data,
            pattern_iter,
            curr_pattern
        }
    }


}

impl<'a> Iterator for ForwardTimetablesOfStop<'a> {
    type Item = (StopPattern, Position, Timetable);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((pattern, position, timetable_iter)) = & mut self.curr_pattern {
                // if there is still a timetable in this pattern, we return it
                if let Some(timetable) = timetable_iter.next() {
                    return Some((*pattern, *position, timetable));
                }
                else {
                // otherwise, all timetables in the current pattern have been yielded
                    match self.pattern_iter.next() {
                        None => { self.curr_pattern = None;},
                        Some((new_pattern, new_position)) => {
                            let new_timetable_iter = self.transit_data.forward_pattern(&new_pattern).timetables();
                            self.curr_pattern = Some((*new_pattern, *new_position, new_timetable_iter));
                        }
                    }
                }
            }
            else {
                return None;
            }
        }
    }
}

use std::ops::Range;
pub struct TransfersOfStopIter {
    stop : Stop,
    tranfer_idx_iter : Range<usize>
}

impl Iterator for TransfersOfStopIter {
    type Item = Transfer;

    fn next(&mut self) -> Option<Self::Item> {
        self.tranfer_idx_iter.next().map(|idx_in_stop_transfers| {
            Transfer{
                stop : self.stop,
                idx_in_stop_transfers
            }
        })
    }
}