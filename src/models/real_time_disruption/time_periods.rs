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

use crate::time::MAX_SECONDS_IN_UTC_DAY;
use chrono::{Duration, NaiveDate, NaiveDateTime};
use std::{
    cmp::{max, min},
    fmt::{Debug, Display},
};

/// An half open interval of time.
/// A instant `t` is contained in it
/// if and only if
///  `start <= t < end`
///
#[derive(Debug, Clone)]
pub struct TimePeriod {
    start: NaiveDateTime,
    end: NaiveDateTime,
}

impl TimePeriod {
    pub fn new(start: NaiveDateTime, end: NaiveDateTime) -> Result<TimePeriod, TimePeriodError> {
        if start < end {
            Ok(TimePeriod { start, end })
        } else {
            Err(TimePeriodError::StartAfterEnd(start, end))
        }
    }

    pub fn start(&self) -> NaiveDateTime {
        self.start
    }

    pub fn end(&self) -> NaiveDateTime {
        self.end
    }

    pub fn contains(&self, t: &NaiveDateTime) -> bool {
        self.start <= *t && *t < self.end
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.contains(&other.start) || other.contains(&self.start)
    }

    // Returns an iterator that contains all dates D such that
    //  a vehicle_journey on D is "concerned" by this time_period,
    //  where "concerned" means that a stop_time of the vehicle_journey
    //   circulating on date D is contained in this time_period
    //
    // Note that the iterator may contains dates for which a vehicle
    // journey is *NOT* concerned. The caller should check by himself.
    pub fn dates_possibly_concerned(&self) -> DateIter {
        // since the vehicle journey stop_times are given in local time
        // and we accept values up to 48h, we use a 3 days offset
        // that account for both
        let offset = Duration::seconds(i64::from(MAX_SECONDS_IN_UTC_DAY));
        let first_date = (self.start - offset).date();
        let last_date = (self.end + offset).date();
        DateIter::new(first_date, last_date)
    }
}

pub struct TimePeriods<'a> {
    periods: &'a [TimePeriod],
}

impl<'a> TimePeriods<'a> {
    pub fn new(periods: &'a [TimePeriod]) -> Option<Self> {
        if periods.is_empty() {
            None
        } else {
            Some(Self { periods })
        }
    }

    pub fn contains(&self, t: &NaiveDateTime) -> bool {
        for period in self.periods {
            if period.contains(t) {
                return true;
            }
        }
        false
    }

    pub fn intersects(&self, other: &TimePeriod) -> bool {
        for period in self.periods {
            if period.intersects(other) {
                return true;
            }
        }
        false
    }

    // Returns an iterator that contains all dates D such that
    //  a vehicle_journey on D is "concerned" by this time_periods,
    //  where "concerned" means that a stop_time of the vehicle_journey
    //   circulating on date D is contained in this time_periods
    //
    // Note that the iterator may contains dates for which a vehicle
    // journey is *NOT* concerned. The caller should check by himself.
    pub fn dates_possibly_concerned(&self) -> DateIter {
        let earliest_datetime = self
            .periods
            .iter()
            .map(|period| period.start)
            .min()
            .unwrap(); // unwrap safe here because we check in new() that ! periods.is_empty()

        let latest_datetime = self.periods.iter().map(|period| period.end).max().unwrap(); // unwrap safe here because we check in new() that ! periods.is_empty()

        // since the vehicle journey stop_times are given in local time
        // and we accept values up to 48h, we use a 3 days offset
        // that account for both
        let offset = Duration::seconds(i64::from(MAX_SECONDS_IN_UTC_DAY));
        let first_date = (earliest_datetime - offset).date();
        let last_date = (latest_datetime + offset).date();
        DateIter::new(first_date, last_date)
    }
}

pub fn intersection(lhs: &TimePeriod, rhs: &TimePeriod) -> Option<TimePeriod> {
    TimePeriod::new(max(lhs.start, rhs.start), min(lhs.end, rhs.end)).ok()
}

pub enum TimePeriodError {
    StartAfterEnd(NaiveDateTime, NaiveDateTime),
}

impl std::error::Error for TimePeriodError {}

impl Display for TimePeriodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}

impl Debug for TimePeriodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimePeriodError::StartAfterEnd(start, end) => {
                write!(
                    f,
                    "Bad TimePeriod, start {} must be strictly lower than end {}",
                    start, end
                )
            }
        }
    }
}

// Yields all dates between current_date (included)
// and last_date (also included)
pub struct DateIter {
    has_current_date: Option<NaiveDate>,
    last_date: NaiveDate,
}

impl DateIter {
    pub fn new(first_date: NaiveDate, last_date: NaiveDate) -> Self {
        if first_date <= last_date {
            Self {
                has_current_date: Some(first_date),
                last_date,
            }
        } else {
            Self {
                has_current_date: None,
                last_date,
            }
        }
    }
}

impl Iterator for DateIter {
    type Item = NaiveDate;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(current_date) = self.has_current_date else {
            return None;
        };
        if current_date <= self.last_date {
            self.has_current_date = current_date.succ_opt();
            Some(current_date)
        } else {
            self.has_current_date = None;
            None
        }
    }
}
