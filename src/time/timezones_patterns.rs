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

use super::days_patterns::{DaysPattern, DaysPatterns};
use crate::time::Calendar;

use chrono::FixedOffset;
use chrono::NaiveDate;
use chrono::Offset;
use chrono::TimeZone as TimeZoneTrait;
use chrono_tz::Tz as TimeZone;

#[derive(Debug)]
pub struct TimezonesPatterns {
    timezones_patterns: HashMap<TimeZone, Vec<(FixedOffset, DaysPattern)>>,
    buffer: HashMap<FixedOffset, Vec<NaiveDate>>,
}

impl TimezonesPatterns {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fetch_or_insert(
        &mut self,
        timezone: &TimeZone,
        days_patterns: &mut DaysPatterns,
        calendar: &Calendar,
    ) -> &[(FixedOffset, DaysPattern)] {
        use std::collections::hash_map::Entry;
        if let Entry::Vacant(vacant_entry) = self.timezones_patterns.entry(*timezone) {
            self.buffer.clear();

            for day in calendar.days() {
                let naive_date: NaiveDate = calendar.to_naive_date(&day);
                // From : https://developers.google.com/transit/gtfs/reference#field_types
                // The local times of a vehicle journey are interpreted as a duration
                // since "noon minus 12h" on each day.
                // Hence the offset between local time and UTC should be computed
                // at noon on each day.
                let datetime_timezoned = timezone.from_utc_date(&naive_date).and_hms(12, 0, 0);
                let offset = datetime_timezoned.offset().fix();
                let dates_for_offset = self.buffer.entry(offset).or_insert_with(Vec::new);
                dates_for_offset.push(naive_date);
            }

            let mut patterns = Vec::with_capacity(self.buffer.len());
            for (offset, dates) in self.buffer.iter() {
                let days_pattern = days_patterns.get_or_insert(dates.iter(), calendar);
                patterns.push((*offset, days_pattern));
            }
            vacant_entry.insert(patterns);
        }
        // unwrap is safe since we just added a value for this key above in case of a vacant entry
        self.timezones_patterns.get(timezone).unwrap().as_slice()
    }
}

impl Default for TimezonesPatterns {
    fn default() -> Self {
        Self {
            timezones_patterns: HashMap::new(),
            buffer: HashMap::new(),
        }
    }
}
