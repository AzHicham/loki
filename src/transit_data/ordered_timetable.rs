use std::cmp::Ordering;
use super::data::{Stop, FlowDirection, VehicleData};
use std::ops::Range;
use std::iter::{Map, Chain};

use super::time::SecondsSinceDayStart as Time;

// TODO : document more explicitely !
pub struct StopPatternData {
    stops : Vec<Stop>,
    flow_directions : Vec<FlowDirection>,
    timetables : Vec<TimetableData>,
}

#[derive(Debug)]
// TODO : document more explicitely !
pub struct TimetableData {
    // vehicles data, ordered by increasing times
    // meaning that is v1 is before v2 in this vector,
    // then for all `position` we have 
    //    debark_time_by_vehicle[v1][position] <= debark_time_by_vehicle[v2][position]
    vehicles_data : Vec<VehicleData>,


    // `board_times_by_position[position][vehicle]`
    //   is the time at which a traveler waiting
    //   at `position` can board `vehicle`
    // Vehicles are ordered by increasing time
    //  so for each `position` the vector
    //  board_times_by_position[position] is sorted by increasing times
    board_times_by_position : Vec<Vec<Time>>, 

    // `debark_times_by_position[position][vehicle]`
    //   is the time at which a traveler being inside `vehicle`
    //   will debark at `position` 
    // Vehicles are ordered by increasing time
    //  so for each `position` the vector
    //  debark_times_by_position[position] is sorted by increasing times
    debark_times_by_position : Vec<Vec<Time>>, 

    latest_board_time_by_position : Vec<Time>, 


}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Ord, PartialOrd)]
pub struct Position {
    idx : usize,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Timetable {
    idx : usize,
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Vehicle {
    idx : usize
}


pub type TimetablesIter = Map<Range<usize>, fn(usize) -> Timetable >;

impl StopPatternData
{
    pub fn new(stops : Vec<Stop>, flow_directions : Vec<FlowDirection>) -> Self {
        assert!(stops.len() >= 2);
        assert!(flow_directions.len() == stops.len());
        assert!(flow_directions.first().unwrap() == &FlowDirection::BoardOnly);
        assert!(flow_directions.last().unwrap() == &FlowDirection::DebarkOnly);
        Self{
            stops,
            flow_directions,
            timetables : Vec::new(),
        }
    }


    pub fn stops_and_positions(&self) -> impl Iterator<Item = (Stop, Position)> + '_ {
        self.stops.iter().enumerate()
            .map(|(idx, stop)| {
                let position = Position { idx };
                (stop.clone(), position)
            })
    }

    pub fn is_valid(&self, position : & Position) -> bool {
        position.idx < self.nb_of_positions()
    }

    pub fn next_position(&self, position : & Position) -> Option<Position> {
        let next_position = Position{ idx : position.idx + 1};
        if self.is_valid(&next_position) {
            Some(next_position)
        }
        else{
            None
        }
    }

    pub fn stop_at(&self, position : & Position) -> & Stop {
        & self.stops[position.idx]
    }


    pub fn is_last_position(&self, position : & Position) -> bool {
        position.idx == self.stops.len() - 1   
    }


    pub fn can_debark(&self, position : & Position) -> bool {
        let flow_direction = & self.flow_directions[position.idx];
        match flow_direction {
            FlowDirection::BoardAndDebark 
            | FlowDirection::DebarkOnly => { true},
            FlowDirection::BoardOnly => { false }
        }
    }

    pub fn can_board(&self, position : & Position) -> bool {
        let flow_direction = & self.flow_directions[position.idx];
        match flow_direction {
            FlowDirection::BoardAndDebark 
            | FlowDirection::BoardOnly => { true},
            FlowDirection::DebarkOnly => { false }
        }
    }

    fn timetable_data<'a>(& 'a self, timetable : & Timetable) -> & 'a TimetableData {
        self.timetables.get(timetable.idx)
            .unwrap_or_else( || panic!(format!(
                "The timetable {:?} is expected to belongs to the stop_pattern ", 
                    *timetable)
                )
            )
    }

    pub fn timetables(&self) -> TimetablesIter {
        (0..self.timetables.len()).map(|idx| {
            Timetable {
                idx
            }
        })
    }

    pub fn nb_of_timetables(&self) -> usize {
        self.timetables.len()
    }

    pub fn nb_of_vehicles(&self) -> usize {
        self.timetables.iter().map(|timetable| timetable.nb_of_vehicles()).sum()
    }

    pub fn vehicles(&self, timetable : & Timetable) -> VehiclesIter {
        let timetable_data = self.timetable_data(timetable);
        let nb_of_vehicles = timetable_data.nb_of_vehicles();
        (0..nb_of_vehicles).map(|idx| {
            Vehicle{
                idx
            }
        })
    }

    pub fn debark_time_at(&self, timetable : & Timetable, vehicle : & Vehicle, position : & Position) -> Option<& Time> {
        
        if self.can_debark(position) {
            let timetable_data = self.timetable_data(timetable);
            let time = timetable_data.debark_time_at(vehicle, position);
            Some(time)
        }
        else {
            None
        }
    }

    pub fn arrival_time_at(&self, timetable : & Timetable, vehicle : & Vehicle, position : & Position) -> & Time {
        
        let timetable_data = self.timetable_data(timetable);
        timetable_data.debark_time_at(vehicle, position)

    }

    pub fn board_time_at(&self, timetable : & Timetable, vehicle: & Vehicle, position : & Position) -> Option<&Time> {
        if self.can_board(position) {
            let timetable_data = self.timetable_data(timetable);
            let time = timetable_data.board_time_at(vehicle, position);
            Some(time)
        }
        else {
            None
        }
    }

    pub fn latest_board_time_at(&self, timetable : & Timetable, position : & Position) -> Option<& Time> {
        if self.can_board(position) {
            let timetable_data = self.timetable_data(timetable);
            let time = timetable_data.latest_board_time_at(position);
            Some(time)
        }
        else {
            None
        }
    }

    pub fn vehicle_data(&self, timetable : & Timetable, vehicle : & Vehicle) -> &VehicleData {
        & self.timetable_data(timetable).vehicles_data[vehicle.idx]
    }

    // If we are waiting to board a vehicle at `position` at time `waiting_time`
    // will return the index of the vehicle, among those vehicle on which `filter` returns true,
    //  to board that allows to debark at the subsequent positions at the earliest time.
    pub fn earliest_filtered_vehicle_to_board_at<Filter>(&self, 
        waiting_time : & Time, 
        timetable : & Timetable,
        position : & Position,
        filter : Filter
    ) -> Option<Vehicle> 
    where Filter : Fn(&VehicleData) -> bool
    {
        if ! self.can_board(position) {
            return None;
        }
        let timetable_data = self.timetable_data(timetable);
        timetable_data.best_filtered_vehicle_to_board_at(waiting_time, position, filter)
    }

    fn nb_of_positions(&self) -> usize {
        self.stops.len()
    }

    // Insert in the vehicle in a timetable if 
    // the given debark_times and board_times are coherent.
    // Returns a VehicleTimesError otherwise.
    pub fn insert<'a, 'b, BoardDebarkTimes,  >(& mut self,  
        board_debark_times : BoardDebarkTimes, 
        vehicle_data : VehicleData
    ) -> Result<(), VehicleTimesError>
    where 
    BoardDebarkTimes : Iterator<Item = (Time, Time)> + ExactSizeIterator + Clone,
    {
        assert!(self.nb_of_positions() == board_debark_times.len());

        let valid_enumerated_board_times = board_debark_times.clone()
                                .zip(self.flow_directions.iter())
                                .enumerate()
                                .filter_map(|(position, ((board_time, _), flow_direction)) | {
                                    match flow_direction {
                                        FlowDirection::BoardOnly 
                                        | FlowDirection::BoardAndDebark => { Some((position, board_time))},
                                        FlowDirection::DebarkOnly => { None },
                                    }
                                });

        if let Err((upstream, downstream)) = is_increasing(valid_enumerated_board_times.clone()) {
            let position_pair = PositionPair{
                upstream,
                downstream
            };
            return Err(VehicleTimesError::DecreasingBoardTime(position_pair));
        }


        let valid_enumerated_debark_times = board_debark_times.clone()
                .zip(self.flow_directions.iter())
                .enumerate()
                .filter_map(|(position, ((_, debark_time), flow_direction)) | {
                    match flow_direction {
                        FlowDirection::DebarkOnly 
                        | FlowDirection::BoardAndDebark => { Some((position, debark_time))},
                        FlowDirection::BoardOnly => { None },
                    }
                });

        if let Err((upstream, downstream)) = is_increasing(valid_enumerated_debark_times.clone()) {
            let position_pair = PositionPair{
                upstream,
                downstream
            };
            return Err(VehicleTimesError::DecreasingDebarkTime(position_pair))
        }

        let pair_iter = board_debark_times.clone().zip(board_debark_times.clone().skip(1)).enumerate();
        for (board_idx, ((board_time,_), (_, debark_time))) in pair_iter {
            let board_position = Position {
                idx : board_idx
            };
            let debark_position = Position {
                idx : board_idx + 1
            };
            if self.can_board(&board_position) && self.can_debark(&debark_position) && board_time > debark_time {
                let position_pair = PositionPair {
                    upstream : board_position.idx,
                    downstream : debark_position.idx
                };
                return Err(VehicleTimesError::DebarkBeforeUpstreamBoard(position_pair));
            }
        }

        let corrected_board_debark_times = board_debark_times
            .zip(self.flow_directions.iter())
            .map(|((board_time, debark_time), flow_direction)| {
                match flow_direction {
                    FlowDirection::BoardAndDebark => (board_time, debark_time),
                    FlowDirection::BoardOnly => (board_time.clone(), board_time),
                    FlowDirection::DebarkOnly => (debark_time.clone(), debark_time)
                }
            });

        for timetable_data in & mut self.timetables {
            let inserted = timetable_data.try_insert(corrected_board_debark_times.clone(), vehicle_data.clone());
            if inserted {
                return Ok(())
            }
        }
        let mut new_timetable_data =TimetableData::new(self.nb_of_positions());
        let inserted = new_timetable_data.try_insert(corrected_board_debark_times, vehicle_data);
        assert!(inserted);
        self.timetables.push(new_timetable_data);
        Ok(())
    }

}

pub type VehiclesIter = Map<Range<usize>, fn(usize)->Vehicle>;

impl TimetableData
{

    fn new(nb_of_positions : usize) -> Self {
        assert!( nb_of_positions >= 2);
        Self{
            vehicles_data : Vec::new(),
            debark_times_by_position : vec![Vec::new(); nb_of_positions],
            board_times_by_position : vec![Vec::new(); nb_of_positions],
            latest_board_time_by_position : vec![Time::zero(); nb_of_positions],
        }
    }

    fn debark_time_at(&self, vehicle : & Vehicle, position :  & Position) -> &Time {
        &self.debark_times_by_position[position.idx][vehicle.idx]
    }

    fn board_time_at(&self, vehicle : & Vehicle, position :  & Position) -> &Time {
        & self.board_times_by_position[position.idx][vehicle.idx]
    }

    fn latest_board_time_at(&self, position : & Position) -> &Time {
        & self.latest_board_time_by_position[position.idx]
    }


    fn nb_of_positions(&self) -> usize {
        self.board_times_by_position.len()
    }

    fn nb_of_vehicles(&self) -> usize {
        self.vehicles_data.len()
    }

    fn vehicle_debark_times<'a>(& 'a self, vehicle_idx : usize) -> VehicleTimes<'a> {
        debug_assert!( vehicle_idx < self.vehicles_data.len() );
        VehicleTimes {
            times_by_position : & self.debark_times_by_position,
            position : 0,
            vehicle : vehicle_idx
        }
    }

    fn vehicle_board_times<'a>(& 'a self, vehicle_idx : usize) -> VehicleTimes<'a> {
        debug_assert!( vehicle_idx < self.vehicles_data.len() );
        VehicleTimes {
            times_by_position : & self.board_times_by_position,
            position : 0,
            vehicle : vehicle_idx
        }
    }

    fn vehicle_board_then_debark_times<'a>(& 'a self, vehicle_idx : usize) -> Chain<VehicleTimes<'a>, VehicleTimes<'a>>
    {
        self.vehicle_board_times(vehicle_idx).chain(self.vehicle_debark_times(vehicle_idx))
    }


    fn find_insert_idx_after<BoardDebarkTimes>(& self, board_debark_times : BoardDebarkTimes, start_search_idx : usize) -> Option<usize>
    where 
    BoardDebarkTimes : Iterator<Item =  (Time, Time)> + ExactSizeIterator + Clone,
    {

        let nb_of_vehicles = self.nb_of_vehicles();
        assert!(start_search_idx < nb_of_vehicles);
        
        let board_then_debark = board_debark_times.clone().map(|(board,_)| board)
                                    .chain(board_debark_times.clone().map(|(_, debark)| debark));

        let first_vehicle_idx = start_search_idx;
        let has_first_vehicle_comp = partial_cmp(board_then_debark.clone(), self.vehicle_board_then_debark_times(first_vehicle_idx));
        // if the candidate_times_vector is not comparable with first_vehicle_times_vector
        // then we cannot add the candidate to this timetable
        if has_first_vehicle_comp.is_none() {
            return None;
        }
        let first_vehicle_comp = has_first_vehicle_comp.unwrap();
        // if first_vehicle_times_vector >= candidate_times_vector , 
        // then we should insert the candidate at the first position
        if first_vehicle_comp == Ordering::Less || first_vehicle_comp == Ordering::Equal {
            return Some(first_vehicle_idx);
        }
        assert!(first_vehicle_comp == Ordering::Greater);
        // otherwise, we look for a vehicle such that
        // prev_vehicle_times_vector <= candidate_times_vector <= vehicle_times_vector
        let second_vehicle_idx = first_vehicle_idx + 1;
        for vehicle_idx in second_vehicle_idx..nb_of_vehicles {
            let has_vehicle_comp = partial_cmp(board_then_debark.clone(), self.vehicle_board_then_debark_times(vehicle_idx), );
            // if the candidate_times_vector is not comparable with vehicle_times_vector
            // then we cannot add the candidate to this timetable
            if has_vehicle_comp.is_none() {
                return None;
            }
            let vehicle_comp = has_vehicle_comp.unwrap();

            if vehicle_comp == Ordering::Less ||  vehicle_comp == Ordering::Equal {
                return Some(vehicle_idx)
            }            assert!(vehicle_comp == Ordering::Greater);
        }
        
        // here  candidate_times_vector  >= vehicle_times_vector for all vehicles,
        // so we can insert the candidate as the last vehicle
        Some(nb_of_vehicles)

    }

    fn find_insert_idx<BoardDebarkTimes>(& self, board_debark_times : BoardDebarkTimes) -> Option<usize>
    where 
    BoardDebarkTimes : Iterator<Item =  (Time, Time)> + ExactSizeIterator + Clone,
    {

        

        let nb_of_vehicles = self.nb_of_vehicles();
        if nb_of_vehicles == 0 {
            return Some(0);
        }

        let board_then_debark = board_debark_times.clone().map(|(board,_)| board)
        .chain(board_debark_times.clone().map(|(_, debark)| debark));

        let first_board_time = board_debark_times.clone().next().unwrap().0;
        let first_board_time_binary_search = (&self.board_times_by_position[0]).binary_search(&first_board_time);
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
                if insert_idx < self.nb_of_vehicles() {
                    match partial_cmp(board_then_debark.clone(), self.vehicle_board_then_debark_times(insert_idx)) {
                        None => { return None;},
                        Some(Ordering::Equal) | Some(Ordering::Greater) => { assert!(false);},
                        Some(Ordering::Less) => {()}
                    }
                }

                if insert_idx > 0 {
                    match partial_cmp(board_then_debark.clone(), self.vehicle_board_then_debark_times(insert_idx - 1)) {
                        None => { return None;},
                        Some(Ordering::Equal) | Some(Ordering::Less) => { assert!(false);},
                        Some(Ordering::Greater) => {()}
                    } 
                }

                return Some(insert_idx);
            },
            Ok(insert_idx) => {
                assert!(self.board_times_by_position[0][insert_idx] == first_board_time );
                let mut refined_insert_idx = insert_idx;
                while refined_insert_idx > 0 && self.board_times_by_position[0][refined_insert_idx] == first_board_time {
                    refined_insert_idx = refined_insert_idx - 1;
                }
                if refined_insert_idx > 0 {
                    match partial_cmp(board_then_debark.clone(), self.vehicle_board_then_debark_times(refined_insert_idx - 1)) {
                        None => { return None;},
                        Some(Ordering::Equal) | Some(Ordering::Less) => { assert!(false);},
                        Some(Ordering::Greater) => {()}
                    } 
                }
                return self.find_insert_idx_after(board_debark_times, refined_insert_idx);
            }
        }


    }

    fn do_insert<BoardDebarkTimes>(& mut self, board_debark_times : BoardDebarkTimes, vehicle_data : VehicleData, insert_idx : usize)
    where 
    BoardDebarkTimes : Iterator<Item =  (Time, Time)> + ExactSizeIterator + Clone,
    {
        if insert_idx < self.nb_of_vehicles() {
            assert!({
                let board_then_debark = board_debark_times.clone().map(|(board,_)| board)
                .chain(board_debark_times.clone().map(|(_, debark)| debark));
                let insert_cmp = partial_cmp(board_then_debark, self.vehicle_board_then_debark_times(insert_idx));
                insert_cmp == Some(Ordering::Less) || insert_cmp == Some(Ordering::Equal)
            });
        }
        if insert_idx > 0 {
            assert!({
                let board_then_debark = board_debark_times.clone().map(|(board,_)| board)
                .chain(board_debark_times.clone().map(|(_, debark)| debark));
                let prev_insert_cmp = partial_cmp(board_then_debark, self.vehicle_board_then_debark_times(insert_idx-1));
                prev_insert_cmp == Some(Ordering::Greater) 
            });
        }
        
        for (position, (board_time, debark_time)) in board_debark_times.enumerate() {
            self.board_times_by_position[position].insert(insert_idx, board_time.clone());
            self.debark_times_by_position[position].insert(insert_idx, debark_time);
            let latest_board_time = & mut self.latest_board_time_by_position[position];
            * latest_board_time = std::cmp::max(latest_board_time.clone(), board_time);

        }
        self.vehicles_data.insert(insert_idx, vehicle_data);
    }

    // Try to insert the vehicle in this timetable
    // Returns `true` if insertion was succesfull, `false` otherwise
    fn try_insert<BoardDebarkTimes >(& mut self,  
        board_debark_times : BoardDebarkTimes, 
        vehicle_data : VehicleData) -> bool
    where 
    BoardDebarkTimes : Iterator<Item =  (Time, Time)> + ExactSizeIterator + Clone, {
        assert!(board_debark_times.len() == self.nb_of_positions());
        let has_insert_idx = self.find_insert_idx(board_debark_times.clone());
        if let Some(insert_idx) = has_insert_idx {
            self.do_insert(board_debark_times, vehicle_data, insert_idx);
            true
        }
        else {
            false
        }
    }

    // If we are waiting to board a vehicle at `position` at time `waiting_time`
    // return `Some(best_vehicle)`
    // where `best_vehicle` is the vehicle, among those vehicle on which `filter` returns true,
    //  to board that allows to debark at the subsequent positions at the earliest time,
    fn _best_filtered_vehicle_to_board_at_by_linear_search<Filter>(&self, 
        waiting_time : & Time, 
        position : & Position,
        filter : Filter
    ) -> Option<Vehicle> 
    where Filter : Fn(&VehicleData) -> bool
    {
        self.board_times_by_position[position.idx].iter()
            .zip(self.vehicles_data.iter())
            .enumerate()
            .filter(|(_, (_, vehicle_data)) | filter(vehicle_data))
            .find_map(|(idx, (board_time, _)) | {
                if waiting_time <= board_time {
                    let vehicle = Vehicle { idx };
                    Some(vehicle)
                }
                else {
                    None
                }
            })
    }

    // If we are waiting to board a vehicle at `position` at time `waiting_time`
    // return `Some(best_vehicle)`
    // where `best_vehicle` is the vehicle, among those vehicle on which `filter` returns true,
    //  to board that allows to debark at the subsequent positions at the earliest time,
    fn best_filtered_vehicle_to_board_at<Filter>(&self, 
        waiting_time : & Time, 
        position : & Position,
        filter : Filter
    ) -> Option<Vehicle> 
    where Filter : Fn(&VehicleData) -> bool
    {
        let search_result = self.board_times_by_position[position.idx].binary_search(waiting_time);
        let first_boardable_vehicle = match search_result {
            // here it means that 
            //    waiting_time < board_time(idx)    if idx < len
            //    waiting_time > board_time(idx -1) if idx > 0
            // so idx is indeed the first vehicle that can be boarded 
            Err(idx) => {
                idx
            },
            // here it means that 
            //    waiting_time == board_time(idx)
            // but maybe idx is not the smallest idx such hat waiting_time == board_time(idx)
            Ok(idx) => {
                let mut first_idx = idx;
                while first_idx > 0 && self.board_times_by_position[position.idx][first_idx] == *waiting_time {
                    first_idx = first_idx - 1;
                }
                first_idx
            }
        };


        for vehicle_idx in first_boardable_vehicle..self.nb_of_vehicles() {
            let vehicle_data = &self.vehicles_data[vehicle_idx];
            let board_time = &self.board_times_by_position[position.idx][vehicle_idx];
            if filter(vehicle_data) && waiting_time <= board_time {
                let vehicle = Vehicle{ idx : vehicle_idx};
                return Some(vehicle);
            }
        }
        return None;
    }

}



pub struct PositionPair {
    pub upstream : usize,
    pub downstream : usize,
}

pub enum VehicleTimesError {
    DebarkBeforeUpstreamBoard(PositionPair), // board_time[upstream] > debark_time[downstream]
    DecreasingBoardTime(PositionPair),  // board_time[upstream] > board_time[downstream] 
    DecreasingDebarkTime(PositionPair)  // debark_time[upstream] > debark_time[downstream] 
}

fn is_increasing<EnumeratedValues>(mut enumerated_values : EnumeratedValues) -> Result<(), (usize, usize) >
where EnumeratedValues : Iterator<Item = (usize, Time)>
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

// Retuns 
//    - Some(Equal)   if lower[i] == upper[i] for all i
//    - Some(Less)    if lower[i] <= upper[i] for all i
//    - Some(Greater) if lower[i] >= upper[i] for all i
//    - None otherwise (the two vector are not comparable)
fn partial_cmp<Lower, Upper, Value> (lower : Lower, upper : Upper) -> Option<Ordering> 
where 
Lower : Iterator<Item = Value> + Clone,
Upper : Iterator<Item = Value> + Clone,
Value : Ord,
{
    debug_assert!( lower.clone().count() == upper.clone().count() );
    let zip_iter = lower.zip(upper);
    let mut first_not_equal_iter = zip_iter.skip_while(|(lower_val, upper_val) | lower_val == upper_val);
    let has_first_not_equal = first_not_equal_iter.next();
    if let Some(first_not_equal) = has_first_not_equal {
        let ordering = {
            let lower_val = first_not_equal.0;
            let upper_val = first_not_equal.1;
            lower_val.cmp(&upper_val)
        };
        debug_assert!( ordering != Ordering::Equal);
        // let's see if there is an index where the ordering is not the same
        // as first_ordering
        let found = first_not_equal_iter.find(|(lower_val, upper_val)| {
            let cmp = lower_val.cmp(&upper_val);
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
    return Some(Ordering::Equal);
    
}
#[derive(Clone)]
struct VehicleTimes<'a> {
    times_by_position : & 'a [Vec<Time>],
    position : usize,
    vehicle : usize
}

impl<'a> Iterator for VehicleTimes<'a> {
    type Item =  Time;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.times_by_position.get(self.position)
            .map( |time_by_vehicles| {
                &time_by_vehicles[self.vehicle]
            });
        if result.is_some() {
            self.position += 1;
        }    
        result.cloned()

    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.times_by_position.len() - self.position;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for VehicleTimes<'a> {

}

