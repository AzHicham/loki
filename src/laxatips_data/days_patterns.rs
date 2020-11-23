use crate::time::{DaysSinceDatasetStart, Calendar};
use chrono::{NaiveDate};

pub struct DaysPatterns {
    days_patterns: Vec<DaysPatternData>,

    buffer: Vec<bool>,
}

struct DaysPatternData {
    allowed_dates: Vec<bool>,
}

#[derive(Debug, Copy, Clone)]
pub struct DaysPattern {
    idx: usize,
}

impl DaysPatterns {

    pub fn new(nb_of_days : usize ) -> Self {
        Self {
            days_patterns : Vec::new(),
            buffer :  vec![false; nb_of_days],
        }
    }

    pub fn is_allowed(&self, days_pattern: &DaysPattern, day: &DaysSinceDatasetStart) -> bool {
        debug_assert!((day.days as usize) < self.buffer.len());
        debug_assert!(days_pattern.idx < self.days_patterns.len());
        let day_idx: usize = day.days.into();
        self.days_patterns[days_pattern.idx].allowed_dates[day_idx]
    }

    pub fn get_or_insert<'a, Dates>(&mut self, dates: Dates, calendar : & Calendar) -> DaysPattern
    where
        Dates: Iterator<Item = &'a NaiveDate>,
    {
        // set all elements of the buffer to false
        for val in self.buffer.iter_mut() {
            *val = false
        }

        for date in dates {
            let has_offset = calendar.date_to_offset(date);
            if let Some(offset) = has_offset {
                self.buffer[offset as usize] = true;
            }
        }

        let has_days_pattern = self
            .days_patterns
            .iter()
            .enumerate()
            .find(|(_, calendar)| calendar.allowed_dates == self.buffer)
            .map(|(idx, _)| idx);

        let idx = if let Some(idx) = has_days_pattern {
            idx
        } else {
            let idx = self.days_patterns.len();
            let days_pattern_data = DaysPatternData {
                allowed_dates: self.buffer.clone(),
            };
            self.days_patterns.push(days_pattern_data);
            idx
        };

        DaysPattern { idx }
    }

}

