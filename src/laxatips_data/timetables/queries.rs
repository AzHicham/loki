use crate::laxatips_data::calendar::Calendar;

use std::cmp::Ordering;


use crate::laxatips_data::time::{ SecondsSinceTimezonedDayStart, SecondsSinceDatasetUTCStart, DaysSinceDatasetStart};

use super::timetables_data::{Timetables, TimetableData, Timetable, Position, VehicleData, Vehicle};

impl Timetables {
    pub fn best_vehicle_to_board(&self, 
            waiting_time : & SecondsSinceDatasetUTCStart , 
            timetable : & Timetable, 
            position : & Position,
            calendar : & Calendar,
        ) -> Option<(Vehicle, DaysSinceDatasetStart, SecondsSinceDatasetUTCStart)> {
        assert!(*timetable == position.timetable);
        self.timetable_data(timetable)
            .best_vehicle_to_board(waiting_time, calendar, position.idx)
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
        calendar : & Calendar,
        position_idx: usize,
    ) -> Option<(usize, DaysSinceDatasetStart, SecondsSinceDatasetUTCStart)> {
        //TODO : reread this and look for optimization

        let next_position_idx = position_idx + 1;
        // if we are at the last position, we cannot board
        if next_position_idx >= self.stop_flows.len() {
            return None;
        };
        

        let has_latest_board_time = self.latest_board_time_at(position_idx);

        // if there is no latest board time, it means that this position cannot be boarded
        // and we return None
        let latest_board_time_in_day = has_latest_board_time?;

        let mut nb_of_days_to_offset = 0u16;
        let (mut waiting_day, mut waiting_time_in_day) = waiting_time.decompose(calendar, &self.timezone);
        let mut best_vehicle_day_and_its_arrival_time_at_next_position: Option<(
            usize, // vehicle_idx
            DaysSinceDatasetStart,
            SecondsSinceDatasetUTCStart,
        )> = None;

        while waiting_time_in_day <= *latest_board_time_in_day {
            let has_vehicle = self.best_vehicle_to_board_in_day(
                &waiting_day,
                &waiting_time_in_day,
                calendar,
                position_idx,
            );
            if let Some(vehicle) = has_vehicle {
                let vehicle_arrival_time_in_day_at_next_stop =
                    self.arrival_time_at(vehicle, next_position_idx);
                let vehicle_arrival_time_at_next_stop = SecondsSinceDatasetUTCStart::compose(
                    &waiting_day,
                    vehicle_arrival_time_in_day_at_next_stop,
                    calendar,
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
            nb_of_days_to_offset += 1;
            let has_prev_day = waiting_time.decompose_with_days_offset(nb_of_days_to_offset, calendar, &self.timezone);
            if let Some((day, time_in_day)) = has_prev_day {
                waiting_day = day;
                waiting_time_in_day = time_in_day;
            } else {
                break;
            }
        }

        best_vehicle_day_and_its_arrival_time_at_next_position
    }

    fn best_vehicle_to_board_in_day(
        &self,
        day: &DaysSinceDatasetStart,
        time_in_day: &SecondsSinceTimezonedDayStart,
        calendar : & Calendar,
        position_idx: usize
    ) -> Option<usize> {
        self.best_filtered_vehicle_to_board_at(
            time_in_day,
            position_idx,
            |vehicle_data| {
                let days_pattern = vehicle_data.days_pattern;
                calendar.is_allowed(&days_pattern, day)
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


