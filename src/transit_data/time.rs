
const SECONDS_IN_A_DAY : u32 = 60 * 60 * 24;

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct SecondsSinceDayStart {
    pub (super) seconds : u32
}
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct SecondsSinceDatasetStart {
    pub (super) seconds : u32
}

pub struct DaysSinceDatasetStart {
    pub (super) days : u16
}

#[derive(Debug, Clone, Copy)]
pub struct PositiveDuration {
    pub(super) seconds : u32
}

impl SecondsSinceDatasetStart {
       // TODO : add doc and doctest
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

    pub fn compose( days : & DaysSinceDatasetStart, seconds_in_day : & SecondsSinceDayStart) -> Self {
        let days_u32 : u32 = days.days.into();
        let seconds : u32 = SECONDS_IN_A_DAY * days_u32   + seconds_in_day.seconds;
        Self {
            seconds
        }
    }

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