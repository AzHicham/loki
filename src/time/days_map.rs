// Copyright  (C) 2021, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Hove (www.kisio.com).
// Help us simplify mobility and open public transport:
// a non ending quest to the responsive locomotion way of traveling!
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

use crate::time::DaysSinceDatasetStart;

use super::days_patterns::{DaysPattern, DaysPatterns};

#[derive(Debug)]
pub struct DaysMap<T> {
    // invariants :
    //  1. a day is set in at most one DaysPattern of the Vec
    //  2. a T appears at most once in the vec
    data: Vec<(DaysPattern, T)>,
}

impl<T> DaysMap<T> {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn get(&self, day: DaysSinceDatasetStart, days_patterns: &DaysPatterns) -> Option<&T> {
        self.data.iter().find_map(|(days_pattern, value)| {
            if days_patterns.is_allowed(days_pattern, day) {
                Some(value)
            } else {
                None
            }
        })
    }

    pub fn insert(
        &mut self,
        days_pattern_to_insert: &DaysPattern,
        value_to_insert: T,
        days_patterns: &mut DaysPatterns,
    ) -> Result<(), InsertError>
    where
        T: Eq,
    {
        // is there a day in days_pattern_to_insert that is already set somewhere ?
        for (days_pattern, _) in self.data.iter() {
            let common_days = days_patterns.common_days(days_pattern, days_pattern_to_insert);
            if !common_days.is_empty() {
                return Err(InsertError::DaysAlreadySet(common_days));
            }
        }

        // We try to find the first element whose value is value_to_insert .
        // Because of our invariant 2., if such an element is found we know that
        // value_to_insert does not appears in any other element of the vec.
        let has_days_pattern = self
            .data
            .iter_mut()
            .find(|(_days_pattern, value)| *value == value_to_insert)
            .map(|(days_pattern, _)| days_pattern); // we are just interested in the pattern

        if let Some(old_days_pattern) = has_days_pattern {
            // so now timetable_to_insert is valid on old_days_pattern and days_pattern_to_insert
            // let's create a new days_pattern for that
            let new_days_pattern =
                days_patterns.get_union(*old_days_pattern, *days_pattern_to_insert);

            *old_days_pattern = new_days_pattern;
        } else {
            // if value_to_insert does not appears in the Vec,
            // let's push a new element to the Vec with it
            self.data.push((*days_pattern_to_insert, value_to_insert));
        }

        Ok(())
    }

    pub fn remove(
        &mut self,
        day_to_remove: DaysSinceDatasetStart,
        days_patterns: &mut DaysPatterns,
    ) -> Result<T, RemoveError>
    where
        T: Clone,
    {
        // let's try to find the first element where day_to_remove is set.
        // Because of invariant 1., if such an element is found, we know that
        // day_to_remove is not set in any other element
        let has_days_pattern =
            self.data
                .iter_mut()
                .enumerate()
                .find(|(_idx, (days_pattern, _timetable))| {
                    days_patterns.is_allowed(days_pattern, day_to_remove)
                });

        let (removed_timetable, has_idx_to_remove) = match has_days_pattern {
            None => {
                return Err(RemoveError::DayNotSet);
            }
            Some((idx, (old_days_pattern, value))) => {
                let new_days_pattern = days_patterns
                    .get_pattern_without_day(*old_days_pattern, day_to_remove)
                    .ok_or(RemoveError::DayNotSet)?;

                if days_patterns.is_empty_pattern(&new_days_pattern) {
                    (value.clone(), Some(idx))
                } else {
                    *old_days_pattern = new_days_pattern;
                    (value.clone(), None)
                }
            }
        };

        if let Some(idx) = has_idx_to_remove {
            self.data.swap_remove(idx);
        }

        Ok(removed_timetable)
    }
}

#[derive(Debug)]
pub enum InsertError {
    DaysAlreadySet(Vec<DaysSinceDatasetStart>),
}
#[derive(Debug)]
pub enum RemoveError {
    DayNotSet,
}
