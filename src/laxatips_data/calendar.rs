use std::convert::TryFrom;

use super::time::{DaysSinceDatasetStart, SecondsSinceDatasetUTCStart};

use chrono::{NaiveDate, NaiveDateTime};

pub struct Calendar {
    first_date: NaiveDate,  //first date which may be allowed
    last_date: NaiveDate,   //last date (included) which may be allowed
    nb_of_days: u16,        // == (last_date - first_date).num_of_days()
                            // we allow at most u16::MAX = 65_535 days

    // the first UTC datetime allowed in this calendar
    // it 
    // first_datetime : NaiveDateTime,
    // last_datetime : NaiveDateTime,

}

impl Calendar {
    pub fn new(first_date: NaiveDate, last_date: NaiveDate) -> Self {
        assert!(first_date <= last_date);
        let nb_of_days_i64: i64 = (last_date - first_date).num_days() + 1;

        let nb_of_days: u16 = TryFrom::try_from(nb_of_days_i64)
            .expect("Trying to construct a calendar with more days than u16::MAX.");

        Self {
            first_date,
            last_date,
            nb_of_days,
        }
    }

    pub fn nb_of_days(&self) -> u16 {
        self.nb_of_days
    }

    pub fn days(&self) -> DaysIter {
        DaysIter {
            inner: 0..self.nb_of_days,
        }
    }

    /// The first datetime that can be obtained
    pub fn first_datetime(&self) -> NaiveDateTime {
        self.first_date.and_hms(0,0,0) 
            - chrono::Duration::hours(24) // in the west most timezone, we are at UTC-12, with take some margin (day saving times...) and make it -24h
            - chrono::Duration::seconds(MAX_SECONDS_SINCE_TIMEZONED_DAY_START as i64)
            
    }

    pub fn last_datetime(&self) -> NaiveDateTime {
        self.last_date.and_hms(0, 0, 0)
        + chrono::Duration::hours(24) // in the west most timezone, we are at UTC+14, with take some margin and make it +24h
        + chrono::Duration::seconds(MAX_SECONDS_SINCE_TIMEZONED_DAY_START as i64)
    }

    fn first_date(&self) -> &NaiveDate {
        &self.first_date
    }

    fn last_date(&self) -> &NaiveDate {
        &self.last_date
    }

    pub fn to_naive_date(&self, day : & DaysSinceDatasetStart) -> NaiveDate {
        *self.first_date() + chrono::Duration::days(day.days as i64)
    }

    pub fn to_string(&self, seconds: &SecondsSinceDatasetUTCStart) -> String {
        let datetime = self.to_naive_datetime(seconds);
        datetime.format("%Y%m%dT%H%M%S").to_string()
    }

    pub fn to_pretty_string(&self, seconds: &SecondsSinceDatasetUTCStart) -> String {
        let datetime = self.to_naive_datetime(seconds);
        datetime.format("%H:%M:%S %d-%b-%y").to_string()
    }

    pub fn to_naive_datetime(&self, seconds: &SecondsSinceDatasetUTCStart) -> NaiveDateTime {
        self.first_date.and_hms(0, 0, 0) + seconds.to_chrono_duration()
    }

    pub fn contains(&self, date: &NaiveDate) -> bool {
        self.first_date <= *date && *date <= self.last_date
    }


    pub fn date_to_days_since_start(&self, date: &NaiveDate) -> Option<DaysSinceDatasetStart> {
        self.date_to_offset(date)
            .map(|offset| {
                DaysSinceDatasetStart {
                    days : offset
                }
            })
    }

    pub (super) fn date_to_offset(&self, date: &NaiveDate) -> Option<u16> {
        if *date < self.first_date || *date > self.last_date {
            None
        } else {
            let offset_64: i64 = (*date - self.first_date).num_days();
            // should be safe because :
            //  - we check that offset_64 is positive above when testing if date < self.first_date
            //  - we check that offset_64 is smaller than u8::MAX because at construction of Calendars
            //    we ensure that (last_date - first_date).num_days() < u8::MAX
            //    and we check above that date <= self.last_date
            let offset = u16::try_from(offset_64).unwrap();
            Some(offset)
        }
    }
}

pub struct DaysIter {
    inner: std::ops::Range<u16>,
}

impl Iterator for DaysIter {
    type Item = DaysSinceDatasetStart;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|idx| DaysSinceDatasetStart { days: idx })
    }
}
