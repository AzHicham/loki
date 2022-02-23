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

use chrono::Local;
use std::{collections::HashMap, iter};

use crate::{
    models::VehicleJourneyIdx,
    time::{
        days_map::{DaysMap, InsertError},
        days_patterns::{DaysPattern, DaysPatterns},
        DaysSinceDatasetStart,
    },
    RealTimeLevel,
};

pub type LocalZone = Option<u16>;

pub struct VehicleJourneyToTimetable<Timetable> {
    data: HashMap<VehicleJourneyIdx, HashMap<LocalZone, DayToTimetable<Timetable>>>,
}

struct DayToTimetable<Timetable> {
    base: DaysMap<Timetable>,
    real_time: DaysMap<Timetable>,
}

impl<Timetable> DayToTimetable<Timetable> {
    fn new() -> Self {
        Self {
            base: DaysMap::new(),
            real_time: DaysMap::new(),
        }
    }
}

impl<Timetable> VehicleJourneyToTimetable<Timetable>
where
    Timetable: PartialEq + Eq + Clone,
{
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn insert_base_and_realtime_vehicle(
        &mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        local_zone: &LocalZone,
        days_pattern_to_insert: &DaysPattern,
        timetable_to_insert: &Timetable,
        days_patterns: &mut DaysPatterns,
    ) -> Result<(), InsertionError> {
        let day_to_timetable = self
            .data
            .entry(vehicle_journey_idx.clone())
            .or_insert(HashMap::new())
            .entry(local_zone.clone())
            .or_insert(DayToTimetable::new());

        let base_insert_result = day_to_timetable.base.insert(
            days_pattern_to_insert,
            timetable_to_insert.clone(),
            days_patterns,
        );
        if let Err(InsertError::DaysAlreadySet(days)) = base_insert_result {
            let err = InsertionError::DaysAlreadySet(
                vehicle_journey_idx.clone(),
                RealTimeLevel::Base,
                days,
            );
            return Err(err);
        }
        let real_time_insert_result = day_to_timetable.real_time.insert(
            days_pattern_to_insert,
            timetable_to_insert.clone(),
            days_patterns,
        );
        if let Err(InsertError::DaysAlreadySet(days)) = real_time_insert_result {
            let err = InsertionError::DaysAlreadySet(
                vehicle_journey_idx.clone(),
                RealTimeLevel::RealTime,
                days,
            );
            return Err(err);
        }

        Ok(())
    }

    pub fn insert_real_time_only_vehicle(
        &mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        local_zone: &LocalZone,
        days_pattern_to_insert: &DaysPattern,
        timetable_to_insert: &Timetable,
        days_patterns: &mut DaysPatterns,
    ) -> Result<(), InsertionError> {
        let day_to_timetable = self
            .data
            .entry(vehicle_journey_idx.clone())
            .or_insert(HashMap::new())
            .entry(local_zone.clone())
            .or_insert(DayToTimetable::new());

        let real_time_insert_result = day_to_timetable.real_time.insert(
            days_pattern_to_insert,
            timetable_to_insert.clone(),
            days_patterns,
        );
        if let Err(InsertError::DaysAlreadySet(days)) = real_time_insert_result {
            let err = InsertionError::DaysAlreadySet(
                vehicle_journey_idx.clone(),
                RealTimeLevel::RealTime,
                days,
            );
            return Err(err);
        }

        Ok(())
    }

    fn get_mut_day_to_timetable(
        &mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        local_zone: &LocalZone,
    ) -> Option<&mut DayToTimetable<Timetable>> {
        let has_day_to_timetable = self.data.get_mut(vehicle_journey_idx)?;
        has_day_to_timetable.get_mut(local_zone)
    }

    fn get_day_to_timetable(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        local_zone: &LocalZone,
    ) -> Option<&DayToTimetable<Timetable>> {
        let has_day_to_timetable = self.data.get(vehicle_journey_idx)?;
        has_day_to_timetable.get(local_zone)
    }

    pub fn remove_real_time_vehicle(
        &mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        local_zone: &LocalZone,
        day: &DaysSinceDatasetStart,
        days_patterns: &mut DaysPatterns,
    ) -> Result<Timetable, Unknown> {
        let day_to_timetable = self
            .get_mut_day_to_timetable(vehicle_journey_idx, local_zone)
            .ok_or(Unknown::VehicleJourneyIdx)?;
        day_to_timetable
            .real_time
            .remove(day, days_patterns)
            .map_err(|_| Unknown::DayForVehicleJourney)
    }

    pub fn base_vehicle_exists(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        local_zone: &LocalZone,
    ) -> bool {
        let day_to_timetable = self.get_day_to_timetable(vehicle_journey_idx, local_zone);
        match day_to_timetable {
            Some(day_to_timetable) => !day_to_timetable.base.is_empty(),
            None => false,
        }
    }

    pub fn real_time_vehicle_exists(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        local_zone: &LocalZone,
        day: &DaysSinceDatasetStart,
        days_patterns: &DaysPatterns,
    ) -> bool {
        let day_to_timetable = self.get_day_to_timetable(vehicle_journey_idx, local_zone);
        match day_to_timetable {
            Some(day_to_timetable) => day_to_timetable.real_time.get(day, days_patterns).is_some(),
            None => false,
        }
    }

    pub fn get_vehicle_local_zones(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
    ) -> Vec<LocalZone> {
        match self.data.get(vehicle_journey_idx) {
            Some(map) => map.keys().map(|k| k.clone()).collect(),
            None => Vec::new(),
        }
    }
}
#[derive(Debug)]
pub enum InsertionError {
    DaysAlreadySet(VehicleJourneyIdx, RealTimeLevel, Vec<DaysSinceDatasetStart>),
}

#[derive(Debug)]
pub enum Unknown {
    VehicleJourneyIdx,
    DayForVehicleJourney,
}
