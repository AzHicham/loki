
use std::{borrow::Borrow, cmp::Ordering};


use std::collections::BTreeMap;

use crate::timetables::{Stop, StopFlows, FlowDirection};

#[derive(Debug)]
pub (super) struct Timetables<Time, TimezoneData, VehicleData> {
    pub (super) stop_flows_to_timetables : BTreeMap< StopFlows, Vec<Timetable> >,
    pub (super) timetable_datas : Vec< TimetableData<Time, TimezoneData, VehicleData> >,
}

#[derive(Debug)]
// TODO : document more explicitely !
pub (super) struct TimetableData<Time, TimezoneData, VehicleData> {

    pub (super) timezone_data : TimezoneData,

    pub (super) stop_flows: StopFlows,

    // vehicle data, ordered by increasing times
    // meaning that is vehicle_1 is before vehicle_2 in this vector,
    // then for all `position` we have
    //    debark_times_by_position[position][vehicle_1] <= debark_times_by_position[position][vehicle_2]
    pub (super) vehicle_datas: Vec<VehicleData>,

    // `board_times_by_position[position][vehicle]`
    //   is the time at which a traveler waiting
    //   at `position` can board `vehicle`
    // Vehicles are ordered by increasing time
    //  so for each `position` the vector
    //  board_times_by_position[position] is sorted by increasing times
    pub (super) board_times_by_position: Vec<Vec<Time>>,

    // `debark_times_by_position[position][vehicle]`
    //   is the time at which a traveler being inside `vehicle`
    //   will debark at `position`
    // Vehicles are ordered by increasing time
    //  so for each `position` the vector
    //  debark_times_by_position[position] is sorted by increasing times
    pub (super) debark_times_by_position: Vec<Vec<Time>>,

    pub (super) earliest_and_latest_board_time_by_position: Vec<(Time, Time)>,
}



#[derive(Debug, PartialEq, Eq, Clone, Hash, )]
pub struct Timetable {
    pub (super) idx: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Position {
    pub (super) timetable : Timetable,
    pub (super) idx : usize
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Vehicle {
    pub (super) timetable : Timetable,
    pub (super) idx: usize,
}

impl<Time, TimezoneData, VehicleData> 
Timetables <Time, TimezoneData, VehicleData>
where 
Time : Ord,
TimezoneData : PartialEq + Clone
{
    pub(super) fn new() -> Self {
        Self {
            stop_flows_to_timetables : BTreeMap::new(),
            timetable_datas : Vec::new(),
        }
    }


    pub(super) fn timezone_data(&self, timetable : & Timetable) -> &TimezoneData {
        &self.timetable_data(timetable).timezone_data
    }

    pub(super) fn nb_of_timetables(&self) -> usize {
        self.timetable_datas.len()
    }

    pub(super) fn nb_of_vehicles(&self) -> usize {
        self.timetable_datas.iter()
            .map(|timetable_data| timetable_data.nb_of_vehicle())
            .sum()
    }


    pub(super) fn timetable_data(&self, timetable : & Timetable) -> & TimetableData<Time, TimezoneData, VehicleData> {
        &self.timetable_datas[timetable.idx]
    }

    pub(super) fn vehicle_data(&self, vehicle : & Vehicle) -> & VehicleData {
        self.timetable_data(&vehicle.timetable).vehicle_data(vehicle.idx)
    }


    pub(super) fn stoptime_idx(&self, position : &Position) -> usize {
        position.idx
    }

    pub (super) fn timetable_of(&self, vehicle : & Vehicle) ->  Timetable {
        vehicle.timetable.clone()
    }

    pub(super) fn stop_at(&self, position : & Position, timetable : & Timetable) -> & Stop {
        assert!(*timetable == position.timetable);
        self.timetable_data(timetable).stop_at(position.idx)
    }


    pub(super) fn is_upstream(
        &self,
        upstream: &Position,
        downstream: &Position,
        timetable : & Timetable,
    ) -> bool {
        assert!(upstream.timetable == *timetable);
        assert!(upstream.timetable == *timetable);
        upstream.idx < downstream.idx
    }

    pub(super) fn next_position(&self, 
        position : & Position,
        timetable : & Timetable,
    ) -> Option<Position> {
        assert!(position.timetable == *timetable);
        if position.idx + 1 < self.timetable_data(&position.timetable).nb_of_positions() {
            let result = Position {
                timetable : position.timetable.clone(),
                idx : position.idx + 1
            };
            Some(result)
        }
        else {
            None
        }
    }

    pub(super)  fn debark_time(&self, vehicle: &Vehicle, position: &Position) -> Option<&Time> {
        assert!(vehicle.timetable == position.timetable);
        let timetable_data = self.timetable_data(&vehicle.timetable);
        timetable_data.debark_time(vehicle.idx, position.idx)
    }

    pub(super)  fn board_time(&self, vehicle: &Vehicle, position: &Position) -> Option<&Time> {
        assert!(vehicle.timetable == position.timetable);
        let timetable_data = self.timetable_data(&vehicle.timetable);
        timetable_data.board_time(vehicle.idx, position.idx)
    }

    pub(super)  fn arrival_time(&self, vehicle: &Vehicle, position: &Position) -> &Time {
        assert!(vehicle.timetable == position.timetable);
        let timetable_data = self.timetable_data(&vehicle.timetable);
        timetable_data.arrival_time(vehicle.idx, position.idx)
    }

    pub(super) fn earliest_and_latest_board_time(&self, position : & Position) -> Option<&(Time, Time)> {
        let timetable_data = self.timetable_data(&position.timetable);
        timetable_data.earliest_and_latest_board_time(position.idx)
    }

    pub(super) fn earliest_vehicle_to_board(&self, 
        waiting_time : & Time, 
        timetable : & Timetable, 
        position : & Position
    ) -> Option<(Vehicle, &Time)> 
    {
        self.earliest_filtered_vehicle_to_board(waiting_time, timetable, position, |_| true)
        
    }

    pub(super) fn earliest_filtered_vehicle_to_board<Filter>(&self, 
        waiting_time : & Time, 
        timetable : & Timetable, 
        position : & Position,
        filter : Filter
    ) -> Option<(Vehicle, &Time)> 
    where Filter: Fn(&VehicleData) -> bool
    {
        assert!(position.timetable == *timetable);
        self.timetable_data(timetable).earliest_filtered_vehicle_to_board(waiting_time, position.idx, filter)
            .map(|(idx, time)| {
                let vehicle = Vehicle {
                    timetable : timetable.clone(),
                    idx
                };
                (vehicle, time)
            })
    }

    // Insert in the trip in a timetable if
    // the given debark_times and board_times are coherent.
    // Returns a VehicleTimesError otherwise.
    pub fn insert<BoardDebarkTimes>(
        &mut self,
        stop_flows : StopFlows,
        board_debark_times: BoardDebarkTimes,
        timezone_data : TimezoneData,
        vehicle_data: VehicleData,
    ) -> Result<Timetable, VehicleTimesError>
    where
        BoardDebarkTimes: Iterator<Item = (Time, Time)> + ExactSizeIterator + Clone,
        Time : Clone,
        VehicleData : Clone
    {
        inspect(&stop_flows, board_debark_times.clone())?;
        
        let corrected_board_debark_times = board_debark_times.clone().zip(stop_flows.iter()).map(
            |((board_time, debark_time), (_,flow_direction))| match flow_direction {
                FlowDirection::BoardAndDebark => (board_time, debark_time),
                FlowDirection::BoardOnly => (board_time.clone(), board_time),
                FlowDirection::DebarkOnly => (debark_time.clone(), debark_time),
            },
        );
        let stop_flows_timetables = self.stop_flows_to_timetables.entry(stop_flows.clone()).or_insert(Vec::new());

        for mission in stop_flows_timetables.iter() {
            let timetable_data = & mut self.timetable_datas[mission.idx];
            let inserted = timetable_data
                .try_insert(corrected_board_debark_times.clone(), timezone_data.clone(), vehicle_data.clone());
            if inserted {
                return Ok(mission.clone());
            }
        }
        let new_timetable_data = TimetableData::new(stop_flows, board_debark_times, timezone_data, vehicle_data);
        let timetable = Timetable{ idx : self.timetable_datas.len() };
        self.timetable_datas.push(new_timetable_data);
        stop_flows_timetables.push(timetable.clone());
        Ok(timetable)
    }

}


impl<Time, TimezoneData, VehicleData>
TimetableData<Time, TimezoneData, VehicleData >
where 
Time : Ord,
TimezoneData : PartialEq
{

    fn can_board(&self, position_idx: usize) -> bool {
        match &self.stop_flows[position_idx].1 {
            FlowDirection::BoardAndDebark
            | FlowDirection::BoardOnly => true,
            FlowDirection::DebarkOnly => false
        }
    }

    fn can_debark(&self, position_idx: usize) -> bool {
        match &self.stop_flows[position_idx].1 {
            FlowDirection::BoardAndDebark
            | FlowDirection::DebarkOnly => true,
            FlowDirection::BoardOnly => false
        }
    }

    fn arrival_time(&self, trip_idx : usize, position_idx: usize) -> &Time {
        &self.debark_times_by_position[position_idx][trip_idx]
 
        
    }

    fn debark_time(&self, trip_idx : usize, position_idx: usize) -> Option<&Time> {
        if self.can_debark(position_idx) {
            Some(&self.debark_times_by_position[position_idx][trip_idx])
        }
        else {
            None
        }
        
    }

    fn board_time(&self, trip_idx : usize, position_idx: usize)-> Option<&Time> {
        if self.can_board(position_idx) {
            Some(&self.board_times_by_position[position_idx][trip_idx])
        }
        else {
            None
        }
        
    }

    fn earliest_and_latest_board_time(&self, position_idx: usize) -> Option<&(Time, Time)> {
        if self.can_board(position_idx) {
            Some(&self.earliest_and_latest_board_time_by_position[position_idx])
        }
        else {
            None
        }
        
    }


    fn stop_at(&self, position_idx: usize) -> & Stop {
        &self.stop_flows[position_idx].0
    }

    fn flow_direction_at(&self, position_idx: usize) -> & FlowDirection {
        &self.stop_flows[position_idx].1
    }

    pub(super) fn nb_of_positions(&self) -> usize {
        self.board_times_by_position.len()
    }

    pub(super) fn nb_of_vehicle(&self) -> usize {
        self.vehicle_datas.len()
    }

    fn vehicle_data(&self, trip_idx : usize) -> & VehicleData {
        &self.vehicle_datas[trip_idx]
    }




   
    // If we are waiting to board a trip at `position` at time `waiting_time`
    // return `Some(best_trip_idx)`
    // where `best_trip_idx` is the idx of the trip, among those trip on which `filter` returns true,
    //  to board that allows to debark at the subsequent positions at the earliest time,
    fn earliest_filtered_vehicle_to_board<Filter>(
        &self,
        waiting_time: &Time,
        position_idx: usize,
        filter: Filter,
    ) -> Option<(usize, &Time)>
    where
        Filter: Fn(&VehicleData) -> bool,
    {
        if ! self.can_board(position_idx) {
            return None;
        }
        let next_position_idx = position_idx + 1;
        // we should not be able to board at the last position
        assert!(next_position_idx  < self.nb_of_positions());

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

        for vehicle_idx in first_boardable_vehicle..self.nb_of_vehicle() {
            let vehicle_data = &self.vehicle_datas[vehicle_idx];
            let board_time = &self.board_times_by_position[position_idx][vehicle_idx];
            if filter(vehicle_data) && waiting_time <= board_time {
                let arrival_time_at_next_position = self.arrival_time(vehicle_idx, next_position_idx);
                return Some((vehicle_idx, arrival_time_at_next_position));
            }
        }
        None
    }

    fn new<BoardDebarkTimes>(
        stop_flows : StopFlows, 
        board_debark_times: BoardDebarkTimes,
        timezone_data : TimezoneData,
        vehicle_data: VehicleData,
    ) -> Self 
    where
        BoardDebarkTimes: Iterator<Item = (Time, Time)> + ExactSizeIterator + Clone,
        Time : Clone
    {
        let nb_of_positions = stop_flows.len();
        assert!(nb_of_positions >= 2);
        let mut result = Self {
            timezone_data,
            stop_flows,
            vehicle_datas: Vec::new(),
            debark_times_by_position: vec![Vec::new(); nb_of_positions],
            board_times_by_position: vec![Vec::new(); nb_of_positions],
            earliest_and_latest_board_time_by_position: Vec::new(),
        };
        result.do_insert(board_debark_times, vehicle_data, 0);
        result
    }
    
    // Try to insert the trip in this timetable
    // Returns `true` if insertion was succesfull, `false` otherwise
    fn try_insert<BoardDebarkTimes>(
        &mut self,
        board_debark_times: BoardDebarkTimes,
        timezone_data : TimezoneData,
        trip_data: VehicleData,
    ) -> bool
    where
        BoardDebarkTimes: Iterator<Item =(Time, Time)> + ExactSizeIterator + Clone,
    {
        assert!(board_debark_times.len() == self.nb_of_positions());
        if self.timezone_data != timezone_data {
            return false;
        }
        let has_insert_idx = self.find_insert_idx(board_debark_times.clone());
        if let Some(insert_idx) = has_insert_idx {
            self.do_insert(board_debark_times, trip_data, insert_idx);
            true
        } else {
            false
        }
    }

    fn find_insert_idx< BoardDebarkTimes>(
        &self,
        board_debark_times: BoardDebarkTimes,
    ) -> Option<usize>
    where
        BoardDebarkTimes: Iterator<Item = (Time, Time)> + ExactSizeIterator + Clone,
    {
        let nb_of_vehicle = self.nb_of_vehicle();
        if nb_of_vehicle == 0 {
            return Some(0);
        }

        let board_then_debark = board_debark_times
            .clone()
            .map(|(board, _)| board)
            .chain(board_debark_times.clone().map(|(_, debark)| debark));

        let first_board_time = board_debark_times.clone().next().unwrap().0;
        let first_board_time_binary_search =
            (&self.board_times_by_position[0]).binary_search(&first_board_time);
        match first_board_time_binary_search {
            // here, first_board_time has not been found in &self.board_times_by_position[0]
            // and insert_idx is the index where this first_board_time should be inserted
            // so as to keep &self.board_times_by_position[0] sorted
            // so we  have
            //  first_board_time < &self.board_times_by_position[0][insert_idx]     if insert_idx < len
            //  first_board_time > &self.board_times_by_position[0][insert_idx -1]  if insert_idx > 0
            // so we are be able to insert the vehicle at insert_idx only if
            //       board_then_debark <= vehicle_board_then_debark_times(insert_idx) if insert_idx < len
            // and   board_then_debark >= vehicle_board_then_debark_times(insert_idx - 1) if insert_idx > 0
            Err(insert_idx) => {
                if insert_idx < self.nb_of_vehicle() {
                    match partial_cmp(
                        board_then_debark.clone(),
                        self.vehicle_board_then_debark_times(insert_idx),
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
                    match partial_cmp(
                        board_then_debark,
                        self.vehicle_board_then_debark_times(insert_idx - 1),
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
                    refined_insert_idx -=  1;
                }
                if refined_insert_idx > 0 {
                    match partial_cmp(
                        board_then_debark,
                        self.vehicle_board_then_debark_times(refined_insert_idx - 1),
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
                self.find_insert_idx_after(board_debark_times, refined_insert_idx)
            }
        }
    }

    fn find_insert_idx_after<BoardDebarkTimes>(
        &self,
        board_debark_times: BoardDebarkTimes,
        start_search_idx: usize,
    ) -> Option<usize>
    where
        BoardDebarkTimes: Iterator<Item =  (Time, Time)> + ExactSizeIterator + Clone,
    {
        let nb_of_vehicle = self.nb_of_vehicle();
        assert!(start_search_idx < nb_of_vehicle);

        let board_then_debark = board_debark_times
            .clone()
            .map(|(board, _)| board)
            .chain(board_debark_times.map(|(_, debark)| debark));

        let first_trip_idx = start_search_idx;
        let has_first_trip_comp = partial_cmp(
            board_then_debark.clone(),
            self.vehicle_board_then_debark_times(first_trip_idx),
        );
        // if the candidate_times_vector is not comparable with first_trip_times_vector
        // then we cannot add the candidate to this timetable
        let first_trip_comp = has_first_trip_comp?;
        // if first_vehicle_times_vector >= candidate_times_vector ,
        // then we should insert the candidate at the first position
        if first_trip_comp == Ordering::Less || first_trip_comp == Ordering::Equal {
            return Some(first_trip_idx);
        }
        assert!(first_trip_comp == Ordering::Greater);
        // otherwise, we look for a trip such that
        // prev_trip_times_vector <= candidate_times_vector <= trip_times_vector
        let second_trip_idx = first_trip_idx + 1;
        for trip_idx in second_trip_idx..nb_of_vehicle {
            let has_trip_comp = partial_cmp(
                board_then_debark.clone(),
                self.vehicle_board_then_debark_times(trip_idx),
            );
            // if the candidate_times_vector is not comparable with trip_times_vector
            // then we cannot add the candidate to this timetable
            let trip_comp = has_trip_comp?;

            if trip_comp == Ordering::Less || trip_comp == Ordering::Equal {
                return Some(trip_idx);
            }
            assert!(trip_comp == Ordering::Greater);
        }

        // here  candidate_times_vector  >= vehicle_times_vector for all vehicles,
        // so we can insert the candidate as the last vehicle
        Some(nb_of_vehicle)
    }


    fn do_insert<BoardDebarkTimes>(
        &mut self,
        board_debark_times: BoardDebarkTimes,
        vehicle_data: VehicleData,
        insert_idx: usize,
    )    
    where
        BoardDebarkTimes: Iterator<Item = (Time, Time)> + ExactSizeIterator + Clone,
    {
        if insert_idx < self.nb_of_vehicle() {
            assert!({
                let board_then_debark = board_debark_times
                    .clone()
                    .map(|(board, _)| board)
                    .chain(board_debark_times.clone().map(|(_, debark)| debark));
                let insert_cmp = partial_cmp(
                    board_then_debark,
                    self.vehicle_board_then_debark_times(insert_idx),
                );
                insert_cmp == Some(Ordering::Less) || insert_cmp == Some(Ordering::Equal)
            });
        }
        if insert_idx > 0 {
            assert!({
                let board_then_debark = board_debark_times
                    .clone()
                    .map(|(board, _)| board)
                    .chain(board_debark_times.clone().map(|(_, debark)| debark));
                let prev_insert_cmp = partial_cmp(
                    board_then_debark,
                    self.vehicle_board_then_debark_times(insert_idx - 1),
                );
                prev_insert_cmp == Some(Ordering::Greater)
            });
        }

        for (position, (board_time, debark_time)) in board_debark_times.enumerate() {
            self.board_times_by_position[position].insert(insert_idx, board_time);
            self.debark_times_by_position[position].insert(insert_idx, debark_time);
        }
        self.vehicle_datas.insert(insert_idx, vehicle_data);
    }


   
}

// Retuns
//    - Some(Equal)   if lower[i] == upper[i] for all i
//    - Some(Less)    if lower[i] <= upper[i] for all i
//    - Some(Greater) if lower[i] >= upper[i] for all i
//    - None otherwise (the two vector are not comparable)
pub (super) fn partial_cmp< Lower, Upper, Value, UpperVal, LowerVal>(lower: Lower, upper: Upper) -> Option<Ordering>
where
    Lower: Iterator<Item =  UpperVal> + Clone,
    Upper: Iterator<Item =  LowerVal> + Clone,
    Value: Ord ,
    UpperVal : Borrow<Value>,
    LowerVal : Borrow<Value>,
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
            lower_val.borrow().cmp(&upper_val.borrow())
        };
        debug_assert!(ordering != Ordering::Equal);
        // let's see if there is an index where the ordering is not the same
        // as first_ordering
        let found = first_not_equal_iter.find(|(lower_val, upper_val)| {
            let cmp = lower_val.borrow().cmp(&upper_val.borrow());
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

fn is_increasing<EnumeratedValues, Value>(
    mut enumerated_values: EnumeratedValues,
) -> Result<(), (usize, usize)>
where
    EnumeratedValues: Iterator<Item = (usize, Value)>,
    Value : Ord
{
    let has_previous = enumerated_values.next();
    let (mut prev_position, mut prev_value) = has_previous.unwrap();
    for (position, value) in enumerated_values {
        if value < prev_value {
            return Err((prev_position, position));
        }
        prev_position = position;
        prev_value = value;
    }
    Ok(())
}

pub(super) fn inspect<BoardDebarkTimes, Time>(
    stop_flows : & StopFlows,
    board_debark_times:  BoardDebarkTimes,
) -> Result< (), VehicleTimesError > 
where
    BoardDebarkTimes: Iterator<Item = (Time, Time)> + ExactSizeIterator + Clone ,
    Time : Ord + Clone
{
    assert!(stop_flows.len() == board_debark_times.len());

    let valid_enumerated_board_times = board_debark_times
        .clone()
        .zip(stop_flows.iter())
        .enumerate()
        .filter_map(
            |(position, ((board_time, _), (_,flow_direction)))| match flow_direction {
                FlowDirection::BoardOnly | FlowDirection::BoardAndDebark => {
                    Some((position, board_time))
                }
                FlowDirection::DebarkOnly => None,
            },
        );

    if let Err((upstream, downstream)) = is_increasing(valid_enumerated_board_times.clone()) {
        let position_pair = PositionPair {
            upstream,
            downstream,
        };
        return Err(VehicleTimesError::DecreasingBoardTime(position_pair));
    }

    let valid_enumerated_debark_times = board_debark_times
        .clone()
        .zip(stop_flows.iter())
        .enumerate()
        .filter_map(
            |(position, ((_, debark_time), (_, flow_direction)))| match flow_direction {
                FlowDirection::DebarkOnly | FlowDirection::BoardAndDebark => {
                    Some((position, debark_time))
                }
                FlowDirection::BoardOnly => None,
            },
        );

    if let Err((upstream, downstream)) = is_increasing(valid_enumerated_debark_times.clone()) {
        let position_pair = PositionPair {
            upstream,
            downstream,
        };
        return Err(VehicleTimesError::DecreasingDebarkTime(position_pair));
    }

    let pair_iter = board_debark_times
        .clone()
        .zip(board_debark_times.clone().skip(1))
        .enumerate();
    for (board_idx, ((board_time, _), (_, debark_time))) in pair_iter {
        let debark_idx = board_idx  + 1;
        let can_board = match &stop_flows[board_idx].1 {
            FlowDirection::BoardAndDebark
            | FlowDirection::BoardOnly => true,
            FlowDirection::DebarkOnly =>false
        };
        let can_debark = match &stop_flows[debark_idx].1 {
            FlowDirection::BoardAndDebark
            | FlowDirection::DebarkOnly => true,
            FlowDirection::BoardOnly => false
        };
        if can_board
            && can_debark
            && board_time > debark_time
        {
            let position_pair = PositionPair {
                upstream: board_idx,
                downstream: debark_idx,
            };
            return Err(VehicleTimesError::DebarkBeforeUpstreamBoard(position_pair));
        }
    }

    Ok(())

}

pub(super) struct PositionPair {
    pub upstream: usize,
    pub downstream: usize,
}

pub(super) enum VehicleTimesError {
    DebarkBeforeUpstreamBoard(PositionPair), // board_time[upstream] > debark_time[downstream]
    DecreasingBoardTime(PositionPair),       // board_time[upstream] > board_time[downstream]
    DecreasingDebarkTime(PositionPair),      // debark_time[upstream] > debark_time[downstream]
}
