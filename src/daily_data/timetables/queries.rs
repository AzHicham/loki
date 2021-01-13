use crate::time::{SecondsSinceDatasetUTCStart, };

use super::timetables_data::{Timetables, TimetableData, Timetable, Position, VehicleData, Vehicle};

impl Timetables {
    pub fn best_vehicle_to_board(&self, 
            waiting_time : & SecondsSinceDatasetUTCStart , 
            timetable : & Timetable, 
            position : & Position,
        ) -> Option<(Vehicle, SecondsSinceDatasetUTCStart)> {
        assert!(*timetable == position.timetable);
        self.timetable_data(timetable)
            .best_vehicle_to_board(waiting_time, position.idx)
            .map(|(vehicle_idx, arrival_time_at_next_position) | {
                let vehicle = Vehicle {
                    timetable : timetable.clone(),
                    idx : vehicle_idx
                };
                (vehicle, arrival_time_at_next_position)
            })
    }
}

impl TimetableData {
 

    fn best_vehicle_to_board(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        position_idx: usize,
    ) -> Option<(usize, SecondsSinceDatasetUTCStart)> {
        //TODO : reread this and look for optimization

        let next_position_idx = position_idx + 1;
        // if we are at the last position, we cannot board
        if next_position_idx >= self.stop_flows.len() {
            return None;
        };

        let has_best_vehicle_idx = self.best_filtered_vehicle_to_board_at(waiting_time, position_idx, |_| true);
        has_best_vehicle_idx.map(|vehicle_idx| {
            let vehicle_arrival_time_in_day_at_next_stop = self.arrival_time_at(vehicle_idx, next_position_idx).clone();
            (vehicle_idx, vehicle_arrival_time_in_day_at_next_stop)
        })
        
    }

   
    // If we are waiting to board a vehicle at `position` at time `waiting_time`
    // return `Some(best_vehicle_idx)`
    // where `best_vehicle_idx` is the idx of the vehicle, among those vehicle on which `filter` returns true,
    //  to board that allows to debark at the subsequent positions at the earliest time,
    fn best_filtered_vehicle_to_board_at<Filter>(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
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


