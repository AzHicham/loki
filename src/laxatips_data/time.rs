use std::fmt::{Display, Formatter};
use chrono_tz::Tz as Timezone;

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
    pub fn decompose(&self, calendar : & Calendar, timezone : &Timezone) -> (DaysSinceDatasetStart, SecondsSinceTimezonedDayStart) {
        self.decompose_with_days_offset(0, calendar, timezone).unwrap()
    }

    pub fn decompose_with_days_offset(
        &self,
        nb_of_days_to_offset: u16,
        calendar : & Calendar,
        timezone : &Timezone,
    ) -> Option<(DaysSinceDatasetStart, SecondsSinceTimezonedDayStart)> {
        let datetime_utc = calendar.first_date().and_hms(0, 0, 0) + chrono::Duration::seconds(self.seconds as i64);
        use chrono::offset::TimeZone;
        let datetime_timezoned = timezone.from_utc_datetime(&datetime_utc);
        let date = datetime_timezoned.date().naive_utc();
        let reference_date = date.checked_sub_signed(chrono::Duration::days(nb_of_days_to_offset as i64))?;
        let reference_datetime_utc = reference_date.and_hms(12, 0, 0) - chrono::Duration::hours(12);
        let reference_datetime_timezoned = timezone.from_local_datetime(&reference_datetime_utc).earliest()?;
        let reference_day = calendar.date_to_days_since_start(&reference_date)?;
        let seconds_i64 = (datetime_timezoned - reference_datetime_timezoned).num_seconds();
        use std::convert::TryFrom;
        let seconds = u32::try_from(seconds_i64).ok()
            .map(|seconds_u32|
                SecondsSinceTimezonedDayStart {
                    seconds : seconds_u32,
                }
            )?;
        Some((reference_day, seconds))

    }



    #[inline(always)]
    pub fn compose(day: &DaysSinceDatasetStart, seconds_in_day: &SecondsSinceTimezonedDayStart, calendar : & Calendar, timezone : &Timezone) -> Self {
        let date = *calendar.first_date() + chrono::Duration::days(day.days as i64);
        use chrono::offset::TimeZone;
        let datetime_timezoned = timezone.from_utc_date(&date).and_hms(0, 0, 0) + chrono::Duration::seconds(seconds_in_day.seconds as i64);
        use chrono_tz::UTC;
        let datetime_utc = datetime_timezoned.with_timezone(&UTC).naive_utc();
        
        calendar.naive_datetime_to_seconds_since_start(&datetime_utc).unwrap()

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

