use std::cmp::Ordering;
use super::data::{StopIdx};
use std::ops::Range;
use std::iter::Map;
use std::collections::BTreeMap;

pub struct StopPatternTimetables<VehicleData, Time> {
    pub stops : Vec<StopIdx>,
    pub stops_to_position : BTreeMap<StopIdx, Position>,
    pub timetables : Vec<OrderedTimetable<VehicleData, Time>>,
}


// TODO : document more explicitely !
pub struct OrderedTimetable<VehicleData, Time> {
    // vehicle data, ordered by increasing debark times
    // meaning that is v1 is before v2 in this vector,
    // then for all `position` we have 
    //    debark_time_by_vehicle[v1][position] <= debark_time_by_vehicle[v2][position]
    vehicles_data : Vec<VehicleData>,


    // debark_time_by_vehicle[vehicle][position] 
    // is the time at which a traveler in `vehicle` 
    // will debark at `position`
    debark_times_by_vehicle : Vec<Vec<Time>>, 

    // board_times_by_position[position][vehicle]
    // is the time at which a traveler waiting
    // at `position` can board `vehicle`
    // None if `vehicle` cannot be boarded at `position` 
    board_times_by_position : Vec<Vec<Option<Time>>>, 

    latest_board_time_by_position : Vec<Option<Time>>, 

}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Position {
    idx : usize,
}

#[derive(Debug, PartialEq, Eq, Clone, Ord, PartialOrd)]
pub struct TimeTableIdx {
    idx : usize,
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct VehicleIdx {
    idx : usize
}

pub type TimeTablesIter = Map<Range<usize>, fn(usize) -> TimeTableIdx >;

impl<VehicleData, Time> StopPatternTimetables<VehicleData, Time>
where Time : Ord + Clone
{
    pub fn new(stops : Vec<StopIdx>) -> Self {
        assert!( stops.len() >= 2);
        let mut stops_to_position = BTreeMap::new();
        for (pos_idx, stop) in stops.iter().enumerate() {
            let pos = Position{ idx : pos_idx};
            stops_to_position.insert(stop.clone(), pos);
        }
        Self{
            stops,
            stops_to_position,
            timetables : Vec::new(),
        }
    }

    pub fn position(&self, stop : & StopIdx) -> &Position {
        self.stops_to_position.get(stop)
            .unwrap_or_else( || panic!(format!(
                "The stop {:?} is expected to belongs to the stop_pattern ", 
                    *stop)
                )
            )
    }

    pub fn is_upstream(&self, upstream : & StopIdx, downstream : & StopIdx) -> bool {
        let upstream_position = self.position(upstream);
        let downstream_position = self.position(downstream);
        upstream_position.idx < downstream_position.idx
    }


    pub fn next_stop(&self, stop : & StopIdx) -> Option<&StopIdx> {
        let position = self.position(stop);
        self.stops.get(position.idx + 1)
    }

    pub fn is_last_stop(&self, stop : & StopIdx) -> bool {
        let position = self.position(stop);
        position.idx == self.stops.len() - 1
        
    }


    pub fn timetable<'a>(& 'a self, timetable : & TimeTableIdx) -> & 'a OrderedTimetable<VehicleData, Time> {
        self.timetables.get(timetable.idx)
            .unwrap_or_else( || panic!(format!(
                "The timetable {:?} is expected to belongs to the stop_pattern ", 
                    *timetable)
                )
            )
    }

    pub fn timetables(&self) -> TimeTablesIter {
        (0..self.timetables.len()).map(|idx| {
            TimeTableIdx {
                idx
            }
        })
    }

    pub fn nb_of_timetables(&self) -> usize {
        self.timetables.len()
    }

    pub fn debark_time_at(&self, timetable : & TimeTableIdx, vehicle : & VehicleIdx, stop : & StopIdx) -> & Time {
        let position = self.position(stop);
        let timetable = self.timetable(timetable);
        timetable.debark_time_at_(vehicle.idx, position.idx)
    }

    pub fn board_time_at(&self, timetable : & TimeTableIdx, vehicle: & VehicleIdx, stop : & StopIdx) -> & Option<Time> {
        let position = self.position(stop);
        let timetable = self.timetable(timetable);
        timetable.board_time_at_(vehicle.idx, position.idx)
    }

    pub fn last_board_time_at(&self, timetable : & TimeTableIdx, stop : & StopIdx) -> & Option<Time> {
        let position = self.position(stop);
        let timetable = self.timetable(timetable);
        timetable.latest_board_time_at(position.idx)
    }

    fn nb_of_positions(&self) -> usize {
        self.stops.len()
    }

    // Insert in the vehicle in a timetable if 
    // the given debark_times and board_times are coherent.
    // Returns a VehicleTimesError otherwise.
    pub fn insert<'a, 'b, DebarkTimes, BoardTimes >(& mut self,  
        debark_times : DebarkTimes, 
        board_times : BoardTimes, 
        vehicle_data : VehicleData
    ) -> Result<(), VehicleTimesError>
    where 
    DebarkTimes : Iterator<Item = Time> +  Clone,
    BoardTimes : Iterator<Item = Option<Time>> +  Clone,
    Time : Ord + Clone
    {
        debug_assert!(self.nb_of_positions() == debark_times.clone().count());
        debug_assert!(self.nb_of_positions() == board_times.clone().count());
        if let Err(err) =  is_intertwined(debark_times.clone(), board_times.clone(), self.nb_of_positions()) {
            return Err(err);
        }

        for timetable in & mut self.timetables {
            if timetable.accept(debark_times.clone()) {
                timetable.insert(debark_times, board_times, vehicle_data);
                return Ok(());
            }
        }
        let mut new_timetable = OrderedTimetable::new(self.nb_of_positions());
        new_timetable.insert(debark_times, board_times, vehicle_data);
        self.timetables.push(new_timetable);
        Ok(())
    }

}

pub type VehiclesIter = Map<Range<usize>, fn(usize)->VehicleIdx>;

impl<VehicleData, Time> OrderedTimetable<VehicleData, Time>
where Time : Ord + Clone 
{

    fn new(nb_of_positions : usize) -> Self {
        assert!( nb_of_positions >= 2);
        OrderedTimetable{
            vehicles_data : Vec::new(),
            debark_times_by_vehicle : Vec::new(),
            board_times_by_position : vec![Vec::new(); nb_of_positions],
            latest_board_time_by_position : vec![None; nb_of_positions],
        }
    }

    fn nb_of_positions(&self) -> usize {
        self.board_times_by_position.len()
    }

    fn nb_of_vehicles(&self) -> usize {
        self.vehicles_data.len()
    }

    fn debark_time_at_(&self, vehicle_idx : usize, pos_idx : usize) -> & Time {
        &self.debark_times_by_vehicle[vehicle_idx][pos_idx]
    }

    pub fn debark_time_at(&self, vehicle : & VehicleIdx, position :  & Position) -> &Time {
        self.debark_time_at_(vehicle.idx, position.idx)
    }

    fn board_time_at_(&self, vehicle_idx : usize, pos_idx : usize) -> & Option<Time >{
        &self.board_times_by_position[pos_idx][vehicle_idx]
    }

    pub fn board_time_at(&self, vehicle : & VehicleIdx, position :  & Position) -> &Option<Time> {
        self.board_time_at_(vehicle.idx, position.idx)
    }

    fn latest_board_time_at(&self, pos_idx : usize) -> & Option<Time> {
        & self.latest_board_time_by_position[pos_idx]
    }


    pub fn vehicles(&self) -> VehiclesIter {
        (0..self.nb_of_vehicles()).map(|idx| {
            VehicleIdx{
                idx
            }
        })
    }

    fn vehicle_debark_times<'a>(& 'a self, vehicle_idx : usize) -> & 'a [Time] {
        debug_assert!( vehicle_idx < self.vehicles_data.len() );
        & self.debark_times_by_vehicle[vehicle_idx]
    }

    // If we denote `vehicle_debark_times` the debark times of the vehicle present at `vehicle_idx`, 
    //   then this function returns :
    //    - Some(Equal)   if debark_times[pos] == vehicle_debark_times[pos] for all pos
    //    - Some(Less)    if debark_times[pos] <= vehicle_debark_times[pos] for all pos
    //    - Some(Greater) if debark_times[pos] >= vehicle_debark_times[pos] for all pos
    //    - None otherwise (the two times vector are not comparable)
    fn partial_cmp<DebarkTimes> (&self, vehicle_idx : usize, debark_times : DebarkTimes) -> Option<Ordering> 
    where 
    DebarkTimes : Iterator<Item = Time> + Clone,
    {
        debug_assert!( debark_times.clone().count() == self.nb_of_positions() );
        let vehicle_times = self.vehicle_debark_times(vehicle_idx);
        let zip_iter = debark_times.zip(vehicle_times);
        let mut first_not_equal_iter = zip_iter.skip_while(|(candidate, vehicle) : &(Time, &Time)| candidate == *vehicle);
        let has_first_not_equal = first_not_equal_iter.next();
        if let Some(first_not_equal) = has_first_not_equal {
            let ordering = {
                let candidate = first_not_equal.0;
                let vehicle = first_not_equal.1;
                candidate.cmp(vehicle)
            };
            debug_assert!( ordering != Ordering::Equal);
            // let's see if there is a position where the ordering is not the same
            // as first_ordering
            let found = first_not_equal_iter.find(|(candidate, vehicle)| {
                let cmp = candidate.cmp(&vehicle);
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


    fn accept<DebarkTimes>(& self, debark_times : DebarkTimes) -> bool 
    where 
    DebarkTimes : Iterator<Item =  Time> +  Clone,
    {
        debug_assert!( debark_times.clone().count() == self.nb_of_positions() );
        for vehicle_idx in 0..self.nb_of_vehicles() {
            if self.partial_cmp(vehicle_idx, debark_times.clone()).is_none() {
                return false;
            }
        }
        true
    }

    fn insert<DebarkTimes, BoardTimes >(& mut self,  
            debark_times : DebarkTimes, 
            board_times : BoardTimes, 
            vehicle_data : VehicleData)
    where 
    DebarkTimes : Iterator<Item =  Time> +  Clone,
    BoardTimes : Iterator<Item =  Option<Time> > +  Clone,
    {
        debug_assert!(debark_times.clone().count() == self.nb_of_positions());
        debug_assert!(board_times.clone().count() == self.nb_of_positions());
        debug_assert!(self.accept(debark_times.clone()));
        let nb_of_vehicles = self.nb_of_vehicles();
        // TODO : maybe start testing from the end ?
        // TODO : can be simplified if we know that self.accept(&debark_times) ??
        let insert_idx = (0..nb_of_vehicles).find(|&idx| {
            let partial_cmp = self.partial_cmp(idx, debark_times.clone()); 
            partial_cmp == Some(Ordering::Less)
        })
        .unwrap_or(nb_of_vehicles);

        for (pos, has_board_time) in board_times.enumerate() {
            self.board_times_by_position[pos].insert(insert_idx, has_board_time.clone());
            
            if let Some(board_time) = has_board_time {
                let has_latest_board_time = & mut self.latest_board_time_by_position[pos];
                match has_latest_board_time {
                    None => { *has_latest_board_time = Some(board_time); }
                    Some(lastest_board_time) if *lastest_board_time < board_time => {
                        * has_latest_board_time = Some(board_time);
                    }
                    _ => ()
                }
            }

        }

        self.debark_times_by_vehicle.insert(insert_idx, debark_times.map(|time| time.clone()).collect());
        self.vehicles_data.insert(insert_idx, vehicle_data);

    }

    // If we are waiting to board a vehicle at `position` at time `waiting_time`
    // will return the index of the vehicle to board that allows to debark
    // at the subsequent positions at the earliest time.
    // Note : this may NOT be the vehicle with the earliest boarding time
    // Returns None if no vehicle can be boarded after `waiting_time`
    pub fn best_vehicle_to_board_at(&self, waiting_time : & Time, position : & Position) -> Option<VehicleIdx> {
        self.board_times_by_position[position.idx]
            .iter()
            .enumerate()
            .find_map(|(idx, has_board_time)| {
                match has_board_time {
                    Some(board_time) if waiting_time <= board_time => {
                        let vehicle = VehicleIdx {
                            idx
                        };
                        Some(vehicle)
                    },
                    _ => None
                }
            })
        
    }


    // If we are waiting to board a vehicle at `position` at time `waiting_time`
    // will return the index of the vehicle, among those vehicle on which `filter` returns true,
    //  to board that allows to debark at the subsequent positions at the earliest time.
    // Note : this may NOT be the vehicle with the earliest boarding time
    // Returns None if no vehicle can be boarded after `waiting_time`
    pub fn best_filtered_vehicle_to_board_at<Filter>(&self, 
        waiting_time : & Time, 
        position : & Position,
        filter : Filter
    ) -> Option<VehicleIdx> 
    where Filter : Fn(&VehicleData) -> bool
    {
        self.board_times_by_position[position.idx].iter()
            .zip(self.vehicles_data.iter())
            .enumerate()
            .filter(|(_, (_, vehicle_data)) | filter(vehicle_data))
            .find_map(|(idx, (has_board_time, _)) | {
                match has_board_time {
                    Some(board_time) if waiting_time <= board_time => {
                        let vehicle = VehicleIdx { idx };
                        Some(vehicle)
                    },
                    _ => None
                }
            })
    }
}



pub enum VehicleTimesError {
    BoardBeforeDebark(usize),            // board_time[idx] < debark_time[idx]
    NextDebarkIsBeforeBoard(usize),      // board_time[idx] > debark_time[idx+1]
    NextDebarkIsBeforePrevDebark(usize)  // debark_time[idx] > debark_time[idx + 1]
}

// check that 
//  - debark_times[i] <= debark_times[i+1] for all i >= 1 (as debark_time[0] has no meaning)
//  - debark_times[i] <= board_times[i] for all i >= 1 such that board_times[i] is not None
//  - board_times[i] <= debark_times[i+1] for all i < len - 2 such that board_times[i] is not None
fn is_intertwined<DebarkTimes, BoardTimes, Time>(
    debark_times : DebarkTimes, 
    board_times : BoardTimes,
    len : usize,
) ->  Result<(), VehicleTimesError>
where 
DebarkTimes : Iterator<Item = Time> +  Clone,
BoardTimes : Iterator<Item = Option<Time>> +  Clone,
Time : Ord + Clone
{
    debug_assert!(board_times.clone().count() == debark_times.clone().count());
    debug_assert!(board_times.clone().count() == len);
    debug_assert!(len >= 2);
    let mut iter = board_times.zip(debark_times).enumerate();
    let (mut prev_idx, (mut has_prev_board, mut prev_debark) ) = iter.next().unwrap();

    while let Some((curr_idx, (has_curr_board, curr_debark))) = iter.next() {
        if let Some(prev_board) = has_prev_board {
            if prev_board > curr_debark {
                return Err(VehicleTimesError::NextDebarkIsBeforeBoard(prev_idx));
            }

        }
        // the last board time has no meaning
        // so we do not check if
        // last_debark_time <= last_board_time
        if curr_idx < len - 1 {
            if let Some(curr_board) = has_curr_board.clone() {
                if curr_board < curr_debark {
                    return Err(VehicleTimesError::BoardBeforeDebark(curr_idx));
                }
            }
        }

        // the first debark time has no meaning, so we do not check
        // if debark_time[0] <= debark_time[1]
        if prev_idx >= 1 {
            if prev_debark > curr_debark {
                return Err(VehicleTimesError::NextDebarkIsBeforePrevDebark(prev_idx));
            }
        }

        prev_idx = curr_idx;
        has_prev_board = has_curr_board;
        prev_debark = curr_debark;

    }

    Ok(())

}
