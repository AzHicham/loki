
use super::data::{
    EngineData,
    Stop,
    StopIdx,
    StopPatternIdx,
    Position,
};

use super::time::{ DaysSinceDatasetStart ,SecondsSinceDatasetStart};


use super::ordered_timetable::{TimeTableIdx, VehicleIdx};


impl Stop {

    // Returns 
    // - None if this Stop does not appears in `stop_pattern_idx`
    // - Some(position) otherwise, where `position` is the position of this Stop in the StopPattern
    fn get_position_in_arrival_pattern(&self, stop_pattern_idx: & StopPatternIdx) -> Option<Position> {
        self.position_in_arrival_patterns.iter()
            .find(|&(candidate_stop_pattern_idx, _)| {
                candidate_stop_pattern_idx == stop_pattern_idx
            })
            .map(|&(_, position)| position)
    }
}

impl EngineData {

    pub fn is_upstream_in_arrival_pattern(&self,
            upstream_idx : & StopIdx, 
            downstream_idx : & StopIdx,
            arrival_pattern_idx : & StopPatternIdx 
    ) -> bool {
        let upstream = &self.stops[upstream_idx.idx];
        let dowstream = &self.stops[downstream_idx.idx];

        format!("The stop {:?} is expected to belongs to the stop_pattern {:?}", *upstream_idx, *arrival_pattern_idx);

        let upstream_position = upstream
            .get_position_in_arrival_pattern(arrival_pattern_idx)
            .unwrap_or_else( || panic!(format!("The stop {:?} is expected to belongs to the stop_pattern {:?}", 
                                                    *upstream_idx, 
                                                    *arrival_pattern_idx))
                            );

        let downstream_position = dowstream
            .get_position_in_arrival_pattern(arrival_pattern_idx)
            .unwrap_or_else( || panic!(format!("The stop {:?} is expected to belongs to the stop_pattern {:?}", 
                                                    *upstream_idx, 
                                                    *arrival_pattern_idx))
                            );
        upstream_position.idx < downstream_position.idx

    }

    pub fn next_stop_in_arrival_pattern(&self, 
        stop_idx : & StopIdx,
        stop_pattern_idx : & StopPatternIdx,
    ) -> Option<StopIdx> 
    {
        let stop = &self.stops[stop_idx.idx];
        let position = stop.get_position_in_arrival_pattern(stop_pattern_idx)
            .unwrap_or_else(|| panic!(format!("The stop {:?} is expected to belongs to the stop_pattern {:?}", 
                                                *stop_idx, 
                                                *stop_pattern_idx))
                            );
        let arrival_pattern = &self.arrival_stop_patterns[stop_pattern_idx.idx];
        if position.idx + 1 == arrival_pattern.nb_of_positions() {
            return None;
        }
        debug_assert!(position.idx < arrival_pattern.nb_of_positions() );
        let next_position = Position{ idx : position.idx + 1};
        Some(arrival_pattern.get_stop_at(&next_position))
    }


    pub fn get_best_vehicle_to_board(&self, 
        waiting_time : & SecondsSinceDatasetStart,
        stop_pattern_idx : & StopPatternIdx,
        timetable_idx : & TimeTableIdx
     ) -> Option<(DaysSinceDatasetStart, VehicleIdx)> {

        let timetable = self.arrival_stop_patterns[stop_pattern_idx.idx].get_timetable(timetable_idx);
        let (day, seconds_in_day) = waiting_time.decompose();
        
        // get best vehicle at (day, seconds in day)
        // if timetable goes overmidnight, get best vehicle at (day - 1, seconds in day -1 )
        // compare the two ? the best one is the one which arrives the earliest at the next stop

     }


}