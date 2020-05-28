use std::cmp::Ordering;
use super::data::{Position, StopIdx};


pub struct StopPatternTimetables<VehicleData, Time> {
    pub (super) stops : Vec<StopIdx>,
    pub (super) timetables : Vec<OrderedTimetable<VehicleData, Time>>,
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

}

pub struct TimeTableIdx {
    idx : usize,
}

pub struct VehicleIdx {
    vehicle_idx : usize
}

impl<VehicleData, Time> StopPatternTimetables<VehicleData, Time> {
    pub fn new(stops : Vec<StopIdx>) -> Self {
        assert!( stops.len() >= 2);
        Self{
            stops,
            timetables : Vec::new(),
        }
    }

    pub fn get_stop_at(&self, position : & Position) -> StopIdx {
        self.stops[position.idx]
    }

    pub fn nb_of_positions(&self) -> usize {
        return self.stops.len()
    }

    pub fn insert<'a, 'b, DebarkTimes, BoardTimes >(& mut self,  
        debark_times : DebarkTimes, 
        board_times : BoardTimes, 
        vehicle_data : VehicleData)
    where 
    DebarkTimes : Iterator<Item = Time> +  Clone,
    BoardTimes : Iterator<Item = Option<Time>> +  Clone,
    Time : Ord + Clone
    {
        debug_assert!(self.nb_of_positions() == debark_times.clone().count());
        debug_assert!(self.nb_of_positions() == board_times.clone().count());
        for timetable in & mut self.timetables {
            if timetable.accept(debark_times.clone()) {
                timetable.insert(debark_times, board_times, vehicle_data);
                return;
            }
        }
        let mut new_timetable = OrderedTimetable::new(self.nb_of_positions());
        new_timetable.insert(debark_times, board_times, vehicle_data);
        self.timetables.push(new_timetable);
    }

}



impl<VehicleData, Time> OrderedTimetable<VehicleData, Time>
where Time : Ord + Clone 
{

    fn new(nb_of_positions : usize) -> Self {
        assert!( nb_of_positions >= 2);
        OrderedTimetable{
            vehicles_data : Vec::new(),
            debark_times_by_vehicle : Vec::new(),
            board_times_by_position : vec![Vec::new(); nb_of_positions],
        }
    }

    fn nb_of_positions(&self) -> usize {
        self.board_times_by_position.len()
    }

    fn nb_of_vehicles(&self) -> usize {
        self.vehicles_data.len()
    }

    fn debark_time_at(&self, vehicle_idx : usize, pos_idx : usize) -> & Time {
        &self.debark_times_by_vehicle[vehicle_idx][pos_idx]
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

        for (pos, board_time) in board_times.enumerate() {
            self.board_times_by_position[pos].insert(insert_idx, board_time.clone());
        }

        self.debark_times_by_vehicle.insert(insert_idx, debark_times.map(|time| time.clone()).collect());
        self.vehicles_data.insert(insert_idx, vehicle_data);

    }

    // If we are waiting to board a vehicle at `position` at time `waiting_time`
    // will return the index of the vehicle to board that allows to debark
    // at the subsequent positions at the earliest time.
    // Note : this may NOT be the vehicle with the earliest boarding time
    // Returns None if no vehicle can be boarded after `waiting_time`
    fn get_idx_of_best_vehicle_to_board_at(&self, waiting_time : & Time, position : usize) -> Option<usize> {
        self.board_times_by_position[position]
            .iter()
            .enumerate()
            .find_map(|(idx, has_board_time)| {
                match has_board_time {
                    Some(board_time) if waiting_time <= board_time => {
                        Some(idx)
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
    fn get_idx_of_best_filtered_vehicle_to_board_at<Filter>(&self, 
        waiting_time : & Time, 
        position : usize,
        filter : Filter
    ) -> Option<usize> 
    where Filter : Fn(&VehicleData) -> bool
    {
        self.board_times_by_position[position].iter()
            .zip(self.vehicles_data.iter())
            .enumerate()
            .filter(|(_, (_, vehicle_data)) | filter(vehicle_data))
            .find_map(|(idx, (has_board_time, _)) | {
                match has_board_time {
                    Some(board_time) if waiting_time <= board_time => {
                        Some(idx)
                    },
                    _ => None
                }
            })
    }


}
