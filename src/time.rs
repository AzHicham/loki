use std::fmt::{Display, Formatter};
use chrono_tz::Tz as Timezone;
use std::convert::TryFrom;
use chrono::{NaiveDate, NaiveDateTime};

/// Duration since "noon minus 12 hours" on a day in a specific timezone
/// This corresponds to the "Time" notion found in gtfs/ntfs stop_times.txt
/// It should be built from a TransitModelTime.
/// This types accept only times are comprised between -48:00:00 and 48:00:00 (maximum plus/minus 2 days)
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct SecondsSinceTimezonedDayStart {
    seconds: i32,
}


const MAX_SECONDS_SINCE_TIMEZONED_DAY_START : i32 = 48 * 60 * 60; // 48h

const MAX_TIMEZONE_OFFSET : i32 = 24 * 60 * 60; // 24h in seconds

/// Duration since 00:00:00 UTC in the first allowed day of the data
/// This is used in the engine to store a point in time in an unambiguous way
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct SecondsSinceDatasetUTCStart {
    seconds: u32,
}

/// Number of days since the first allowed day of the data
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct DaysSinceDatasetStart {
    pub (super) days: u16,
}

// we allow 36_600 days which is more than 100 years, and less than u16::MAX = 65_535 days
const MAX_DAYS_IN_CALENDAR : u16 = 100*366; 


pub struct Calendar {
    first_date: NaiveDate,  //first date which may be allowed
    last_date: NaiveDate,   //last date (included) which may be allowed
    nb_of_days: u16,        // == (last_date - first_date).num_of_days() + 1
                            // we allow at most MAX_DAYS_IN_CALENDAR days
}

impl Calendar {
    pub fn new(first_date: NaiveDate, last_date: NaiveDate) -> Self {
        assert!(first_date <= last_date);
        let nb_of_days_i64: i64 = (last_date - first_date).num_days() + 1;
        assert!(nb_of_days_i64 < MAX_DAYS_IN_CALENDAR as i64, 
            "Trying to construct a calendar with {:#} days \
            which is more than the maximum allowed of {:#} days",
            nb_of_days_i64,
            MAX_DAYS_IN_CALENDAR
        );

        // unwrap here is safe because :
        // - nb_of_days_i64 >=0 since we asserted above that first_date <= last_date
        // - nb_of_days_i64 < MAX_DAYS_IN_CALENDAR < u16::MAX
        let nb_of_days: u16 = TryFrom::try_from(nb_of_days_i64).unwrap();

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
            - chrono::Duration::seconds(i64::from(MAX_TIMEZONE_OFFSET)) // in the west-most timezone, we are at UTC-12, with take some margin (day saving times...) and make it -24h
            - chrono::Duration::seconds(i64::from(MAX_SECONDS_SINCE_TIMEZONED_DAY_START))
            
    }

    pub fn last_datetime(&self) -> NaiveDateTime {
        self.last_date.and_hms(0, 0, 0)
        + chrono::Duration::seconds(i64::from(MAX_TIMEZONE_OFFSET)) // in the east-most timezone, we are at UTC+14, with take some margin and make it +24h
        + chrono::Duration::seconds(i64::from(MAX_SECONDS_SINCE_TIMEZONED_DAY_START))
    }

    pub fn contains_datetime(&self, datetime: & NaiveDateTime) -> bool {
        *datetime >= self.first_datetime() && *datetime <= self.last_datetime()
    }

    pub fn first_date(&self) -> &NaiveDate {
        &self.first_date
    }

    pub fn last_date(&self) -> &NaiveDate {
        &self.last_date
    }

    pub fn contains_date(&self, date: &NaiveDate) -> bool {
        self.first_date <= *date && *date <= self.last_date
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
        self.first_date.and_hms(0, 0, 0) + chrono::Duration::seconds(i64::from(seconds.seconds))
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
            //  - we check that offset_64 is smaller than u16::MAX because at construction of Calendars
            //    we ensure that (last_date - first_date).num_days() < u16::MAX
            //    and we check above that date <= self.last_date
            let offset = offset_64 as u16;
            Some(offset)
        }
    }

    // returns an iterator that provides all decompositions of `second_in_dataset_start`
    // of the form (day, time_in_timezoned_day) such that :
    //  - `day` belongs to the calendar
    //  - `time_in_timezoned_day` belongs to the interval `[min_seconds_since_timezoned_day_start, max_seconds_since_timezoned_day_start]`
    pub fn decompositions<'a>(&'a self,
        seconds_since_dataset_start : & SecondsSinceDatasetUTCStart, 
        timezone : &Timezone,
        max_seconds_since_timezoned_day_start :  SecondsSinceTimezonedDayStart,
        min_seconds_since_timezoned_day_start :  SecondsSinceTimezonedDayStart,
    ) -> impl Iterator<Item = (DaysSinceDatasetStart, SecondsSinceTimezonedDayStart) > + 'a {
        // will advance the date until `time_in_timezoned_day` becomes greater than `min_seconds_since_timezoned_day_start`
        let forward_iter = ForwardDecompose::new(
            seconds_since_dataset_start, 
            min_seconds_since_timezoned_day_start, 
            timezone, 
            &self
        );
        // will decrease the date until `time_in_timezoned_day` becomes greater than `min_seconds_since_timezoned_day_start`
        let mut backward_iter = BackwardDecompose::new(
            seconds_since_dataset_start, 
            max_seconds_since_timezoned_day_start, 
            timezone, 
            &self,
        );
        // we want to skip the first date as it is already provided by `forward_iter`
        backward_iter.next();

        forward_iter.chain(backward_iter)
    }
 
    pub fn compose(&self, day: &DaysSinceDatasetStart, seconds_in_day: &SecondsSinceTimezonedDayStart,  timezone : &Timezone) -> SecondsSinceDatasetUTCStart {
        debug_assert!(day.days < self.nb_of_days);
        let date = *self.first_date() + chrono::Duration::days(day.days as i64);
        // Since DaySinceDatasetStart can only be constructed from the calendar, the date should be allowed by the calendar
        debug_assert!(self.contains_date(&date));
        use chrono::offset::TimeZone;
        let datetime_timezoned = 
            timezone.from_utc_date(&date).and_hms(12, 0, 0) 
            - chrono::Duration::hours(12)
            + chrono::Duration::seconds(seconds_in_day.seconds as i64);
        use chrono_tz::UTC;
        let datetime_utc = datetime_timezoned.with_timezone(&UTC).naive_utc();

        debug_assert!(self.contains_datetime(&datetime_utc));
        let seconds_i64 = (datetime_utc - self.first_datetime()).num_seconds();
        // seconds_i64 should be >=0 since 
        // by construction above, we have 
        // datetime_utc >=
        //        first_date 
        //        - MAX_SECONDS_SINCE_TIMEZONED_DAY_START 
        //        - MAX_TIMEZONE_OFFSET
        // which is exactly how first_datetime() is constructed
        debug_assert!(seconds_i64 >= 0);
        // seconds_i64 <= u32::MAX since : 
        // - day < MAX_DAYS_IN_CALENDAR
        // - seconds_in_day < MAX_SECONDS_SINCE_TIMEZONED_DAY_START
        // thus seconds_i64 <=  MAX_DAYS_IN_CALENDAR * 24 * 60 * 60 
        //                     + MAX_SECONDS_SINCE_TIMEZONED_DAY_START 
        //                     + MAX_TIMEZONE_OFFSET
        // and the latter number is smaller than u32::MAX
        static_assertions::const_assert!( 
            (MAX_DAYS_IN_CALENDAR as i64) * 24 * 60 * 60  
            + (MAX_SECONDS_SINCE_TIMEZONED_DAY_START as i64)
            + (MAX_TIMEZONE_OFFSET as i64)
            <=  u32::MAX as i64
        );
        debug_assert!(seconds_i64 <= u32::MAX as i64);
        let seconds_u32 = seconds_i64 as u32;
        let result = SecondsSinceDatasetUTCStart {
            seconds : seconds_u32
        };
        result

    }
 
    /// Returns Some(_) if datetime is allowed by calendar,
    /// and None otherwise
    pub fn from_naive_datetime(
        &self,
        datetime: & chrono::NaiveDateTime,
    ) -> Option<SecondsSinceDatasetUTCStart> {
        if !self.contains_datetime(&datetime) {
            return None;
        }
        let seconds_i64 = (*datetime - self.first_datetime()).num_seconds();
        // seconds_i64 should be >=0 since calendar.contains_datetime(&datetime)
        debug_assert!(seconds_i64 >= 0);
        // seconds_i64 <= u32::MAX
        // since calendar.contains_datetime(&datetime) and 
        // (last_datetime - first_datetime).num_seconds() <=
        //   MAX_DAYS_IN_CALENDAR * 24 * 60 * 60
        //    + MAX_SECONDS_SINCE_TIMEZONED_DAY_START 
        //    + MAX_TIMEZONE_OFFSET
        // and the latter number is smaller than u32::MAX
        static_assertions::const_assert!( 
            (MAX_DAYS_IN_CALENDAR as i64) * 24 * 60 * 60  
            + (MAX_SECONDS_SINCE_TIMEZONED_DAY_START as i64)
            + (MAX_TIMEZONE_OFFSET as i64)
            <=  u32::MAX as i64
        );
        debug_assert!(seconds_i64 <= u32::MAX as i64);

        let seconds_u32 = seconds_i64 as u32;
        let result = SecondsSinceDatasetUTCStart {
            seconds : seconds_u32
        };
        Some(result)
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


#[derive(Debug, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
pub struct PositiveDuration {
    pub (super) seconds: u32,
}

impl PositiveDuration {
    pub fn zero() -> Self {
        Self { seconds: 0 }
    }

    pub const fn from_hms(hours: u32, minutes: u32, seconds: u32) -> PositiveDuration {
        let total_seconds = seconds + 60 * minutes + 60 * 60 * hours;
        PositiveDuration {
            seconds: total_seconds,
        }
    }

    pub fn total_seconds(&self) -> u64 {
        self.seconds as u64
    }
}

impl Display for PositiveDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let hours = self.seconds / (60 * 60);
        let minutes_in_secs = self.seconds % (60 * 60);
        let minutes = minutes_in_secs / 60;
        let seconds = minutes_in_secs % 60;
        if hours != 0 {
            write!(f, "{}h{:02}m{:02}s", hours, minutes, seconds)
        } else if minutes != 0 {
            write!(f, "{}m{:02}s", minutes, seconds)
        } else {
            write!(f, "{}s", seconds)
        }
    }
}

impl SecondsSinceTimezonedDayStart {
    pub fn zero() -> Self {
        Self { seconds: 0 }
    }

    pub fn max() -> Self {
        Self {
            seconds : MAX_SECONDS_SINCE_TIMEZONED_DAY_START
        }
    }

    pub fn min() -> Self {
        Self {
            seconds : MAX_SECONDS_SINCE_TIMEZONED_DAY_START
        }
    }

    pub fn from_seconds( seconds : i32) -> Option<Self> {
        if seconds > MAX_SECONDS_SINCE_TIMEZONED_DAY_START || seconds < - MAX_SECONDS_SINCE_TIMEZONED_DAY_START {
            None
        }
        else {
            let result = Self{ seconds  };
            Some(result)
        }
        
    }

    pub fn from_seconds_i64( seconds_i64 : i64) -> Option<Self> {
        let max_i64 = i64::from(MAX_SECONDS_SINCE_TIMEZONED_DAY_START);
        if seconds_i64 > max_i64  || seconds_i64 < - max_i64 {
            None
        }
        else {
            // since  :
            //  - seconds_i64 belongs to [-MAX_SECONDS_SINCE_TIMEZONED_DAY_START, MAX_SECONDS_SINCE_TIMEZONED_DAY_START] 
            //  - MAX_SECONDS_SINCE_TIMEZONED_DAY_START <= i32::MAX
            // we can safely cas seconds_i64 to i32
            let seconds_i32 = seconds_i64 as i32;
            let result = Self{ seconds : seconds_i32  };
            Some(result)
        }
        
    }

}


impl std::fmt::Display for SecondsSinceTimezonedDayStart {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:02}:{:02}:{:02}",
            self.seconds / 60 / 60,
            self.seconds / 60 % 60,
            self.seconds % 60
        )
    }
}



impl SecondsSinceDatasetUTCStart {

    pub fn duration_since(&self, start_datetime : & SecondsSinceDatasetUTCStart) -> Option<PositiveDuration> {
        self.seconds.checked_sub(start_datetime.seconds)
            .map(|seconds| PositiveDuration{seconds})
    }
}

impl std::ops::Add for PositiveDuration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            seconds: self.seconds + rhs.seconds,
        }
    }
}

impl std::ops::Add<PositiveDuration> for SecondsSinceDatasetUTCStart {
    type Output = Self;

    fn add(self, rhs: PositiveDuration) -> Self::Output {
        Self {
            seconds: self.seconds + rhs.seconds,
        }
    }
}

impl std::ops::Mul<u32> for PositiveDuration {
    type Output = Self;

    fn mul(self, rhs: u32) -> Self::Output {
        PositiveDuration {
            seconds: self.seconds * rhs,
        }
    }
}

// Iterator that provides decompositions of `datetime_timezoned_to_decompose`
// in the form of  a `(day, time_in_timezoned_day)`.
// It will advance the `day` until `time_in_timezoned_day`
// becomes strictly smaller than `min_seconds_since_timezoned_day_start`
pub struct ForwardDecompose<'calendar> {
    datetime_timezoned_to_decompose : chrono::DateTime<Timezone>,
    has_reference_date : Option<chrono::Date<Timezone>>,
    min_seconds_since_timezoned_day_start : SecondsSinceTimezonedDayStart,
    calendar : & 'calendar Calendar,
}

impl<'calendar>  ForwardDecompose<'calendar> {
    fn new(
        seconds_since_dataset_start : & SecondsSinceDatasetUTCStart,
        min_seconds_since_timezoned_day_start : SecondsSinceTimezonedDayStart,
        timezone : &Timezone,
        calendar : & 'calendar Calendar,
    ) -> Self {
        let datetime_utc = calendar.first_datetime() + chrono::Duration::seconds(i64::from(seconds_since_dataset_start.seconds));
        debug_assert!(calendar.contains_datetime(&datetime_utc));
        use chrono::TimeZone;
        let datetime_timezoned = timezone.from_utc_datetime(&datetime_utc);
        let date_timezoned = datetime_timezoned.date();
        Self {
            datetime_timezoned_to_decompose : datetime_timezoned,
            has_reference_date : Some(date_timezoned),
            min_seconds_since_timezoned_day_start,
            calendar
        }
    }

}


impl<'calendar> Iterator for ForwardDecompose<'calendar> {
    type Item = (DaysSinceDatasetStart, SecondsSinceTimezonedDayStart);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref reference_date) = self.has_reference_date {
            let reference_datetime_timezoned = reference_date.and_hms(12, 0, 0) - chrono::Duration::hours(12);
            let seconds_i64 = (self.datetime_timezoned_to_decompose.clone() - reference_datetime_timezoned).num_seconds();

            let has_seconds = SecondsSinceTimezonedDayStart::from_seconds_i64(seconds_i64);
            let has_reference_day = self.calendar.date_to_days_since_start(&reference_date.naive_local());
            match (has_reference_day, has_seconds) {
                (Some(reference_day), Some(seconds)) if seconds >= self.min_seconds_since_timezoned_day_start => {
                    self.has_reference_date = reference_date.succ_opt();
                    Some((reference_day, seconds))
                }
                _ => {
                    self.has_reference_date = None;
                    None
                }
            }
        }
        else {
            None
        }

    }
}




// Iterator that provides decompositions of `datetime_timezoned_to_decompose`
// in the form of  a `(day, time_in_timezoned_day)`.
// It will decrease the `day` until `time_in_timezoned_day`
// becomes strictly greater than `max_seconds_since_timezoned_day_start`
pub struct BackwardDecompose<'calendar> {
    datetime_timezoned_to_decompose : chrono::DateTime<Timezone>,
    has_reference_date : Option<chrono::Date<Timezone>>,
    max_seconds_since_timezoned_day_start : SecondsSinceTimezonedDayStart,
    calendar : & 'calendar Calendar,
}

impl<'calendar>  BackwardDecompose<'calendar> {
    fn new(
        seconds_since_dataset_start : & SecondsSinceDatasetUTCStart,
        max_seconds_since_timezoned_day_start : SecondsSinceTimezonedDayStart,
        timezone : & Timezone,
        calendar : & 'calendar Calendar,
    ) -> Self {
        let datetime_utc = calendar.first_datetime() + chrono::Duration::seconds(i64::from(seconds_since_dataset_start.seconds));
        debug_assert!(calendar.contains_datetime(&datetime_utc));
        use chrono::TimeZone;
        let datetime_timezoned = timezone.from_utc_datetime(&datetime_utc);
        let date_timezoned = datetime_timezoned.date();
        Self {
            datetime_timezoned_to_decompose : datetime_timezoned,
            has_reference_date : Some(date_timezoned),
            max_seconds_since_timezoned_day_start,
            calendar
        }
    }

}


impl<'calendar> Iterator for BackwardDecompose<'calendar> {
    type Item = (DaysSinceDatasetStart, SecondsSinceTimezonedDayStart);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref reference_date) = self.has_reference_date {
            let reference_datetime_timezoned = reference_date.and_hms(12, 0, 0) - chrono::Duration::hours(12);
            let seconds_i64 = (self.datetime_timezoned_to_decompose.clone() - reference_datetime_timezoned).num_seconds();

            let has_seconds = SecondsSinceTimezonedDayStart::from_seconds_i64(seconds_i64);
            let has_reference_day = self.calendar.date_to_days_since_start(&reference_date.naive_local());
            match (has_reference_day, has_seconds) {
                (Some(reference_day), Some(seconds)) if seconds <= self.max_seconds_since_timezoned_day_start => {
                    self.has_reference_date = reference_date.pred_opt();
                    Some((reference_day, seconds))
                }
                _ => {
                    self.has_reference_date = None;
                    None
                }
            }
        }
        else {
            None
        }

    }
}