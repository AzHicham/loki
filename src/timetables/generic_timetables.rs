// Copyright  (C) 2020, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Hove (www.kisio.com).
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

use std::{collections::BTreeMap, fmt::Debug};
use tracing::debug;
use FlowDirection::{BoardAndDebark, BoardOnly, DebarkOnly, NoBoardDebark};

use crate::{
    models::StopTimeIdx,
    time::DaysSinceDatasetStart,
    timetables::{FlowDirection, StopFlows},
    transit_data::Stop,
};

#[derive(Debug)]
pub(super) struct GenericTimetables<Time, Load, VehicleData> {
    pub(super) stop_flows_to_timetables: BTreeMap<StopFlows, Vec<Timetable>>,
    pub(super) timetable_datas: Vec<TimetableData<Time, Load, VehicleData>>,
}

#[derive(Debug)]
pub(super) struct TimetableData<Time, Load, VehicleData> {
    pub(super) stop_flows: StopFlows,

    /// vehicle data, ordered by increasing times
    /// meaning that if vehicle_1 is before vehicle_2 in this vector,
    /// then for all `position` we have
    ///    debark_times_by_position[position][vehicle_1] <= debark_times_by_position[position][vehicle_2]
    pub(super) vehicle_datas: Vec<VehicleData>,

    /// `vehicle_loads[vehicle][position]` is the load in vehicle
    /// between `position` and `position +1`
    pub(super) vehicle_loads: Vec<Vec<Load>>,

    /// `board_times_by_position[position][vehicle]`
    ///   is the time at which a traveler waiting
    ///   at `position` can board `vehicle`
    /// Vehicles are ordered by increasing time
    ///  so for each `position` the vector
    ///  board_times_by_position[position] is sorted by increasing times
    pub(super) board_times_by_position: Vec<Vec<Time>>,

    /// `debark_times_by_position[position][vehicle]`
    ///   is the time at which a traveler being inside `vehicle`
    ///   will debark at `position`
    /// Vehicles are ordered by increasing time
    ///  so for each `position` the vector
    ///  debark_times_by_position[position] is sorted by increasing times
    pub(super) debark_times_by_position: Vec<Vec<Time>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Timetable {
    pub(super) idx: usize,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub(super) struct PositionIdx {
    pub(super) idx: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Position {
    pub(super) timetable: Timetable,
    pub(super) idx: PositionIdx,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub(super) struct VehicleIdx {
    pub(super) idx: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Vehicle {
    pub(super) timetable: Timetable,
    pub(super) idx: VehicleIdx,
}

#[derive(Debug, Clone)]
pub struct Trip {
    pub(super) vehicle: Vehicle,
    pub(super) day: DaysSinceDatasetStart,
}

impl<Time, Load, VehicleData> GenericTimetables<Time, Load, VehicleData>
where
    Time: Ord + Clone + Debug,
    Load: Ord + Clone + Debug,
{
    pub(super) fn new() -> Self {
        Self {
            stop_flows_to_timetables: BTreeMap::new(),
            timetable_datas: Vec::new(),
        }
    }

    pub(super) fn nb_of_timetables(&self) -> usize {
        self.timetable_datas.len()
    }

    pub(super) fn timetable_data(
        &self,
        timetable: &Timetable,
    ) -> &TimetableData<Time, Load, VehicleData> {
        &self.timetable_datas[timetable.idx]
    }

    pub(super) fn timetable_data_mut(
        &mut self,
        timetable: &Timetable,
    ) -> &mut TimetableData<Time, Load, VehicleData> {
        &mut self.timetable_datas[timetable.idx]
    }

    pub(super) fn vehicle_data(&self, vehicle: &Vehicle) -> &VehicleData {
        self.timetable_data(&vehicle.timetable)
            .vehicle_data(vehicle.idx)
    }

    pub(super) fn stoptime_idx(&self, position: &Position) -> usize {
        position.idx.idx
    }

    pub(super) fn timetable_of(&self, vehicle: &Vehicle) -> Timetable {
        vehicle.timetable.clone()
    }

    pub(super) fn stop_at(&self, position: &Position, timetable: &Timetable) -> &Stop {
        assert!(*timetable == position.timetable);
        self.timetable_data(timetable).stop_at(position.idx)
    }

    pub(super) fn is_upstream(
        &self,
        upstream: &Position,
        downstream: &Position,
        timetable: &Timetable,
    ) -> bool {
        assert!(upstream.timetable == *timetable);
        upstream.idx.idx < downstream.idx.idx
    }

    pub(super) fn first_position(&self, timetable: &Timetable) -> Position {
        assert!(self.timetable_data(timetable).nb_of_positions() > 0);
        Position {
            timetable: timetable.clone(),
            idx: PositionIdx { idx: 0 },
        }
    }

    pub(super) fn last_position(&self, timetable: &Timetable) -> Position {
        let nb_of_positions = self.timetable_data(timetable).nb_of_positions();
        assert!(nb_of_positions > 0);
        Position {
            timetable: timetable.clone(),
            idx: PositionIdx {
                idx: nb_of_positions - 1,
            },
        }
    }

    pub(super) fn next_position(
        &self,
        position: &Position,
        timetable: &Timetable,
    ) -> Option<Position> {
        assert!(position.timetable == *timetable);
        let idx = position.idx.idx;
        if idx + 1 < self.timetable_data(&position.timetable).nb_of_positions() {
            let result = Position {
                timetable: position.timetable.clone(),
                idx: PositionIdx { idx: idx + 1 },
            };
            Some(result)
        } else {
            None
        }
    }

    pub(super) fn previous_position(
        &self,
        position: &Position,
        timetable: &Timetable,
    ) -> Option<Position> {
        assert_eq!(position.timetable, *timetable);
        let idx = position.idx.idx;
        if idx >= 1 {
            let result = Position {
                timetable: position.timetable.clone(),
                idx: PositionIdx { idx: idx - 1 },
            };
            Some(result)
        } else {
            None
        }
    }

    pub(super) fn debark_time(&self, vehicle: &Vehicle, position: &Position) -> Option<&Time> {
        assert!(vehicle.timetable == position.timetable);
        self.timetable_data(&vehicle.timetable)
            .debark_time(vehicle.idx, position.idx)
    }

    pub(super) fn board_time(&self, vehicle: &Vehicle, position: &Position) -> Option<&Time> {
        assert!(vehicle.timetable == position.timetable);
        self.timetable_data(&vehicle.timetable)
            .board_time(vehicle.idx, position.idx)
    }

    pub(super) fn arrival_time(&self, vehicle: &Vehicle, position: &Position) -> &Time {
        assert!(vehicle.timetable == position.timetable);
        self.timetable_data(&vehicle.timetable)
            .arrival_time(vehicle.idx, position.idx)
    }

    pub(super) fn load_before(&self, vehicle: &Vehicle, position: &Position) -> &Load {
        assert!(vehicle.timetable == position.timetable);
        self.timetable_data(&vehicle.timetable)
            .load_before(vehicle.idx, position.idx)
    }

    pub(super) fn departure_time(&self, vehicle: &Vehicle, position: &Position) -> &Time {
        assert!(vehicle.timetable == position.timetable);
        self.timetable_data(&vehicle.timetable)
            .departure_time(vehicle.idx, position.idx)
    }

    pub(super) fn load_after(&self, vehicle: &Vehicle, position: &Position) -> &Load {
        assert!(vehicle.timetable == position.timetable);
        self.timetable_data(&vehicle.timetable)
            .load_after(vehicle.idx, position.idx)
    }

    pub(super) fn earliest_vehicle_to_board<Filter>(
        &self,
        waiting_time: &Time,
        timetable: &Timetable,
        position: &Position,
        filter: Filter,
    ) -> Option<Vehicle>
    where
        Filter: Fn(&VehicleData) -> bool,
    {
        assert!(position.timetable == *timetable);
        self.timetable_data(timetable)
            .earliest_vehicle_to_board(waiting_time, position.idx, &filter)
            .map(|idx| Vehicle {
                timetable: timetable.clone(),
                idx,
            })
    }

    pub(super) fn latest_vehicle_that_debark<Filter>(
        &self,
        time: &Time,
        timetable: &Timetable,
        position: &Position,
        filter: Filter,
    ) -> Option<Vehicle>
    where
        Filter: Fn(&VehicleData) -> bool,
    {
        assert_eq!(position.timetable, *timetable);
        self.timetable_data(timetable)
            .latest_vehicle_that_debark(time, position.idx, filter)
            .map(|idx| Vehicle {
                timetable: timetable.clone(),
                idx,
            })
    }

    pub fn nb_of_trips(&self) -> usize {
        self.timetable_datas
            .iter()
            .map(TimetableData::nb_of_vehicle)
            .sum()
    }

    // Insert in the trip in a timetable if
    // the given debark_times, board_times and loads are coherent.
    // Returns a VehicleTimesError otherwise.
    pub fn insert<BoardTimes, DebarkTimes, Loads, Stops, Flows>(
        &mut self,
        stops: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads: Loads,
        vehicle_data: VehicleData,
    ) -> Result<Timetable, VehicleTimesError>
    where
        BoardTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
        Loads: Iterator<Item = Load> + ExactSizeIterator + Clone,
        Stops: Iterator<Item = Stop> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        Time: Clone,
        VehicleData: Clone,
    {
        let nb_of_positions = stops.len();
        assert!(nb_of_positions == flows.len());
        assert!(nb_of_positions == board_times.len());
        assert!(nb_of_positions == debark_times.len());
        assert!(nb_of_positions == loads.len() + 1);
        inspect(flows.clone(), board_times.clone(), debark_times.clone())?;

        let corrected_board_debark_times = board_times.zip(debark_times).zip(flows.clone()).map(
            |((board_time, debark_time), flow_direction)| match flow_direction {
                BoardOnly => (board_time.clone(), board_time),
                DebarkOnly => (debark_time.clone(), debark_time),
                BoardAndDebark | NoBoardDebark => (board_time, debark_time),
            },
        );
        let corrected_board_times = corrected_board_debark_times.clone().map(|(board, _)| board);
        let corrected_debark_times = corrected_board_debark_times.map(|(_, debark)| debark);
        let stop_flows: Vec<(Stop, FlowDirection)> = stops.zip(flows).collect();
        let stop_flows_timetables = self
            .stop_flows_to_timetables
            .entry(stop_flows.clone())
            .or_insert_with(Vec::new);

        for timetable in stop_flows_timetables.iter() {
            let timetable_data = &mut self.timetable_datas[timetable.idx];
            let is_inserted = timetable_data.try_insert(
                corrected_board_times.clone(),
                corrected_debark_times.clone(),
                loads.clone(),
                vehicle_data.clone(),
            );
            if is_inserted {
                return Ok(timetable.clone());
            }
        }
        let new_timetable_data = TimetableData::new(
            stop_flows,
            corrected_board_times,
            corrected_debark_times,
            loads,
            vehicle_data,
        );
        let timetable = Timetable {
            idx: self.timetable_datas.len(),
        };
        self.timetable_datas.push(new_timetable_data);
        stop_flows_timetables.push(timetable.clone());
        Ok(timetable)
    }
}

fn is_increasing<EnumeratedValues, Value>(
    mut enumerated_values: EnumeratedValues,
) -> Result<(), PositionPair>
where
    EnumeratedValues: Iterator<Item = (usize, Value)>,
    Value: Ord,
{
    let has_previous = enumerated_values.next();
    if let Some((mut prev_position, mut prev_value)) = has_previous {
        for (position, value) in enumerated_values {
            if value < prev_value {
                let pair = PositionPair {
                    upstream: StopTimeIdx { idx: prev_position },
                    downstream: StopTimeIdx { idx: position },
                };
                return Err(pair);
            }
            prev_position = position;
            prev_value = value;
        }
    } else {
        debug!("Called is_increasing on an empty sequence of values.");
    }

    Ok(())
}

pub(super) fn inspect<BoardTimes, DebarkTimes, Flows, Time>(
    flows: Flows,
    board_times: BoardTimes,
    debark_times: DebarkTimes,
) -> Result<(), VehicleTimesError>
where
    BoardTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
    DebarkTimes: Iterator<Item = Time> + ExactSizeIterator + Clone,
    Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
    Time: Ord + Clone,
{
    assert!(flows.len() == board_times.len());
    assert!(flows.len() == debark_times.len());
    if flows.len() < 2 {
        return Err(VehicleTimesError::LessThanTwoStops);
    }

    let valid_enumerated_board_times = board_times
        .clone()
        .zip(flows.clone())
        .enumerate()
        .filter_map(
            |(position, (board_time, flow_direction))| match flow_direction {
                BoardOnly | BoardAndDebark => Some((position, board_time)),
                NoBoardDebark | DebarkOnly => None,
            },
        );

    if let Err(position_pair) = is_increasing(valid_enumerated_board_times) {
        return Err(VehicleTimesError::DecreasingBoardTime(position_pair));
    }

    let valid_enumerated_debark_times = debark_times
        .clone()
        .zip(flows.clone())
        .enumerate()
        .filter_map(
            |(position, (debark_time, flow_direction))| match flow_direction {
                DebarkOnly | BoardAndDebark => Some((position, debark_time)),
                NoBoardDebark | BoardOnly => None,
            },
        );

    if let Err(position_pair) = is_increasing(valid_enumerated_debark_times) {
        return Err(VehicleTimesError::DecreasingDebarkTime(position_pair));
    }

    let pair_iter = board_times
        .zip(flows.clone())
        .zip(debark_times.zip(flows).skip(1))
        .enumerate();
    for (board_idx, ((board_time, board_flow), (debark_time, debark_flow))) in pair_iter {
        let debark_idx = board_idx + 1;
        let can_board = match board_flow {
            BoardAndDebark | BoardOnly => true,
            NoBoardDebark | DebarkOnly => false,
        };
        let can_debark = match debark_flow {
            BoardAndDebark | DebarkOnly => true,
            NoBoardDebark | BoardOnly => false,
        };
        if can_board && can_debark && board_time > debark_time {
            let position_pair = PositionPair {
                upstream: StopTimeIdx { idx: board_idx },
                downstream: StopTimeIdx { idx: debark_idx },
            };
            return Err(VehicleTimesError::DebarkBeforeUpstreamBoard(position_pair));
        }
    }

    Ok(())
}

#[derive(Clone, Debug)]
pub struct PositionPair {
    pub upstream: StopTimeIdx,
    pub downstream: StopTimeIdx,
}

#[derive(Clone, Debug)]
pub enum VehicleTimesError {
    DebarkBeforeUpstreamBoard(PositionPair), // board_time[upstream] > debark_time[downstream]
    DecreasingBoardTime(PositionPair),       // board_time[upstream] > board_time[downstream]
    DecreasingDebarkTime(PositionPair),      // debark_time[upstream] > debark_time[downstream]
    LessThanTwoStops,
}
