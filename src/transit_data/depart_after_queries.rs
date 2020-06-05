
use super::data::{
    EngineData,
    Stop,
    StopIdx,
    StopPatternIdx,
    Position,
    VehicleData,
};

use super::time::{ DaysSinceDatasetStart ,SecondsSinceDatasetStart, SecondsSinceDayStart};


use super::ordered_timetable::{TimeTableIdx, VehicleIdx, OrderedTimetable};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ForwardMission {
    stop_pattern : StopPatternIdx,
    timetable : TimeTableIdx,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ForwardTrip {
    mission : ForwardMission,
    vehicle : VehicleIdx,
    day : DaysSinceDatasetStart,
}

impl Stop {

    // Returns 
    // - None if this Stop does not appears in `stop_pattern_idx`
    // - Some(position) otherwise, where `position` is the position of this Stop in the StopPattern
    fn position_in_arrival_pattern(&self, stop_pattern_idx: & StopPatternIdx) -> Option<Position> {
        self.position_in_arrival_patterns.iter()
            .find(|&(candidate_stop_pattern_idx, _)| {
                candidate_stop_pattern_idx == stop_pattern_idx
            })
            .map(|&(_, position)| position)
    }
}

impl EngineData {


    pub fn is_upstream_in_forward_mission(&self,
        upstream_idx : & StopIdx,
        downstream_idx : & StopIdx,
        mission : & ForwardMission,
    ) -> bool {
        self.is_upstream_in_arrival_pattern(upstream_idx, downstream_idx, &mission.stop_pattern)
    }

    fn is_upstream_in_arrival_pattern(&self,
            upstream_idx : & StopIdx, 
            downstream_idx : & StopIdx,
            arrival_pattern_idx : & StopPatternIdx 
    ) -> bool {
        let upstream = &self.stops[upstream_idx.idx];
        let dowstream = &self.stops[downstream_idx.idx];

        format!("The stop {:?} is expected to belongs to the stop_pattern {:?}", *upstream_idx, *arrival_pattern_idx);

        let upstream_position = upstream
            .position_in_arrival_pattern(arrival_pattern_idx)
            .unwrap_or_else( || panic!(format!("The stop {:?} is expected to belongs to the stop_pattern {:?}", 
                                                    *upstream_idx, 
                                                    *arrival_pattern_idx))
                            );

        let downstream_position = dowstream
            .position_in_arrival_pattern(arrival_pattern_idx)
            .unwrap_or_else( || panic!(format!("The stop {:?} is expected to belongs to the stop_pattern {:?}", 
                                                    *upstream_idx, 
                                                    *arrival_pattern_idx))
                            );
        upstream_position.idx < downstream_position.idx

    }

    pub fn next_stop_in_forward_mission(&self,
        stop_idx : & StopIdx,
        mission : & ForwardMission,
    ) -> Option<StopIdx> 
    {
        self.next_stop_in_arrival_pattern(stop_idx, &mission.stop_pattern)
    }

    fn next_stop_in_arrival_pattern(&self, 
        stop_idx : & StopIdx,
        stop_pattern_idx : & StopPatternIdx,
    ) -> Option<StopIdx> 
    {
        let stop = &self.stops[stop_idx.idx];
        let position = stop.position_in_arrival_pattern(stop_pattern_idx)
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


    // pub fn boardable_forward_missions<'a>(& 'a self, stop_idx : & StopIdx) -> impl Iterator<Item = ForwardMission> + 'a {
    //     let stop = &self.stops[stop_idx.idx];
    //     stop.position_in_arrival_patterns.iter()
    //         .flat_map(move |(stop_pattern_idx, _)| {
    //             let stop_pattern = & self.arrival_stop_patterns[stop_pattern_idx.idx];
    //             let timetables = stop_pattern.timetables();
    //             timetables.map(move |timetable_idx| {
    //                 ForwardMission {
    //                     stop_pattern : stop_pattern_idx.clone(),
    //                     timetable : timetable_idx.clone()
    //                 }
    //             })
    //         })
 
    // }

    pub fn forward_mission_of(&self, forward_trip : & ForwardTrip) -> ForwardMission {
        forward_trip.mission.clone()
    }

    pub fn best_vehicle_to_board(&self, 
        waiting_time : & SecondsSinceDatasetStart,
        stop_pattern_idx : & StopPatternIdx,
        timetable_idx : & TimeTableIdx,
        position : & Position
     ) -> Option<(DaysSinceDatasetStart, VehicleIdx)> 
     {


        //TODO : reread this and look for optimization

        let stop_pattern = & self.arrival_stop_patterns[stop_pattern_idx.idx];

        // we should never try to board a stop pattern at its last position
        debug_assert!(position.idx < stop_pattern.nb_of_positions() - 1 );
        let next_position = Position {
            idx : position.idx + 1
        };

        let timetable = self.arrival_stop_patterns[stop_pattern_idx.idx].get_timetable(timetable_idx);

        

        let has_latest_board_time = timetable.last_board_time_at(position);
        if has_latest_board_time.is_none() {
            return None;
        }
        let latest_board_time_in_day = has_latest_board_time.clone().unwrap();

        let mut nb_of_days_to_offset = 0u16;
        let (mut waiting_day, mut waiting_time_in_day) = waiting_time.decompose();
        let mut best_vehicle_day_and_its_debark_time_at_next_stop : Option<(VehicleIdx, DaysSinceDatasetStart, SecondsSinceDatasetStart)> = None;

        
        while waiting_time_in_day <= latest_board_time_in_day {
            
            let has_vehicle  = self.best_vehicle_to_board_in_day(&waiting_day, 
                &waiting_time_in_day, 
                timetable, 
                position
            );
            if let Some(vehicle) = has_vehicle {
                let vehicle_debark_time_in_day_at_next_stop = timetable.debark_time_at(&vehicle, &next_position);
                let vehicle_debark_time_at_next_stop = SecondsSinceDatasetStart::compose(&waiting_day, vehicle_debark_time_in_day_at_next_stop);
                if let Some((_, _, best_debark_time)) = & best_vehicle_day_and_its_debark_time_at_next_stop {
                    if vehicle_debark_time_at_next_stop < *best_debark_time {
                        best_vehicle_day_and_its_debark_time_at_next_stop = Some((vehicle, waiting_day, vehicle_debark_time_at_next_stop));
                    }
                }
                else {
                    best_vehicle_day_and_its_debark_time_at_next_stop = Some((vehicle, waiting_day, vehicle_debark_time_at_next_stop));
                }

            }
            nb_of_days_to_offset += 1;
            let has_prev_day = waiting_time.decompose_with_days_offset(nb_of_days_to_offset);
            if let Some((day, time_in_day)) = has_prev_day {
                waiting_day = day;
                waiting_time_in_day = time_in_day;
            }
            else {
                break;
            }
        }

        best_vehicle_day_and_its_debark_time_at_next_stop.map(|(vehicle, day, _)| (day, vehicle))
       
    }

    fn best_vehicle_to_board_in_day(&self, 
        day : & DaysSinceDatasetStart,
        time_in_day : & SecondsSinceDayStart,
        timetable : & OrderedTimetable<VehicleData, SecondsSinceDayStart>,
        position : & Position,
    ) -> Option<VehicleIdx>
    {
        timetable.best_filtered_vehicle_to_board_at(time_in_day, position, |vehicle_data| {
            let calendar_idx = vehicle_data.calendar_idx;
            self.calendars.is_allowed(&calendar_idx, day)
        })

    }


}

// pub struct ForwardMissionsIter<'a> {
//     engine_data : & 'a EngineData,
//     current_pattern_idx : usize, // set to engine_data.arrival_stop_patterns.len() when the iterator is exhausted
//     next_timetable_idx : usize, // set to be >= pattern.timetables.len() when the current pattern is exhausted
// }

// impl<'a> ForwardMissionsIter<'a> {
//     pub fn new(engine_data : &'a EngineData) -> Self {
//         Self {
//             engine_data,
//             current_pattern_idx : 0,  
//             next_timetable_idx : 0,
//         }
//     }
// }

// impl<'a> Iterator for ForwardMissionsIter<'a> {
//     type Item = ForwardMission;

//     fn next(&mut self) -> Option<Self::Item> {
//         let has_current_pattern = self.engine_data.arrival_stop_patterns.get(self.current_pattern_idx);
//         if let Some(pattern) = has_current_pattern {
//             if self.next_timetable_idx < pattern.timetables.len()  {
//                 let timetable =  TimeTableIdx {
//                     idx : self.next_timetable_idx
//                 };
//                 self.next_timetable_idx += 1;
//                 let stop_pattern = StopPatternIdx {
//                     idx : self.current_pattern_idx
//                 };
//                 let result = ForwardMission {
//                     stop_pattern,
//                     timetable,
//                 };
//                 return Some(result);
//             }
//             else {
//                 while self.current_pattern_idx  < self.engine_data.arrival_stop_patterns.len() {
//                     self.current_pattern_idx += 1;
//                     let pattern = &self.engine_data.arrival_stop_patterns[self.current_pattern_idx];
//                     if pattern.timetables.len() > 0 {
//                         self.next_timetable_idx = 1;
//                         let timetable = TimeTableIdx {
//                             idx : 0
//                         };
//                         let stop_pattern = StopPatternIdx {
//                             idx : self.current_pattern_idx,
//                         };
//                         let result = ForwardMission {
//                             stop_pattern,
//                             timetable,
//                         };
//                         return Some(result);
//                     }

//                 }
//                 return None;
//             }
//         }
//         else {
//             return None;
//         }
//     }
// }
