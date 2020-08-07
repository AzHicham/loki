use std::fmt::{Display, Formatter};
use chrono_tz::Tz as Timezone;

const SECONDS_IN_A_DAY: u32 = 60 * 60 * 24;

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct SecondsSinceTimezonedDayStart {
    pub(super) seconds: u32,
}
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct SecondsSinceDatasetUTCStart {
    pub(super) seconds: u32,
}
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct DaysSinceDatasetStart {
    pub(super) days: u16,
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



use crate::laxatips_data::calendar::Calendar;

impl SecondsSinceDatasetUTCStart {
    pub fn zero() -> Self {
        Self { seconds: 0 }
    }

    // TODO : add doc and doctest
    #[inline(always)]
    pub fn decompose(&self) -> (DaysSinceDatasetStart, SecondsSinceTimezonedDayStart) {
        // let utc_datetime = calendar.first_date().and_hms(0, 0, self.seconds);
        // let timezoned_datetime  : chrono::DateTime<Timezone>= chrono::DateTime::from_utc(utc_datetime, timezone);
        // let date = timezoned_datetime.date();
        // let timezoned_day_begin = date.and_hms(0, 12, 0) - chrono::Duration::hours(12);
        // let seconds_in_day_i64 = timezoned_datetime.signed_duration_since(timezoned_day_begin).num_seconds();
        // use std::convert::TryFrom;
        // let seconds_in_day_u32 = u32::try_from(seconds_in_day_i64).unwrap();
        // use chrono::Date;
        // let number_of_days_i64 = (date - Date::from_utc(*calendar.first_date(), timezone)).num_days();
        // let number_of_days_u16 = u16::try_from(number_of_days_i64).unwrap();

        let (days_u16, seconds_u32) = self.decompose_inner();

        let days = DaysSinceDatasetStart { days: days_u16 };
        let seconds = SecondsSinceTimezonedDayStart {
            seconds: seconds_u32,
        };

        (days, seconds)
    }

    // TODO : add doc and doctest
    pub fn decompose_with_days_offset(
        &self,
        nb_of_days_to_offset: u16,
    ) -> Option<(DaysSinceDatasetStart, SecondsSinceTimezonedDayStart)> {
        let (canonical_days_u16, canonical_seconds_u32) = self.decompose_inner();
        let has_days_u16 = canonical_days_u16.checked_sub(nb_of_days_to_offset);
        has_days_u16.map(|days_u16| {
            let days = DaysSinceDatasetStart { days: days_u16 };
            let days_offset_u32: u32 = nb_of_days_to_offset.into();
            let seconds_u32 = canonical_seconds_u32 + days_offset_u32 * SECONDS_IN_A_DAY;
            let seconds = SecondsSinceTimezonedDayStart {
                seconds: seconds_u32,
            };
            (days, seconds)
        })
    }

    #[inline(always)]
    pub fn compose(days: &DaysSinceDatasetStart, seconds_in_day: &SecondsSinceTimezonedDayStart) -> Self {
        let days_u32: u32 = days.days.into();
        let seconds: u32 = SECONDS_IN_A_DAY * days_u32 + seconds_in_day.seconds;
        Self { seconds }
    }

    #[inline(always)]
    fn decompose_inner(&self) -> (u16, u32) {
        let days_u32 = self.seconds / SECONDS_IN_A_DAY;

        // Dangerous cast, that we check in debug build only
        debug_assert!(days_u32 <= (u16::MAX as u32));
        let days_u16 = days_u32 as u16;

        let seconds = self.seconds % SECONDS_IN_A_DAY;

        (days_u16, seconds)
    }

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

