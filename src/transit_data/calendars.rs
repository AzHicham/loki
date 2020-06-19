
use std::convert::{TryFrom};

use super::time::{
    DaysSinceDatasetStart,
    SecondsSinceDatasetStart,
};

use chrono::{
    NaiveDateTime,
    NaiveDate,
};

pub struct Calendars{
    first_date : NaiveDate, //first date which may be allowed
    last_date : NaiveDate,  //last date (included) which may be allowed
    nb_of_days : u16,  // == (last_date - first_date).num_of_days()
                       // we allow at most u16::MAX = 65_535 days
    calendars : Vec<Calendar>,

    buffer : Vec<bool>
}

struct Calendar {
    allowed_dates : Vec<bool>
}

#[derive(Debug, Copy, Clone)]
pub struct CalendarIdx {
    idx : usize
}



impl Calendars {

    pub fn new(first_date : NaiveDate, last_date : NaiveDate) -> Self {
        assert!(first_date <= last_date);
        let nb_of_days_i64 : i64 = (last_date - first_date).num_days() + 1;

        let nb_of_days : u16 = TryFrom::try_from(nb_of_days_i64)
                .expect("Trying to construct a calendar with more days than u16::MAX.");

        Self {
            first_date,
            last_date,
            nb_of_days,
            calendars : Vec::new(),

            buffer : vec![false; nb_of_days.into()]
        }
    }

    pub fn days(&self) -> DaysIter {
        DaysIter {
            inner : 0..self.nb_of_days
        }
    }

    // try to convert a unix timestamp (nb of seconds since midnight UTC on January 1, 1970)
    // to the number of seconds since the beginning of this calendar
    // returns None if the timestamp is out of bounds of this calendar
    // see https://docs.rs/chrono/0.4.11/chrono/naive/struct.NaiveDateTime.html#method.from_timestamp_opt
    pub fn timestamp_to_seconds_since_start(&self, timestamp : i64) -> Option<SecondsSinceDatasetStart> {
        let has_datetime = NaiveDateTime::from_timestamp_opt(timestamp, 0);
        if let Some(datetime) = has_datetime {
            self.to_seconds_since_start(&datetime)
        }
        else {
            None
        }
    }

    // try to parse a string datetime (formatted like '20200316T123560')
    // and convert it to the number of seconds since the beginning of this calendar
    // returns None if the parsing failed, or if the date is out of bounds of this calendar
    pub fn string_datetime_to_seconds_since_start(&self, string_datetime : &str) -> Option<SecondsSinceDatasetStart> {
 
        let try_datetime = NaiveDateTime::parse_from_str(string_datetime, "%Y%m%dT%H%M%S");
        match try_datetime {
            Ok(datetime) => {
                self.to_seconds_since_start(&datetime)
            },
            Err(_) => {
                None
            }
        }
    }

    pub fn to_string(&self, seconds : & SecondsSinceDatasetStart) -> String {
        let datetime = self.first_date.and_hms(0, 0, 0) + chrono::Duration::seconds(seconds.seconds as i64);
        format!("{}", datetime)
    }


    pub fn to_seconds_since_start(&self, datetime : & NaiveDateTime) -> Option<SecondsSinceDatasetStart> {
        let date = datetime.date();
        if ! self.contains(&date) {
            return None;
        }
        let seconds_i64 = (*datetime - self.first_date.and_hms(0, 0, 0)).num_seconds();
        debug_assert!(seconds_i64 >= 0);
        debug_assert!(seconds_i64 <= u32::MAX as i64);
        let try_seconds_u32 = u32::try_from(seconds_i64);
        try_seconds_u32.map_or(None, |seconds_u32| {
            let result = SecondsSinceDatasetStart {
                seconds : seconds_u32
            };
            Some(result)
        })
    }

    pub fn is_allowed(&self, calendar_idx : & CalendarIdx, day : & DaysSinceDatasetStart) -> bool {
        debug_assert!(day.days < self.nb_of_days);
        debug_assert!(calendar_idx.idx < self.calendars.len());
        let day_idx : usize = day.days.into();
        self.calendars[calendar_idx.idx].allowed_dates[day_idx]
    }

    pub fn get_or_insert<'a, Dates>(&mut self, dates : Dates) -> CalendarIdx 
    where Dates : Iterator<Item = & 'a NaiveDate>
    {
        // set all elements of the buffer to false
        for  val in self.buffer.iter_mut() {
            *val = false
        }

        for date in dates {
            let has_offset = self.date_to_offset(date);
            if let Some(offset) = has_offset {
                self.buffer[offset] = true;
            }
        }

        let has_calendar_idx = self.calendars.iter()
                                .enumerate()
                                .find(|(_, calendar) | {
                                    calendar.allowed_dates == self.buffer
                                })
                                .map( |(idx, _)| idx );

        let idx = if let Some(idx) = has_calendar_idx {
                idx
        }
        else {
            let idx = self.calendars.len();
            let calendar = Calendar{
                allowed_dates : self.buffer.clone()
            };
            self.calendars.push(calendar);
            idx
        };

        CalendarIdx{
            idx
        }
    }



    fn contains(&self, date : & NaiveDate) -> bool {
        self.first_date <= *date && *date <= self.last_date
    }

    fn date_to_offset(&self, date : & NaiveDate) ->  Option<usize> 
    {
        if *date < self.first_date || *date > self.last_date {
            None
        }
        else {
            let offset_64 : i64 = (*date - self.first_date).num_days();
            // should be safe because :
            //  - we check that offset_64 is positive above when testing if date < self.first_date
            //  - we check that offset_64 is smaller than usize::MAX because at construction of Calendars
            //    we ensure that (last_date - first_date).num_days() < usize::MAX
            //    and we check above that date <= self.last_date
            let offset : usize = TryFrom::try_from(offset_64).unwrap();
            Some(offset)
        }
    }

}

pub struct DaysIter {
    inner : std::ops::Range<u16>
}

impl Iterator for DaysIter {
    type Item = DaysSinceDatasetStart;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|idx| {
            DaysSinceDatasetStart {
                days : idx
            }
        })
    }
}
