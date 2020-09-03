use crate::laxatips_data::days_patterns::DaysPatterns;
use crate::laxatips_data::time::{Calendar, SecondsSinceTimezonedDayStart, SecondsSinceDatasetUTCStart, DaysSinceDatasetStart};

use super::timetables_data::{Timetables, TimetableData, Timetable, Position, VehicleData, Vehicle};

impl Timetables {
    pub fn best_vehicle_to_board(&self, 
            waiting_time : & SecondsSinceDatasetUTCStart , 
            timetable : & Timetable, 
            position : & Position,
            calendar : & Calendar,
            days_patterns : &  DaysPatterns,
        ) -> Option<(Vehicle, DaysSinceDatasetStart, SecondsSinceDatasetUTCStart)> {
        assert!(*timetable == position.timetable);
        self.timetable_data(timetable)
            .best_vehicle_to_board(waiting_time, position.idx, calendar, days_patterns)
            .map(|(vehicle_idx, days, arrival_time_at_next_position) | {
                let vehicle = Vehicle {
                    timetable : timetable.clone(),
                    idx : vehicle_idx
                };
                (vehicle, days, arrival_time_at_next_position)
            })
    }
}

impl TimetableData {
 

    fn best_vehicle_to_board(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        position_idx: usize,
        calendar : & Calendar,
        days_patterns : & DaysPatterns,
    ) -> Option<(usize, DaysSinceDatasetStart, SecondsSinceDatasetUTCStart)> {
        //TODO : reread this and look for optimization

        let next_position_idx = position_idx + 1;
        // if we are at the last position, we cannot board
        if next_position_idx >= self.stop_flows.len() {
            return None;
        };
        

        let has_earliest_and_latest_board_time = self.earliest_and_latest_board_time_at(position_idx);

        // if there is no earliest/latest board time, it means that this position cannot be boarded
        // and we return None
        let (earliest_board_time_in_day, latest_board_time_in_day) = has_earliest_and_latest_board_time?;

        let decompositions = calendar.decompositions(waiting_time, 
            &self.timezone, 
            *latest_board_time_in_day, 
            *earliest_board_time_in_day
        );
        let mut best_vehicle_day_and_its_arrival_time_at_next_position: Option<(
            usize, // vehicle_idx
            DaysSinceDatasetStart,
            SecondsSinceDatasetUTCStart,
        )> = None;
        for (waiting_day, waiting_time_in_day) in decompositions {
            let has_vehicle = self.best_vehicle_to_board_in_day(
                &waiting_day, 
                &waiting_time_in_day, 
                position_idx, 
                days_patterns,
            );
            if let Some(vehicle) = has_vehicle {
                let vehicle_arrival_time_in_day_at_next_stop = self.arrival_time_at(vehicle, next_position_idx);
                let vehicle_arrival_time_at_next_stop = calendar.compose(
                    &waiting_day,
                    vehicle_arrival_time_in_day_at_next_stop,
                    &self.timezone
                );
                if let Some((_, _, best_arrival_time)) =
                    &best_vehicle_day_and_its_arrival_time_at_next_position
                {
                    if vehicle_arrival_time_at_next_stop < *best_arrival_time {
                        best_vehicle_day_and_its_arrival_time_at_next_position =
                            Some((vehicle, waiting_day, vehicle_arrival_time_at_next_stop));
                    }
                } else {
                    best_vehicle_day_and_its_arrival_time_at_next_position =
                        Some((vehicle, waiting_day, vehicle_arrival_time_at_next_stop));
                }
            }
        }

        best_vehicle_day_and_its_arrival_time_at_next_position
    }

    fn best_vehicle_to_board_in_day(
        &self,
        day: &DaysSinceDatasetStart,
        time_in_day: &SecondsSinceTimezonedDayStart,
        position_idx: usize,
        days_patterns : & DaysPatterns
    ) -> Option<usize> {
        self.best_filtered_vehicle_to_board_at(
            time_in_day,
            position_idx,
            |vehicle_data| {
                let days_pattern = vehicle_data.days_pattern;
                days_patterns.is_allowed(&days_pattern, day)
            },
        )
    }


    // If we are waiting to board a vehicle at `position` at time `waiting_time`
    // return `Some(best_vehicle)_idx`
    // where `best_vehicle_idx` is the idx of the vehicle, among those vehicle on which `filter` returns true,
    //  to board that allows to debark at the subsequent positions at the earliest time,
    fn _best_filtered_vehicle_to_board_at_by_linear_search<Filter>(
        &self,
        waiting_time: &SecondsSinceTimezonedDayStart,
        position: &Position,
        filter: Filter,
    ) -> Option<usize>
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
                    Some(idx)
                } else {
                    None
                }
            })
    }

    // If we are waiting to board a vehicle at `position` at time `waiting_time`
    // return `Some(best_vehicle_idx)`
    // where `best_vehicle_idx` is the idx of the vehicle, among those vehicle on which `filter` returns true,
    //  to board that allows to debark at the subsequent positions at the earliest time,
    fn best_filtered_vehicle_to_board_at<Filter>(
        &self,
        waiting_time: &SecondsSinceTimezonedDayStart,
        position_idx: usize,
        filter: Filter,
    ) -> Option<usize>
    where
        Filter: Fn(&VehicleData) -> bool,
    {
        let search_result = self.board_times_by_position[position_idx].binary_search(waiting_time);
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
                    && self.board_times_by_position[position_idx][first_idx] == *waiting_time
                {
                    first_idx -=  1;
                }
                first_idx
            }
        };

        for vehicle_idx in first_boardable_vehicle..self.nb_of_vehicles() {
            let vehicle_data = &self.vehicles_data[vehicle_idx];
            let board_time = &self.board_times_by_position[position_idx][vehicle_idx];
            if filter(vehicle_data) && waiting_time <= board_time {
                return Some(vehicle_idx);
            }
        }
        None
    }

    
}


