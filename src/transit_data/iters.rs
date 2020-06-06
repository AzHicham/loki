
use super::data::{
    EngineData,
    Stop,
    StopIdx,
    StopPatternIdx,
    Position,
    VehicleData,
    TransferIdx,
};

use super::time::{ DaysSinceDatasetStart ,SecondsSinceDatasetStart, SecondsSinceDayStart};


use super::ordered_timetable::{TimeTableIdx, VehicleIdx, OrderedTimetable, TimeTablesIter};

use std::collections::btree_map::Keys;

type ArrivalPatternsOfStop<'a> = Keys<'a, StopPatternIdx, Position>;

impl EngineData {
    pub fn arrival_patterns_of<'a>(&'a self, stop_idx : & StopIdx) -> ArrivalPatternsOfStop<'a> {
        let stop = self.stop(stop_idx);
        stop.position_in_arrival_patterns.keys()
    }

    pub fn arrival_pattern_and_timetables_of<'a>(&'a self, stop_idx : & StopIdx) -> ArrivalTimetablesOfStop<'a> {
        ArrivalTimetablesOfStop::new(&self, stop_idx)
    }

    pub fn transfers_of(& self, stop_idx : & StopIdx) -> TransfersOfStopIter {
        let stop = self.stop(stop_idx);
        let nb_of_transfers = stop.transfers.len();
        TransfersOfStopIter {
            stop_idx : * stop_idx,
            tranfer_idx_iter : 0..nb_of_transfers
        }
    }

}

pub struct ArrivalTimetablesOfStop<'a> {
    engine_data : & 'a EngineData,
    pattern_iter : ArrivalPatternsOfStop<'a>, 
    curr_pattern : Option<(StopPatternIdx, TimeTablesIter)>, // None when iterator has ended
}

impl<'a> ArrivalTimetablesOfStop<'a> {
    pub(super) fn new(engine_data : &'a EngineData, stop_idx : & StopIdx) -> Self {
        let mut pattern_iter = engine_data.arrival_patterns_of(stop_idx);
        let has_first_pattern_idx = pattern_iter.next();
        let curr_pattern = has_first_pattern_idx.map(|pattern_idx| {
            (*pattern_idx, engine_data.arrival_pattern(&pattern_idx).timetables())
        });
        Self {
            engine_data,
            pattern_iter,
            curr_pattern
        }
    }


}

impl<'a> Iterator for ArrivalTimetablesOfStop<'a> {
    type Item = (StopPatternIdx, TimeTableIdx);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((pattern_idx, timetable_iter)) = & mut self.curr_pattern {
            // if there is still a timetable in this pattern, we return it
            if let Some(timetable_idx) = timetable_iter.next() {
                Some((*pattern_idx, timetable_idx))
            }
            // otherwise, all timetables in the current pattern have been yielded
            else {
                loop {
                    // if there is a new pattern to look at
                    if let Some(new_pattern_idx) = self.pattern_iter.next() {
                        let mut new_timetable_iter = self.engine_data.arrival_pattern(&new_pattern_idx).timetables();
                        // and if the new pattern has at least one timetable
                        // we yield this (new_pattern, timetable)
                        // and we mark the new_pattern as the current one
                        if let Some(next_timetable) = new_timetable_iter.next() {
                            self.curr_pattern = Some((*new_pattern_idx, new_timetable_iter));
                            return Some((*new_pattern_idx, next_timetable));
                        }
                        // if the new_pattern has no timetable,
                        // we loop to the next pattern
                    }
                    // no new pattern to look at
                    // we mark the iterator as exhauster
                    else {
                        self.curr_pattern = None;
                        return None;
                    }
                }
            }
        }
        else {
            None
        }
    }
}

use std::ops::Range;
pub struct TransfersOfStopIter {
    stop_idx : StopIdx,
    tranfer_idx_iter : Range<usize>
}

impl Iterator for TransfersOfStopIter {
    type Item = TransferIdx;

    fn next(&mut self) -> Option<Self::Item> {
        self.tranfer_idx_iter.next().map(|idx_in_stop_transfers| {
            TransferIdx{
                stop_idx : self.stop_idx,
                idx_in_stop_transfers
            }
        })
    }
}