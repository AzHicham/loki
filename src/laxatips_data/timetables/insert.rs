use crate::laxatips_data::transit_data::{Stop};
use crate::laxatips_data::calendar::{DaysPattern};
use std::cmp::Ordering;
use std::iter::{Chain, Map};
use std::ops::Range;

use transit_model::objects::{VehicleJourney};
use transit_model::model::Model;
use typed_index_collection::{Idx};

use crate::laxatips_data::time::SecondsSinceTimezonedDayStart as Time;
use chrono_tz::Tz as TimeZone;
use std::collections::BTreeMap;

use super::timetables_data::*;

impl Timetables {


    // Insert in the vehicle in a timetable if
    // the given debark_times and board_times are coherent.
    // Returns a VehicleTimesError otherwise.
    pub fn insert<BoardDebarkTimes>(
        &mut self,
        stop_flows : StopFlows,
        board_debark_times: BoardDebarkTimes,
        timezone : & TimeZone,
        vehicle_data: VehicleData,
    ) -> Result<Timetable, VehicleTimesError>
    where
        BoardDebarkTimes: Iterator<Item = (Time, Time)> + ExactSizeIterator + Clone,
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

        let corrected_board_debark_times = board_debark_times.zip(stop_flows.iter()).map(
            |((board_time, debark_time), (_,flow_direction))| match flow_direction {
                FlowDirection::BoardAndDebark => (board_time, debark_time),
                FlowDirection::BoardOnly => (board_time, board_time),
                FlowDirection::DebarkOnly => (debark_time, debark_time),
            },
        );
        let stop_flows_timetables = self.stop_flows_to_timetables.entry(stop_flows).or_insert(Vec::new());

        for timetable in stop_flows_timetables {
            let timetable_data = & mut self.timetable_datas[timetable.idx];
            let inserted = timetable_data
                .try_insert(corrected_board_debark_times.clone(), timezone, vehicle_data.clone());
            if inserted {
                return Ok(timetable.clone());
            }
        }
        let mut new_timetable_data = TimetableData::new(stop_flows, timezone, corrected_board_debark_times, vehicle_data);
        let timetable = Timetable{ idx : self.timetable_datas.len() };
        self.timetable_datas.push(new_timetable_data);
        stop_flows_timetables.push(timetable);
        Ok(timetable)
    }
}

impl TimetableData {

    pub  fn new<BoardDebarkTimes>(
        stop_flows : StopFlows, 
        timezone : & TimeZone,
        board_debark_times: BoardDebarkTimes,
        vehicle_data: VehicleData,
    ) -> Self 
    where
        BoardDebarkTimes: Iterator<Item = (Time, Time)> + ExactSizeIterator + Clone,
    {
        let nb_of_positions = stop_flows.len();
        assert!(nb_of_positions >= 2);
        let mut result = Self {
            stop_flows,
            timezone : timezone.clone(),
            vehicles_data: Vec::new(),
            debark_times_by_position: vec![Vec::new(); nb_of_positions],
            board_times_by_position: vec![Vec::new(); nb_of_positions],
            latest_board_time_by_position: vec![Time::zero(); nb_of_positions],
        };
        result.do_insert(board_debark_times, vehicle_data, 0);
        result
    }

    // Try to insert the vehicle in this timetable
    // Returns `true` if insertion was succesfull, `false` otherwise
    fn try_insert<BoardDebarkTimes>(
        &mut self,
        board_debark_times: BoardDebarkTimes,
        timezone : & TimeZone,
        vehicle_data: VehicleData,
    ) -> bool
    where
        BoardDebarkTimes: Iterator<Item = (Time, Time)> + ExactSizeIterator + Clone,
    {
        if self.timezone != *timezone {
            return false;
        }
        assert!(board_debark_times.len() == self.nb_of_positions());
        let has_insert_idx = self.find_insert_idx(board_debark_times.clone());
        if let Some(insert_idx) = has_insert_idx {
            self.do_insert(board_debark_times, vehicle_data, insert_idx);
            true
        } else {
            false
        }
    }

    fn find_insert_idx<BoardDebarkTimes>(
        &self,
        board_debark_times: BoardDebarkTimes,
    ) -> Option<usize>
    where
        BoardDebarkTimes: Iterator<Item = (Time, Time)> + ExactSizeIterator + Clone,
    {
        let nb_of_vehicles = self.nb_of_vehicles();
        if nb_of_vehicles == 0 {
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
                if insert_idx < self.nb_of_vehicles() {
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
        BoardDebarkTimes: Iterator<Item = (Time, Time)> + ExactSizeIterator + Clone,
    {
        let nb_of_vehicles = self.nb_of_vehicles();
        assert!(start_search_idx < nb_of_vehicles);

        let board_then_debark = board_debark_times
            .clone()
            .map(|(board, _)| board)
            .chain(board_debark_times.map(|(_, debark)| debark));

        let first_vehicle_idx = start_search_idx;
        let has_first_vehicle_comp = partial_cmp(
            board_then_debark.clone(),
            self.vehicle_board_then_debark_times(first_vehicle_idx),
        );
        // if the candidate_times_vector is not comparable with first_vehicle_times_vector
        // then we cannot add the candidate to this timetable
        let first_vehicle_comp = has_first_vehicle_comp?;
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
            let has_vehicle_comp = partial_cmp(
                board_then_debark.clone(),
                self.vehicle_board_then_debark_times(vehicle_idx),
            );
            // if the candidate_times_vector is not comparable with vehicle_times_vector
            // then we cannot add the candidate to this timetable
            let vehicle_comp = has_vehicle_comp?;

            if vehicle_comp == Ordering::Less || vehicle_comp == Ordering::Equal {
                return Some(vehicle_idx);
            }
            assert!(vehicle_comp == Ordering::Greater);
        }

        // here  candidate_times_vector  >= vehicle_times_vector for all vehicles,
        // so we can insert the candidate as the last vehicle
        Some(nb_of_vehicles)
    }


    fn do_insert<BoardDebarkTimes>(
        &mut self,
        board_debark_times: BoardDebarkTimes,
        vehicle_data: VehicleData,
        insert_idx: usize,
    ) where
        BoardDebarkTimes: Iterator<Item = (Time, Time)> + ExactSizeIterator + Clone,
    {
        if insert_idx < self.nb_of_vehicles() {
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
            let latest_board_time = &mut self.latest_board_time_by_position[position];
            *latest_board_time = std::cmp::max(*latest_board_time, board_time);
        }
        self.vehicles_data.insert(insert_idx, vehicle_data);
    }


}

pub struct PositionPair {
    pub upstream: usize,
    pub downstream: usize,
}

pub enum VehicleTimesError {
    DebarkBeforeUpstreamBoard(PositionPair), // board_time[upstream] > debark_time[downstream]
    DecreasingBoardTime(PositionPair),       // board_time[upstream] > board_time[downstream]
    DecreasingDebarkTime(PositionPair),      // debark_time[upstream] > debark_time[downstream]
}

fn is_increasing<EnumeratedValues>(
    mut enumerated_values: EnumeratedValues,
) -> Result<(), (usize, usize)>
where
    EnumeratedValues: Iterator<Item = (usize, Time)>,
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
