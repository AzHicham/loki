use std::cmp::Ordering;
use super::data::Position;


// TODO : document more explicitely !

struct OrderedTimetable<VehicleData, Time> {
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
    board_times_by_position : Vec<Vec<Time>>, 

}







impl<ItemData, Time> OrderedTimetable<ItemData, Time>
where Time : Ord + Clone 
{

    fn new(nb_of_positions : usize) -> Self {
        assert!( nb_of_positions >= 1);
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
    //    - Some(Equal) if vehicle_debark_times[pos] == debark_times[pos] for all pos
    //    - Some(Lower) if vehicle_debark_times[pos] <= debark_times[pos] for all pos
    //    - Some(Upper) if vehicle_debark_times[pos] >= debark_times[pos] for all pos
    //    - None otherwise (the two times vector are not comparable)
    fn partial_cmp<'a, DebarkTimes> (&self, vehicle_idx : usize, debark_times : DebarkTimes) -> Option<Ordering> 
    where 
    DebarkTimes : Iterator<Item = & 'a Time> + ExactSizeIterator,
    Time : 'a
    {
        debug_assert!( debark_times.len() == self.nb_of_positions() );
        let item_values = self.vehicle_debark_times(vehicle_idx);
        let zip_iter = debark_times.zip(item_values);
        let mut first_not_equal_iter = zip_iter.skip_while(|&(left, right) : &(&Time, &Time)| left == right);
        let has_first_not_equal = first_not_equal_iter.next();
        if let Some(first_not_equal) = has_first_not_equal {
            let ordering = first_not_equal.0.cmp(&first_not_equal.1);
            assert!( ordering != Ordering::Equal);
            // let's see if there is a position where the ordering is not the same
            // as first_ordering
            let found = first_not_equal_iter.find(|(left, right)| {
                let cmp = left.cmp(&right);
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


    fn accept<'a, DebarkTimes>(& self, debark_times : DebarkTimes) -> bool 
    where 
    DebarkTimes : Iterator<Item = & 'a Time> + ExactSizeIterator + Clone,
    Time : 'a
    {
        debug_assert!( debark_times.len() == self.nb_of_positions() );
        for vehicle_idx in 0..self.nb_of_vehicles() {
            if self.partial_cmp(vehicle_idx, debark_times.clone()).is_none() {
                return false;
            }
        }
        true
    }

    fn insert<'a, 'b, DebarkTimes, BoardTimes >(& mut self,  
            debark_times : DebarkTimes, 
            board_times : BoardTimes, 
            vehicle_data : ItemData)
    where 
    DebarkTimes : Iterator<Item = & 'a Time> + ExactSizeIterator + Clone,
    BoardTimes : Iterator<Item = & 'b Time> + ExactSizeIterator + Clone,
    Time : 'a + 'b
    {
        debug_assert!(debark_times.len() == self.nb_of_positions());
        debug_assert!(board_times.len() == self.nb_of_positions());
        debug_assert!(self.accept(debark_times.clone()));
        let nb_of_vehicles = self.nb_of_vehicles();
        // TODO : maybe start testing from the end ?
        // TODO : can be simplified if we know that self.accept(&debark_times) ??
        let insert_idx = (0..nb_of_vehicles).find(|&idx| {
            let partial_cmp = self.partial_cmp(idx, debark_times.clone()); 
            partial_cmp ==  Some(Ordering::Equal)
            || partial_cmp == Some(Ordering::Greater)
        })
        .map(|idx| std::cmp::max(idx-1, 0) )
        .unwrap_or(nb_of_vehicles);

        for (pos, board_time) in board_times.enumerate() {
            self.board_times_by_position[pos].insert(insert_idx, board_time.clone());
        }

        self.debark_times_by_vehicle.insert(insert_idx, debark_times.map(|time| time.clone()).collect());
        self.vehicles_data.insert(insert_idx, vehicle_data);

    }

    // 
    fn get_best_vehicle_to_board_at(&self, waiting_time : & Time, position : usize) -> Option<usize> {
        let idx = self.board_times_by_position[position].iter().enumerate().find(|&(_, board_time)| {
            waiting_time >= board_time
        })
        .map(|(vehicle_idx, board_time)| {
            if *board_time == *waiting_time {
                vehicle_idx
            }
            else {
                std::cmp::max(vehicle_idx - 1, 0)
            }
        });
        idx
    }

}
