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

use super::generic_timetables::{GenericTimetables, Position, Timetable, TimetableData, Vehicle};
use std::{fmt::Debug, iter::Map, ops::Range};

pub type TimetableIter = Map<Range<usize>, fn(usize) -> Timetable>;

impl<Time, Load, TripData> GenericTimetables<Time, Load, TripData>
where
    Time: Ord + Clone + Debug,
    Load: Ord + Clone + Debug,
{
    pub fn timetables(&self) -> TimetableIter {
        (0..self.nb_of_timetables()).map(|idx| Timetable { idx })
    }

    pub fn vehicles(&self, timetable: &Timetable) -> VehicleIter {
        let timetable_data = self.timetable_data(timetable);
        let nb_of_vehicles = timetable_data.nb_of_vehicle();
        VehicleIter::new(timetable.clone(), 0..nb_of_vehicles)
    }

    pub fn positions(&self, timetable: &Timetable) -> PositionsIter {
        let nb_of_position = self.timetable_data(timetable).nb_of_positions();
        PositionsIter::new(timetable.clone(), 0..nb_of_position)
    }
}

impl<Time, Load, VehicleData> TimetableData<Time, Load, VehicleData>
where
    Time: Ord + Clone + Debug,
    Load: Ord + Debug,
{
    pub(super) fn vehicle_debark_times(&self, vehicle_idx: usize) -> VehicleTimes<Time> {
        debug_assert!(vehicle_idx < self.vehicle_datas.len());
        VehicleTimes {
            times_by_position: &self.debark_times_by_position,
            position_idx: 0,
            vehicle_idx,
        }
    }

    pub(super) fn vehicle_board_times(&self, vehicle_idx: usize) -> VehicleTimes<Time> {
        debug_assert!(vehicle_idx < self.vehicle_datas.len());
        VehicleTimes {
            times_by_position: &self.board_times_by_position,
            position_idx: 0,
            vehicle_idx,
        }
    }

    pub(super) fn vehicle_loads(&self, vehicle_idx: usize) -> std::slice::Iter<'_, Load> {
        debug_assert!(vehicle_idx < self.vehicle_datas.len());
        self.vehicle_loads[vehicle_idx].iter()
    }
}

pub struct PositionsIter {
    timetable: Timetable,
    position_idxs: Range<usize>,
}

impl PositionsIter {
    fn new(timetable: Timetable, position_idxs: Range<usize>) -> Self {
        Self {
            timetable,
            position_idxs,
        }
    }
}

impl Iterator for PositionsIter {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        self.position_idxs.next().map(|idx| Position {
            timetable: self.timetable.clone(),
            idx,
        })
    }
}

pub struct VehicleIter {
    timetable: Timetable,
    vehicle_idxs: Range<usize>,
}

impl VehicleIter {
    fn new(timetable: Timetable, vehicle_idxs: Range<usize>) -> Self {
        Self {
            timetable,
            vehicle_idxs,
        }
    }
}

impl Iterator for VehicleIter {
    type Item = Vehicle;

    fn next(&mut self) -> Option<Self::Item> {
        self.vehicle_idxs.next().map(|idx| Vehicle {
            timetable: self.timetable.clone(),
            idx,
        })
    }
}

pub(super) struct VehicleTimes<'a, Time> {
    times_by_position: &'a [Vec<Time>],
    position_idx: usize,
    vehicle_idx: usize,
}

impl<'a, Time> Clone for VehicleTimes<'a, Time> {
    fn clone(&self) -> Self {
        Self {
            times_by_position: self.times_by_position,
            position_idx: self.position_idx,
            vehicle_idx: self.vehicle_idx,
        }
    }
}

impl<'a, Time> Iterator for VehicleTimes<'a, Time> {
    type Item = &'a Time;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self
            .times_by_position
            .get(self.position_idx)
            .map(|time_by_vehicles| &time_by_vehicles[self.vehicle_idx]);
        if result.is_some() {
            self.position_idx += 1;
        }
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.times_by_position.len() - self.position_idx;
        (remaining, Some(remaining))
    }
}

impl<'a, Time> ExactSizeIterator for VehicleTimes<'a, Time> where Time: Clone {}
