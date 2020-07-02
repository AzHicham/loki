use std::fmt::{Display, Formatter};

const SECONDS_IN_A_DAY : u32 = 60 * 60 * 24;

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct SecondsSinceDayStart {
    pub (super) seconds : u32
}
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct SecondsSinceDatasetStart {
    pub (super) seconds : u32
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DaysSinceDatasetStart {
    pub (super) days : u16
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
pub struct PositiveDuration {
    pub seconds : u32
}

impl PositiveDuration {
    pub fn zero() -> Self {
        Self {
            seconds : 0
        }
    }

    pub fn from_hms(hours : u32, minutes : u32, seconds : u32) -> PositiveDuration {
        let total_seconds = seconds + 60 * minutes + 60*60*hours;
        PositiveDuration{seconds : total_seconds}
    }
}


impl Display for PositiveDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {

        let hours = self.seconds / (60*60);
        let minutes_in_secs = self.seconds % (60*60);
        let minutes = minutes_in_secs / 60;
        let seconds = minutes_in_secs % 60;
        if hours != 0 {
            write!(f, "{}h{:02}m{:02}s", hours, minutes, seconds )
        }
        else if minutes != 0 {
            write!(f, "{}m{:02}s", minutes, seconds)
        }
        else {
            write!(f, "{}s", seconds)
        }
        
    }
}

impl SecondsSinceDayStart {
    pub fn zero() -> Self {
        Self {
            seconds : 0
        }
    }
}

impl SecondsSinceDatasetStart {

    pub fn zero() -> Self {
        SecondsSinceDatasetStart{seconds : 0}
    }

    // TODO : add doc and doctest
    #[inline(always)]
    pub fn decompose(&self) -> (DaysSinceDatasetStart, SecondsSinceDayStart) {
        let (days_u16, seconds_u32) = self.decompose_inner();

        let days =  DaysSinceDatasetStart {
            days : days_u16
        };
        let seconds = SecondsSinceDayStart {
            seconds : seconds_u32
        };

        (days, seconds)

    }

    // TODO : add doc and doctest
    pub fn decompose_with_days_offset(&self, nb_of_days_to_offset : u16) -> Option<(DaysSinceDatasetStart, SecondsSinceDayStart)>
    {
        let (canonical_days_u16, canonical_seconds_u32) = self.decompose_inner();
        let has_days_u16 = canonical_days_u16.checked_sub(nb_of_days_to_offset);
        has_days_u16.map(|days_u16| {
            let days = DaysSinceDatasetStart {
                days : days_u16
            };
            let days_offset_u32 : u32 = nb_of_days_to_offset.into();
            let seconds_u32 = canonical_seconds_u32 + days_offset_u32 * SECONDS_IN_A_DAY;
            let seconds = SecondsSinceDayStart {
                seconds : seconds_u32
            };
            (days, seconds)
        })

        

    }
    
    #[inline(always)]
    pub fn compose( days : & DaysSinceDatasetStart, seconds_in_day : & SecondsSinceDayStart) -> Self {
        let days_u32 : u32 = days.days.into();
        let seconds : u32 = SECONDS_IN_A_DAY * days_u32   + seconds_in_day.seconds;
        Self {
            seconds
        }
    }

    #[inline(always)]
    fn decompose_inner(&self) -> (u16, u32)
    {
        let days_u32 = self.seconds / SECONDS_IN_A_DAY;

        // Dangerous cast, that we check in debug build only
        debug_assert!(days_u32 <= (u16::MAX as u32) );
        let days_u16 = days_u32 as u16;

        let seconds = self.seconds % SECONDS_IN_A_DAY;

        (days_u16, seconds)
    }
}


impl std::ops::Add for PositiveDuration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            seconds : self.seconds + rhs.seconds
        }
    }
}

impl std::ops::Add<PositiveDuration> for SecondsSinceDatasetStart {
    type Output = Self;

    fn add(self, rhs : PositiveDuration) -> Self::Output {
        Self {
            seconds : self.seconds + rhs.seconds
        }
    }
}

impl std::ops::Mul<u32> for PositiveDuration {
    type Output = Self;

    fn mul(self, rhs: u32) -> Self::Output {
        PositiveDuration {
            seconds : self.seconds * rhs
        }
    }
}


impl std::fmt::Display for SecondsSinceDayStart {
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