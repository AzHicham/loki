// Copyright  (C) 2020, Kisio Digital and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Kisio Digital (www.kisio.com).
// Help us simplify mobility and open public transport:
// a non ending quest to the responsive locomotion way of traveling!
//
// This contribution is a part of the research and development work of the
// IVA Project which aims to enhance traveler information and is carried out
// under the leadership of the Technological Research Institute SystemX,
// with the partnership and support of the transport organization authority
// Ile-De-France Mobilités (IDFM), SNCF, and public funds
// under the scope of the French Program "Investissements d’Avenir".
//
// LICENCE: This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// Stay tuned using
// twitter @navitia
// channel `#navitia` on riot https://riot.im/app/#/room/#navitia:matrix.org
// https://groups.google.com/d/forum/navitia
// www.navitia.io

use std::{borrow::Borrow, cmp::Ordering, fmt::Debug, ops::Not};
use FlowDirection::{BoardAndDebark, BoardOnly, DebarkOnly, NoBoardDebark};

use crate::{
    timetables::{FlowDirection, StopFlows},
    transit_data::Stop,
};
use std::cmp::Ordering::{Greater, Less};

use super::generic_timetables::TimetableData;

impl<Time, Load, VehicleData> TimetableData<Time, Load, VehicleData>
where
    Time: Ord + Clone + Debug,
    Load: Ord + Debug,
{
    pub(super) fn can_board(&self, position_idx: usize) -> bool {
        match &self.stop_flows[position_idx].1 {
            BoardAndDebark | BoardOnly => true,
            NoBoardDebark | DebarkOnly => false,
        }
    }

    pub(super) fn can_debark(&self, position_idx: usize) -> bool {
        match &self.stop_flows[position_idx].1 {
            BoardAndDebark | DebarkOnly => true,
            NoBoardDebark | BoardOnly => false,
        }
    }

    pub(super) fn arrival_time(&self, vehicle_idx: usize, position_idx: usize) -> &Time {
        &self.debark_times_by_position[position_idx][vehicle_idx]
    }

    pub(super) fn departure_time(&self, vehicle_idx: usize, position_idx: usize) -> &Time {
        &self.board_times_by_position[position_idx][vehicle_idx]
    }

    pub(super) fn debark_time(&self, vehicle_idx: usize, position_idx: usize) -> Option<&Time> {
        if self.can_debark(position_idx) {
            Some(&self.debark_times_by_position[position_idx][vehicle_idx])
        } else {
            None
        }
    }

    pub(super) fn board_time(&self, vehicle_idx: usize, position_idx: usize) -> Option<&Time> {
        if self.can_board(position_idx) {
            Some(&self.board_times_by_position[position_idx][vehicle_idx])
        } else {
            None
        }
    }

    pub(super) fn load_after(&self, vehicle_idx: usize, position_idx: usize) -> &Load {
        assert!(position_idx + 1 < self.nb_of_positions());
        &self.vehicle_loads[vehicle_idx][position_idx]
    }

    pub(super) fn load_before(&self, vehicle_idx: usize, position_idx: usize) -> &Load {
        assert!(position_idx > 0);
        &self.vehicle_loads[vehicle_idx][position_idx - 1]
    }

    pub(super) fn stop_at(&self, position_idx: usize) -> &Stop {
        &self.stop_flows[position_idx].0
    }

    pub(super) fn nb_of_positions(&self) -> usize {
        self.board_times_by_position.len()
    }

    pub(super) fn nb_of_vehicle(&self) -> usize {
        self.vehicle_datas.len()
    }

    pub(super) fn vehicle_data(&self, vehicle_idx: usize) -> &VehicleData {
        &self.vehicle_datas[vehicle_idx]
    }

    // If we are waiting to board a trip at `position` at time ``
    // return `Some(best_vehicle_idx)`
    // where `best_vehicle_idx` is the idx of the first Vehicle that can be boarded
    //  after waiting_time
    pub(super) fn earliest_vehicle_to_board(
        &self,
        waiting_time: &Time,
        position_idx: usize,
    ) -> Option<usize> {
        if !self.can_board(position_idx) {
            return None;
        }

        let nb_of_vehicles = self.board_times_by_position[position_idx].len();
        if nb_of_vehicles == 0 {
            return None;
        }

        let last_vehicle_idx = nb_of_vehicles - 1; // substraction is safe since we checked that nb_of_vehicles > 0
        if waiting_time > &self.board_times_by_position[position_idx][last_vehicle_idx] {
            return None;
        }

        let first_boardable_vehicle =
            if waiting_time <= &self.board_times_by_position[position_idx][0] {
                0
            } else {
                // We are looking for the smallest index in slice (board_times_by_position here)
                // such that slice(idx) >= waiting_time.
                // In order to do so we use binary_search_by with the comparator
                // function F : |time| if time < waiting_time { Less } else { Greater }
                // binary_search_by on slice with a comparator function F will return :
                // - Ok(idx) if there a idx such that F(slice(idx)) == Equal
                // - Err(idx) otherwise. In this case it means that F(slice(idx)) == Greater,
                // and F(slice(idx-1)) == Less if idx >= 1
                // Since our comparator will never return Equal,
                // binary_search_by will always return Err(idx).
                // So when we obtain Err(idx) it means that slice(idx) >= waiting_time
                // And slice(idx-1) < waiting_time
                // So idx is the smallest index such that slice(idx) >= waiting_time
                self.board_times_by_position[position_idx]
                    .binary_search_by(|time| if time < waiting_time { Less } else { Greater })
                    .unwrap_err()
            };

        Some(first_boardable_vehicle)
    }

    pub(super) fn earliest_filtered_vehicle_to_board<Filter>(
        &self,
        waiting_time: &Time,
        position_idx: usize,
        filter: &Filter,
    ) -> Option<usize>
    where
        Filter: Fn(&VehicleData) -> bool,
    {
        let first_boardable_vehicle = self.earliest_vehicle_to_board(waiting_time, position_idx)?;

        for vehicle_idx in first_boardable_vehicle..self.nb_of_vehicle() {
            let vehicle_data = &self.vehicle_datas[vehicle_idx];
            let board_time = &self.board_times_by_position[position_idx][vehicle_idx];
            if filter(vehicle_data) && waiting_time <= board_time {
                return Some(vehicle_idx);
            }
        }
        None
    }

    // If we are waiting to board a trip at `position` at time ``
    // return `Some(best_vehicle_idx)`
    // where `best_vehicle_idx` is the idx of the first Vehicle that can be debarked
    //  after waiting_time
    pub(super) fn earliest_vehicle_to_debark(
        &self,
        waiting_time: &Time,
        position_idx: usize,
    ) -> Option<usize> {
        if !self.can_debark(position_idx) {
            return None;
        }

        let nb_of_vehicles = self.board_times_by_position[position_idx].len();
        if nb_of_vehicles == 0 {
            return None;
        }

        let last_vehicle_idx = nb_of_vehicles - 1; // substraction is safe since we checked that nb_of_vehicles > 0
        if waiting_time > &self.debark_times_by_position[position_idx][last_vehicle_idx] {
            return None;
        }

        let first_vehicle = if waiting_time <= &self.debark_times_by_position[position_idx][0] {
            0
        } else {
            // We are looking for the smallest index in slice (debark_times_by_position here)
            // such that slice(idx) >= waiting_time.
            // In order to do so we use binary_search_by with the comparator
            // function F : |time| if time < waiting_time { Less } else { Greater }
            // binary_search_by on slice with a comparator function F will return :
            // - Ok(idx) if there a idx such that F(slice(idx)) == Equal
            // - Err(idx) otherwise. In this case it means that F(slice(idx)) == Greater,
            // and F(slice(idx-1)) == Less if idx >= 1
            // Since our comparator will never return Equal,
            // binary_search_by will always return Err(idx).
            // So when we obtain Err(idx) it means that slice(idx) >= waiting_time
            // And slice(idx-1) < waiting_time
            // So idx is the smallest index such that slice(idx) >= waiting_time
            self.debark_times_by_position[position_idx]
                .binary_search_by(|time| if time < waiting_time { Less } else { Greater })
                .unwrap_err()
        };

        Some(first_vehicle)
    }

    // Given a `position` and a `time`
    // return `Some(best_trip_idx)`
    // where `best_trip_idx` is the idx of the trip, among those trip on which `filter` returns true,
    // that debark at the subsequent positions at the latest time
    pub(super) fn latest_filtered_vehicle_that_debark<Filter>(
        &self,
        waiting_time: &Time,
        position_idx: usize,
        filter: Filter,
    ) -> Option<(usize, &Time)>
    where
        Filter: Fn(&VehicleData) -> bool,
    {
        if !self.can_debark(position_idx) {
            return None;
        }
        // we should not be able to debark at the first position
        assert!(position_idx > 0);

        let nb_of_vehicles = self.debark_times_by_position[position_idx].len();
        if nb_of_vehicles == 0 {
            return None;
        }

        let last_vehicle_idx = nb_of_vehicles - 1; // substraction is safe since we checked that nb_of_vehicles > 0

        if waiting_time < &self.debark_times_by_position[position_idx][0] {
            return None;
        }

        let after_last_debarkable_vehicle =
            if waiting_time > &self.debark_times_by_position[position_idx][last_vehicle_idx] {
                last_vehicle_idx + 1
            } else {
                // We are looking for the greatest index in slice (debark_times_by_position here)
                // such that slice(idx) <= waiting_time.
                // In order to do so we use binary_search_by with the comparator
                // function F : |time| if time <= waiting_time { Less } else { Greater }
                // binary_search_by on slice with a comparator function F will return :
                // - Ok(idx) if there a idx such that F(slice(idx)) == Equal
                // - Err(idx) otherwise. In this case it means that F(slice(idx)) == Greater,
                // and F(slice(idx-1)) == Less if idx >= 1
                // Since our comparator will never return Equal,
                // binary_search_by will always return Err(idx).
                // So when we obtain Err(idx) it means that slice(idx) > waiting_time
                // And slice(idx-1) <= waiting_time
                // So idx-1 is the greatest index such that slice(idx-1) <= waiting_time
                self.debark_times_by_position[position_idx]
                    .binary_search_by(|time| if time <= waiting_time { Less } else { Greater })
                    .unwrap_err()
            };

        for vehicle_idx in (0..after_last_debarkable_vehicle).rev() {
            let vehicle_data = &self.vehicle_datas[vehicle_idx];
            if filter(vehicle_data) {
                let departure_time_at_previous_position =
                    self.departure_time(vehicle_idx, position_idx - 1);
                return Some((vehicle_idx, departure_time_at_previous_position));
            }
        }
        None
    }

    pub(super) fn new<BoardTimes, DebarkTimes, Loads>(
        stop_flows: StopFlows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads: Loads,
        vehicle_data: VehicleData,
    ) -> Self
    where
        BoardTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        Loads: Iterator<Item = Load> + ExactSizeIterator + Clone,
        Time: Clone,
    {
        let nb_of_positions = stop_flows.len();
        assert!(nb_of_positions >= 2);
        assert!(board_times.len() == nb_of_positions);
        assert!(debark_times.len() == nb_of_positions);
        assert!(loads.len() == nb_of_positions - 1);

        let mut result = Self {
            stop_flows,
            vehicle_datas: Vec::new(),
            vehicle_loads: Vec::new(),
            debark_times_by_position: vec![Vec::new(); nb_of_positions],
            board_times_by_position: vec![Vec::new(); nb_of_positions],
        };
        result.do_insert(board_times, debark_times, loads, vehicle_data, 0);
        result
    }

    // Try to insert the trip in this timetable
    // Returns `true` if insertion was succesfull, `false` otherwise
    pub(super) fn try_insert<BoardTimes, DebarkTimes, Loads>(
        &mut self,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads: Loads,
        vehicle_data: VehicleData,
    ) -> bool
    where
        BoardTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        Loads: Iterator<Item = Load> + ExactSizeIterator + Clone,
        Time: Clone,
    {
        assert!(board_times.len() == self.nb_of_positions());
        assert!(debark_times.len() == self.nb_of_positions());
        assert!(loads.len() + 1 == self.nb_of_positions());
        let has_insert_idx =
            self.find_insert_idx(board_times.clone(), debark_times.clone(), loads.clone());
        if let Some(insert_idx) = has_insert_idx {
            self.do_insert(board_times, debark_times, loads, vehicle_data, insert_idx);
            true
        } else {
            false
        }
    }

    fn find_insert_idx<BoardTimes, DebarkTimes, Loads>(
        &self,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads: Loads,
    ) -> Option<usize>
    where
        BoardTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        Loads: Iterator<Item = Load> + ExactSizeIterator + Clone,
        Time: Debug,
        Load: Debug,
    {
        let nb_of_vehicle = self.nb_of_vehicle();
        if nb_of_vehicle == 0 {
            return Some(0);
        }

        let first_board_time = board_times.clone().next().unwrap();
        let first_board_time_binary_search =
            self.board_times_by_position[0].binary_search(&first_board_time);
        match first_board_time_binary_search {
            // here, first_board_time has not been found in &self.board_times_by_position[0]
            // and insert_idx is the index where this first_board_time should be inserted
            // so as to keep &self.board_times_by_position[0] sorted
            // so we  have
            //  first_board_time < &self.board_times_by_position[0][insert_idx]     if insert_idx < len
            //  first_board_time > &self.board_times_by_position[0][insert_idx -1]  if insert_idx > 0
            // so we are be able to insert the vehicle at insert_idx only if
            //       (board, debark, loads) <= vehicle_board_debark_loads(insert_idx) if insert_idx < len
            // and   (board, debark, loads) >= vehicle_board_debark_loads(insert_idx - 1) if insert_idx > 0
            Err(insert_idx) => {
                if insert_idx < self.nb_of_vehicle() {
                    match self.partial_cmp_with_vehicle(
                        board_times.clone(),
                        debark_times.clone(),
                        loads.clone(),
                        insert_idx,
                    ) {
                        None => {
                            return None;
                        }
                        Some(Ordering::Equal) | Some(Ordering::Greater) => {
                            unreachable!();
                        }
                        Some(Ordering::Less) => (),
                    }
                }

                if insert_idx > 0 {
                    match self.partial_cmp_with_vehicle(
                        board_times,
                        debark_times,
                        loads,
                        insert_idx - 1,
                    ) {
                        None => {
                            return None;
                        }
                        Some(Ordering::Equal) | Some(Ordering::Less) => {
                            unreachable!();
                        }
                        Some(Ordering::Greater) => (),
                    }
                }

                Some(insert_idx)
            }
            Ok(insert_idx) => {
                assert!(self.board_times_by_position[0][insert_idx] == first_board_time);
                let mut refined_insert_idx = insert_idx;
                while refined_insert_idx > 0
                    && self.board_times_by_position[0][refined_insert_idx] == first_board_time
                {
                    refined_insert_idx -= 1;
                }
                if refined_insert_idx > 0 {
                    match self.partial_cmp_with_vehicle(
                        board_times.clone(),
                        debark_times.clone(),
                        loads.clone(),
                        refined_insert_idx - 1,
                    ) {
                        None => {
                            return None;
                        }
                        Some(Ordering::Equal) | Some(Ordering::Less) => {
                            unreachable!();
                        }
                        Some(Ordering::Greater) => (),
                    }
                }
                self.find_insert_idx_after(board_times, debark_times, loads, refined_insert_idx)
            }
        }
    }

    fn find_insert_idx_after<BoardTimes, DebarkTimes, Loads>(
        &self,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads: Loads,
        start_search_idx: usize,
    ) -> Option<usize>
    where
        BoardTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        Loads: Iterator<Item = Load> + ExactSizeIterator + Clone,
    {
        let nb_of_vehicle = self.nb_of_vehicle();
        assert!(start_search_idx < nb_of_vehicle);

        let first_vehicle_idx = start_search_idx;
        let has_first_vehicle_comp = self.partial_cmp_with_vehicle(
            board_times.clone(),
            debark_times.clone(),
            loads.clone(),
            first_vehicle_idx,
        );

        // if the candidate is not comparable with first_vehicle
        // then we cannot add the candidate to this timetable
        let first_vehicle_comp = has_first_vehicle_comp?;
        // if first_vehicle >= candidate ,
        // then we should insert the candidate at the first position
        if first_vehicle_comp == Ordering::Less || first_vehicle_comp == Ordering::Equal {
            return Some(first_vehicle_idx);
        }
        assert!(first_vehicle_comp == Ordering::Greater);
        // otherwise, we look for a trip such that
        // prev_vehicle <= candidate <= vehicle
        let second_vehicle_idx = first_vehicle_idx + 1;
        for vehicle_idx in second_vehicle_idx..nb_of_vehicle {
            let has_vehicle_comp = self.partial_cmp_with_vehicle(
                board_times.clone(),
                debark_times.clone(),
                loads.clone(),
                vehicle_idx,
            );
            // if the candidate is not comparable with vehicle
            // then we cannot add the candidate to this timetable
            let vehicle_cmp = has_vehicle_comp?;

            if vehicle_cmp == Ordering::Less || vehicle_cmp == Ordering::Equal {
                return Some(vehicle_idx);
            }
            assert!(vehicle_cmp == Ordering::Greater);
        }

        // here  candidate_  >= vehicle for all vehicles,
        // so we can insert the candidate as the last vehicle
        Some(nb_of_vehicle)
    }

    pub(super) fn do_insert<BoardTimes, DebarkTimes, Loads>(
        &mut self,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads: Loads,
        vehicle_data: VehicleData,
        insert_idx: usize,
    ) where
        BoardTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        Loads: Iterator<Item = Load> + ExactSizeIterator + Clone,
    {
        if insert_idx < self.nb_of_vehicle() {
            assert!({
                let insert_cmp = self.partial_cmp_with_vehicle(
                    board_times.clone(),
                    debark_times.clone(),
                    loads.clone(),
                    insert_idx,
                );
                insert_cmp == Some(Ordering::Less) || insert_cmp == Some(Ordering::Equal)
            });
        }
        if insert_idx > 0 {
            assert!({
                let prev_insert_cmp = self.partial_cmp_with_vehicle(
                    board_times.clone(),
                    debark_times.clone(),
                    loads.clone(),
                    insert_idx - 1,
                );
                prev_insert_cmp == Some(Ordering::Greater)
            });
        }

        for (position, (board_time, debark_time)) in board_times.zip(debark_times).enumerate() {
            self.board_times_by_position[position].insert(insert_idx, board_time.clone());
            self.debark_times_by_position[position].insert(insert_idx, debark_time);
        }
        self.vehicle_datas.insert(insert_idx, vehicle_data);

        let loads_vec: Vec<Load> = loads.collect();
        self.vehicle_loads.insert(insert_idx, loads_vec);
    }

    fn partial_cmp_with_vehicle<BoardTimes, DebarkTimes, Loads>(
        &self,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads: Loads,
        vehicle_idx: usize,
    ) -> Option<Ordering>
    where
        BoardTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        Loads: Iterator<Item = Load> + ExactSizeIterator + Clone,
        Time: Clone,
    {
        let board_cmp = partial_cmp(board_times, self.vehicle_board_times(vehicle_idx))?;
        let debark_cmp = partial_cmp(debark_times, self.vehicle_debark_times(vehicle_idx))?;

        let board_debark_cmp = combine(board_cmp, debark_cmp)?;
        let loads_cmp = partial_cmp(loads, self.vehicle_loads(vehicle_idx))?;
        combine(board_debark_cmp, loads_cmp)
    }

    pub(super) fn remove_vehicles<Filter>(&mut self, vehicle_filter: Filter) -> usize
    where
        Filter: Fn(&VehicleData) -> bool,
    {
        let nb_to_remove = self
            .vehicle_datas
            .iter()
            .filter(|vehicle_data| vehicle_filter(vehicle_data))
            .count();
        if nb_to_remove == 0 {
            return 0;
        }

        //  to remove from a vec : use retain with a closure whose state tracks the current index/vehicle
        //              see https://stackoverflow.com/a/59602788
        for board_times in self.board_times_by_position.iter_mut() {
            let mut index = 0;
            let vehicle_datas = &self.vehicle_datas;
            board_times.retain(|_| {
                let to_retain = vehicle_filter(&vehicle_datas[index]).not();
                index += 1;
                to_retain
            });
        }
        for debark_times in self.debark_times_by_position.iter_mut() {
            let mut index = 0;
            let vehicle_datas = &self.vehicle_datas;
            debark_times.retain(|_| {
                let to_retain = vehicle_filter(&vehicle_datas[index]).not();
                index += 1;
                to_retain
            });
        }

        {
            let mut index = 0;
            let vehicle_datas = &self.vehicle_datas;
            self.vehicle_loads.retain(|_| {
                let to_retain = vehicle_filter(&vehicle_datas[index]).not();
                index += 1;
                to_retain
            });
        }

        {
            self.vehicle_datas
                .retain(|vehicle_data| vehicle_filter(vehicle_data).not());
        }

        nb_to_remove
    }

    pub fn update_vehicles_data<Updater>(&mut self, mut updater: Updater) -> usize
    where
        Updater: FnMut(&mut VehicleData) -> bool, // returns true when an update took place
    {
        let mut nb_updated = 0usize;
        for vehicle_data in self.vehicle_datas.iter_mut() {
            let updated = updater(vehicle_data);
            if updated {
                nb_updated += 1;
            }
        }

        nb_updated
    }
}

fn combine(a: Ordering, b: Ordering) -> Option<Ordering> {
    use Ordering::Equal;
    match (a, b) {
        (Less, Less) | (Less, Equal) | (Equal, Less) => Some(Less),
        (Equal, Equal) => Some(Equal),
        (Greater, Greater) | (Greater, Equal) | (Equal, Greater) => Some(Greater),
        _ => None,
    }
}

// Retuns
//    - Some(Equal)   if lower[i] == upper[i] for all i
//    - Some(Less)    if lower[i] <= upper[i] for all i
//    - Some(Greater) if lower[i] >= upper[i] for all i
//    - None otherwise (the two vector are not comparable)
pub(super) fn partial_cmp<Lower, Upper, Value, UpperVal, LowerVal>(
    lower: Lower,
    upper: Upper,
) -> Option<Ordering>
where
    Lower: Iterator<Item = UpperVal> + Clone,
    Upper: Iterator<Item = LowerVal> + Clone,
    Value: Ord,
    UpperVal: Borrow<Value>,
    LowerVal: Borrow<Value>,
{
    debug_assert!(lower.clone().count() == upper.clone().count());
    let zip_iter = lower.zip(upper);
    let mut first_not_equal_iter =
        zip_iter.skip_while(|(lower_val, upper_val)| lower_val.borrow() == upper_val.borrow());
    let has_first_not_equal = first_not_equal_iter.next();
    if let Some(first_not_equal) = has_first_not_equal {
        let ordering = {
            let lower_val = first_not_equal.0;
            let upper_val = first_not_equal.1;
            lower_val.borrow().cmp(upper_val.borrow())
        };
        debug_assert!(ordering != Ordering::Equal);
        // let's see if there is an index where the ordering is not the same
        // as first_ordering
        let found = first_not_equal_iter.find(|(lower_val, upper_val)| {
            let cmp = lower_val.borrow().cmp(upper_val.borrow());
            cmp != ordering && cmp != Ordering::Equal
        });
        if found.is_some() {
            return None;
        }
        // if found.is_none(), it means that
        // all elements are ordered the same, so the two vectors are comparable
        return Some(ordering);
    }
    // if has_first_not_equal == None
    // then values == item_values
    // the two vector are equal
    Some(Ordering::Equal)
}
