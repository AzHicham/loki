
use super::data::{
    EngineData,
    Stop,
    StopIdx,
    StopPatternIdx,
    Position,
    VehicleData,
};

use super::iters::{ArrivalTimetablesOfStop};

use super::time::{ DaysSinceDatasetStart ,SecondsSinceDatasetStart, SecondsSinceDayStart};


use super::ordered_timetable::{TimeTableIdx, VehicleIdx, OrderedTimetable};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ForwardMission {
    pub stop_pattern : StopPatternIdx,
    pub timetable : TimeTableIdx,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ForwardTrip {
    pub mission : ForwardMission,
    pub vehicle : VehicleIdx,
    pub day : DaysSinceDatasetStart,
}

impl Stop {

    // Returns 
    // - None if this Stop does not appears in `stop_pattern_idx`
    // - Some(position) otherwise, where `position` is the position of this Stop in the StopPattern
    fn position_in_arrival_pattern(&self, stop_pattern_idx: & StopPatternIdx) -> Option<&Position> {
        self.position_in_arrival_patterns.get(stop_pattern_idx)
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


    pub fn boardable_forward_missions<'a>(& 'a self, 
        stop_idx : & StopIdx
    ) -> ForwardMissionsOfStop
    {
        let inner = self.arrival_pattern_and_timetables_of(stop_idx);
        ForwardMissionsOfStop {
            inner
        }
    }

    pub fn forward_mission_of(&self, forward_trip : & ForwardTrip) -> ForwardMission {
        forward_trip.mission.clone()
    }

    // Panics if `trip` does not go through `stop_idx` 
    pub fn arrival_time_of(&self, trip : & ForwardTrip, stop_idx : & StopIdx) -> SecondsSinceDatasetStart {
        let pattern_idx = &trip.mission.stop_pattern;
        let timetable_idx = &trip.mission.timetable;
        let timetable = self.arrival_pattern(pattern_idx).get_timetable(timetable_idx);
        let position = self.stop(stop_idx).position_in_arrival_pattern(pattern_idx).unwrap();
        let vehicle_idx = & trip.vehicle;
        let seconds_in_day = timetable.debark_time_at(vehicle_idx, position);
        let days = &trip.day;
        SecondsSinceDatasetStart::compose(days, seconds_in_day)
    }

    // Panics if `trip` does not go through `stop_idx` 
    // None if `trip` does not allows boarding at `stop_idx`
    pub fn departure_time_of(&self, trip : & ForwardTrip, stop_idx : & StopIdx) -> Option<SecondsSinceDatasetStart> {
        let pattern_idx = &trip.mission.stop_pattern;
        let timetable_idx = &trip.mission.timetable;
        let timetable = self.arrival_pattern(pattern_idx).get_timetable(timetable_idx);
        let position = self.stop(stop_idx).position_in_arrival_pattern(pattern_idx).unwrap();
        let vehicle_idx = & trip.vehicle;
        let has_seconds_in_day = timetable.board_time_at(vehicle_idx, position);
        has_seconds_in_day.map(|seconds_in_day| {
            let days = &trip.day;
            SecondsSinceDatasetStart::compose(days, &seconds_in_day)
        })
        
    }


    pub fn best_trip_to_board_at_stop(&self,
        waiting_time : & SecondsSinceDatasetStart,
        mission : & ForwardMission,
        stop_idx : & StopIdx
     ) -> Option<(ForwardTrip, SecondsSinceDatasetStart)> 
     {
        let stop_pattern_idx = &mission.stop_pattern;
        let timetable_idx = &mission.timetable;
        let position = self.stop(stop_idx).position_in_arrival_pattern(stop_pattern_idx).unwrap();
        self.best_vehicle_to_board(waiting_time, stop_pattern_idx, timetable_idx, position)
            .map(|(day, vehicle, arrival_time)| {
                let trip = ForwardTrip {
                    mission : * mission,
                    day,
                    vehicle,               
                };
                (trip, arrival_time)
            })

     }

    fn best_vehicle_to_board(&self, 
        waiting_time : & SecondsSinceDatasetStart,
        stop_pattern_idx : & StopPatternIdx,
        timetable_idx : & TimeTableIdx,
        position : & Position
     ) -> Option<(DaysSinceDatasetStart, VehicleIdx, SecondsSinceDatasetStart)> 
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

        best_vehicle_day_and_its_debark_time_at_next_stop
       
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


pub struct ForwardMissionsOfStop<'a> {
    inner : ArrivalTimetablesOfStop<'a>
}

impl<'a> Iterator for ForwardMissionsOfStop<'a> {
    type Item = ForwardMission;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(pattern, timetable)| {
            ForwardMission{
                stop_pattern : pattern,
                timetable
            }
        })
    }
}