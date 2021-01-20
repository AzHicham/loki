use chrono::NaiveDate;
use std::fmt::{Display, Formatter};

mod calendar;
pub mod days_patterns;

/// Duration since "noon minus 12 hours" on a day in a specific timezone
/// This corresponds to the "Time" notion found in gtfs/ntfs stop_times.txt
/// It should be built from a TransitModelTime.
/// This types accept only times are comprised between -48:00:00 and 48:00:00 (maximum plus/minus 2 days)
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct SecondsSinceTimezonedDayStart {
    seconds: i32,
}

const MAX_SECONDS_SINCE_TIMEZONED_DAY_START: i32 = 48 * 60 * 60; // 48h

const MAX_TIMEZONE_OFFSET: i32 = 24 * 60 * 60; // 24h in seconds

/// Duration since 00:00:00 UTC in the first allowed day of the data
/// This is used in the engine to store a point in time in an unambiguous way
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct SecondsSinceDatasetUTCStart {
    seconds: u32,
}

/// Number of days since the first allowed day of the data
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct DaysSinceDatasetStart {
    pub(super) days: u16,
}

// we allow 36_600 days which is more than 100 years, and less than u16::MAX = 65_535 days
const MAX_DAYS_IN_CALENDAR: u16 = 100 * 366;

pub struct Calendar {
    first_date: NaiveDate, //first date which may be allowed
    last_date: NaiveDate,  //last date (included) which may be allowed
    nb_of_days: u16,       // == (last_date - first_date).num_of_days() + 1
                           // we allow at most MAX_DAYS_IN_CALENDAR days
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
pub struct PositiveDuration {
    pub(super) seconds: u32,
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
            seconds: MAX_SECONDS_SINCE_TIMEZONED_DAY_START,
        }
    }

    pub fn min() -> Self {
        Self {
            seconds: MAX_SECONDS_SINCE_TIMEZONED_DAY_START,
        }
    }

    pub fn from_seconds(seconds: i32) -> Option<Self> {
        if seconds > MAX_SECONDS_SINCE_TIMEZONED_DAY_START
            || seconds < -MAX_SECONDS_SINCE_TIMEZONED_DAY_START
        {
            None
        } else {
            let result = Self { seconds };
            Some(result)
        }
    }

    pub fn from_seconds_i64(seconds_i64: i64) -> Option<Self> {
        let max_i64 = i64::from(MAX_SECONDS_SINCE_TIMEZONED_DAY_START);
        if seconds_i64 > max_i64 || seconds_i64 < -max_i64 {
            None
        } else {
            // since  :
            //  - seconds_i64 belongs to [-MAX_SECONDS_SINCE_TIMEZONED_DAY_START, MAX_SECONDS_SINCE_TIMEZONED_DAY_START]
            //  - MAX_SECONDS_SINCE_TIMEZONED_DAY_START <= i32::MAX
            // we can safely cas seconds_i64 to i32
            let seconds_i32 = seconds_i64 as i32;
            let result = Self {
                seconds: seconds_i32,
            };
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
    pub fn duration_since(
        &self,
        start_datetime: &SecondsSinceDatasetUTCStart,
    ) -> Option<PositiveDuration> {
        self.seconds
            .checked_sub(start_datetime.seconds)
            .map(|seconds| PositiveDuration { seconds })
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
