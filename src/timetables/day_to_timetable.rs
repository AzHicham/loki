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

use std::collections::HashMap;

use crate::{
    models::VehicleJourneyIdx,
    time::{
        days_patterns::{DaysPattern, DaysPatterns},
        DaysSinceDatasetStart,
    },
};

use super::{generic_timetables::Timetable, RealTimeValidity};

#[derive(Debug)]
pub struct VehicleJourneyToTimetable {
    base_and_real_time: HashMap<VehicleJourneyIdx, DayToTimetable>,
    base_only: HashMap<VehicleJourneyIdx, DayToTimetable>,
    real_time_only: HashMap<VehicleJourneyIdx, DayToTimetable>,
}

impl VehicleJourneyToTimetable {
    pub fn new() -> Self {
        Self {
            base_and_real_time: HashMap::new(),
            base_only: HashMap::new(),
            real_time_only: HashMap::new(),
        }
    }

    pub fn get_timetable(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        day: &DaysSinceDatasetStart,
        real_time_validity: &RealTimeValidity,
        days_patterns: &DaysPatterns,
    ) -> Result<Timetable, Unknown> {
        let data = match *real_time_validity {
            RealTimeValidity::BaseAndRealTime => &self.base_and_real_time,
            RealTimeValidity::BaseOnly => &self.base_only,
            RealTimeValidity::RealTimeOnly => &self.real_time_only,
        };
        data.get(vehicle_journey_idx)
            .ok_or(Unknown::VehicleJourneyIdx)?
            .has_timetable_on_day(day, days_patterns)
            .ok_or(Unknown::DayForVehicleJourney)
    }

    pub fn insert(
        &mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        real_time_validity: &RealTimeValidity,
        days_pattern_to_insert: &DaysPattern,
        timetable_to_insert: &Timetable,
        days_patterns: &mut DaysPatterns,
    ) -> Result<(), InsertError> {
        let data = match *real_time_validity {
            RealTimeValidity::BaseAndRealTime => &mut self.base_and_real_time,
            RealTimeValidity::BaseOnly => &mut self.base_only,
            RealTimeValidity::RealTimeOnly => &mut self.real_time_only,
        };
        data.entry(vehicle_journey_idx.clone())
            .or_insert_with(DayToTimetable::new)
            .insert_days_pattern(days_pattern_to_insert, timetable_to_insert, days_patterns)
    }

    pub fn remove(
        &mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        day: &DaysSinceDatasetStart,
        real_time_validity: &RealTimeValidity,
        days_patterns: &mut DaysPatterns,
    ) -> Result<Timetable, Unknown> {
        let data = match *real_time_validity {
            RealTimeValidity::BaseAndRealTime => &mut self.base_and_real_time,
            RealTimeValidity::BaseOnly => &mut self.base_only,
            RealTimeValidity::RealTimeOnly => &mut self.real_time_only,
        };
        data.get_mut(vehicle_journey_idx)
            .ok_or(Unknown::VehicleJourneyIdx)?
            .remove(day, days_patterns)
            .map_err(|_| Unknown::DayForVehicleJourney)
    }
}

#[derive(Debug)]
pub enum Unknown {
    VehicleJourneyIdx,
    DayForVehicleJourney,
}

#[derive(Debug)]
pub struct DayToTimetable {
    // invariants :
    //  1. a day is set in at most one DaysPattern of the Vec
    //  2. a timetable appears at most once in the vec
    pattern_timetables: Vec<(DaysPattern, Timetable)>,
}

impl DayToTimetable {
    pub fn new() -> Self {
        Self {
            pattern_timetables: Vec::new(),
        }
    }

    pub fn has_timetable_on_day(
        &self,
        day: &DaysSinceDatasetStart,
        days_patterns: &DaysPatterns,
    ) -> Option<Timetable> {
        self.pattern_timetables
            .iter()
            .find_map(|(days_pattern, timetable)| {
                if days_patterns.is_allowed(days_pattern, day) {
                    Some(timetable.clone())
                } else {
                    None
                }
            })
    }

    pub fn insert_days_pattern(
        &mut self,
        days_pattern_to_insert: &DaysPattern,
        timetable_to_insert: &Timetable,
        days_patterns: &mut DaysPatterns,
    ) -> Result<(), InsertError> {
        // is there a day in days_pattern_to_insert that is already set somewhere ?
        for (days_pattern, _) in self.pattern_timetables.iter() {
            if let Some(_day) = days_patterns.have_common_day(days_pattern, days_pattern_to_insert)
            {
                return Err(InsertError::DayAlreadySet);
            }
        }

        // We try to find the first element whose timetable contains timetable_to_insert .
        // Because of our invariant 2., if such an element is found we know that
        // timetable_to_insert does not appears in any other element of the vec.
        let has_days_pattern = self
            .pattern_timetables
            .iter_mut()
            .find(|(_days_pattern, timetable)| timetable == timetable_to_insert)
            .map(|(days_pattern, _)| days_pattern); // we are just interested in the pattern

        if let Some(old_days_pattern) = has_days_pattern {
            // so now timetable_to_insert is valid on old_days_pattern and days_pattern_to_insert
            // let's create a new days_pattern for that
            let new_days_pattern =
                days_patterns.get_union(*old_days_pattern, *days_pattern_to_insert);

            *old_days_pattern = new_days_pattern;
        } else {
            // if timetable_to_insert does not appears in the Vec,
            // let's push a new element to the Vec with it
            self.pattern_timetables
                .push((*days_pattern_to_insert, timetable_to_insert.clone()));
        }

        Ok(())
    }

    pub fn remove(
        &mut self,
        day_to_remove: &DaysSinceDatasetStart,
        days_patterns: &mut DaysPatterns,
    ) -> Result<Timetable, RemoveError> {
        // let's try to find the first element where day_to_remove is set.
        // Because of invariant 1., if such an element is found, we know that
        // day_to_remove is not set in any other element
        let has_days_pattern = self.pattern_timetables.iter_mut().enumerate().find(
            |(_idx, (days_pattern, _timetable))| {
                days_patterns.is_allowed(days_pattern, day_to_remove)
            },
        );

        let (removed_timetable, has_idx_to_remove) = match has_days_pattern {
            None => {
                return Err(RemoveError::DayNotSet);
            }
            Some((idx, (old_days_pattern, timetable))) => {
                let new_days_pattern = days_patterns
                    .get_pattern_without_day(*old_days_pattern, day_to_remove)
                    .ok_or(RemoveError::DayNotSet)?;

                if days_patterns.is_empty_pattern(&new_days_pattern) {
                    (timetable.clone(), Some(idx))
                } else {
                    *old_days_pattern = new_days_pattern;
                    (timetable.clone(), None)
                }
            }
        };

        if let Some(idx) = has_idx_to_remove {
            self.pattern_timetables.swap_remove(idx);
        }

        Ok(removed_timetable)
    }
}

#[derive(Debug)]
pub enum InsertError {
    DayAlreadySet,
}
#[derive(Debug)]
pub enum RemoveError {
    DayNotSet,
}
