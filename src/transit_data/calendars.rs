use transit_model::objects::Date;
use std::convert::TryFrom;

use super::time::DaysSinceDatasetStart;

pub struct Calendars{
    first_date : Date, //first date which may be allowed
    last_date : Date,  //last date (included) which may be allowed
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

    pub fn new(first_date : Date, last_date : Date) -> Self {
        assert!(first_date <= last_date);
        let nb_of_days_i64 : i64 = (last_date - first_date).num_days();

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

    pub fn get_or_insert<'a, Dates>(&mut self, dates : Dates) -> CalendarIdx 
    where Dates : Iterator<Item = & 'a Date>
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



    fn contains(&self, date : & Date) -> bool {
        self.first_date <= *date && *date <= self.last_date
    }

    fn date_to_offset(&self, date : & Date) ->  Option<usize> 
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
