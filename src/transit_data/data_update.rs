// Copyright  (C) 2020, Kisio Digital and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Kisio Digital (www.kisio.com).
// Help us simplify mobility and open public transport:
// a non ending quest to the responsive locomotion way of traveling!
//
// This contribution is a part of the research and development work of the
// IVA Project which aims to enhance traveler information and is carried out
// under the leadership of the Technological Research Institute SystemX,
// with the partnership and support of the transport organization authority
// Ile-De-France Mobilités (IDFM), SNCF, and public funds
// under the scope of the French Program "Investissements d’Avenir".
//
// LICENCE: This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// Stay tuned using
// twitter @navitia
// channel `#navitia` on riot https://riot.im/app/#/room/#navitia:matrix.org
// https://groups.google.com/d/forum/navitia
// www.navitia.io

use std::fmt::Debug;

use crate::{
    loads_data::LoadsData,
    models::{StopPointIdx, VehicleJourneyIdx},
    timetables::{InsertionError, RemovalError},
    transit_data::TransitData,
};

use crate::{
    time::SecondsSinceTimezonedDayStart,
    timetables::{FlowDirection, Timetables as TimetablesTrait, TimetablesIter},
};
use chrono::NaiveDate;

use tracing::warn;

use super::{data_interface, init::restrict_dates};

impl<Timetables> data_interface::DataUpdate for TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
{
    fn remove_vehicle(
        &mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &chrono::NaiveDate,
    ) -> Result<(), RemovalError> {
        if *date < self.start_date || *date > self.end_date {
            Err(RemovalError::UnknownDate(
                *date,
                vehicle_journey_idx.clone(),
            ))
        } else {
            self.timetables.remove(date, vehicle_journey_idx)
        }
    }

    fn add_vehicle<'date, Stops, Flows, Dates, BoardTimes, DebarkTimes>(
        &mut self,
        stop_points: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads_data: &LoadsData,
        valid_dates: Dates,
        timezone: &chrono_tz::Tz,
        vehicle_journey_idx: VehicleJourneyIdx,
    ) -> Vec<InsertionError>
    where
        Stops: Iterator<Item = StopPointIdx> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        Dates: Iterator<Item = &'date chrono::NaiveDate> + Clone,
        BoardTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
    {
        let mut errors = Vec::new();
        let start_date = self.start_date;
        let end_date = self.end_date;
        for date in valid_dates.clone() {
            if *date < start_date || *date > end_date {
                errors.push(InsertionError::InvalidDate(
                    *date,
                    vehicle_journey_idx.clone(),
                ));
            }
        }

        let dates = valid_dates.filter(|&&date| date >= start_date && date <= end_date);

        let stops = self.create_stops(stop_points).into_iter();
        let (missions, insertion_errors) = self.timetables.insert(
            stops,
            flows,
            board_times,
            debark_times,
            loads_data,
            dates,
            timezone,
            &vehicle_journey_idx,
        );

        for mission in missions.iter() {
            for position in self.timetables.positions(mission) {
                let stop = self.timetables.stop_at(&position, mission);
                let stop_data = &mut self.stops_data[stop.idx];
                stop_data
                    .position_in_timetables
                    .push((mission.clone(), position));
            }
        }

        errors.extend(insertion_errors);
        errors
    }

    fn modify_vehicle<'date, Stops, Flows, Dates, BoardTimes, DebarkTimes>(
        &mut self,
        stops: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads_data: &LoadsData,
        valid_dates: Dates,
        timezone: &chrono_tz::Tz,
        vehicle_journey_idx: VehicleJourneyIdx,
    ) -> (Vec<RemovalError>, Vec<InsertionError>)
    where
        Stops: Iterator<Item = StopPointIdx> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        Dates: Iterator<Item = &'date chrono::NaiveDate> + Clone,
        BoardTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
    {
        let mut removal_errors = Vec::new();
        let mut insertion_errors = Vec::new();

        let start_date = self.start_date;
        let end_date = self.end_date;
        for date in valid_dates.clone() {
            if *date < start_date || *date > end_date {
                insertion_errors.push(InsertionError::InvalidDate(
                    *date,
                    vehicle_journey_idx.clone(),
                ));
            }
        }

        let dates = valid_dates.filter(|&&date| date >= start_date && date <= end_date);

        for date in dates.clone() {
            let removal_result = self.remove_vehicle(&vehicle_journey_idx, date);
            match removal_result {
                Ok(()) => {
                    let errors = self.add_vehicle(
                        stops.clone(),
                        flows.clone(),
                        board_times.clone(),
                        debark_times.clone(),
                        loads_data,
                        dates.clone(),
                        timezone,
                        vehicle_journey_idx.clone(),
                    );
                    insertion_errors.extend_from_slice(errors.as_slice());
                }
                Err(removal_error) => {
                    removal_errors.push(removal_error);
                }
            }
        }
        (removal_errors, insertion_errors)
    }

    fn set_start_end_date(
        &mut self,
        restricted_start_date: &NaiveDate,
        restricted_end_date: &NaiveDate,
    ) -> (Vec<NaiveDate>, Vec<NaiveDate>) {
        let old_start_date = self.start_date;
        let old_end_date = self.end_date;
        let calendar = self.timetables.calendar();
        let calendar_start_date = calendar.first_date();
        let calendar_end_date = calendar.last_date();
        let (start_date, end_date) = restrict_dates(
            calendar_start_date,
            calendar_end_date,
            restricted_start_date,
            restricted_end_date,
        );

        let mut removed_days = Vec::new();
        if old_start_date <= old_end_date {
            let num_days = (old_end_date - old_start_date).num_days();
            if num_days >= 0 {
                for day_offset in 0..=num_days {
                    let date = old_start_date + chrono::Duration::days(day_offset);
                    if date < start_date || date > end_date {
                        removed_days.push(date);
                        self.remove_all_vehicles_on_date(&date);
                    }
                }
            }
        }
        let mut added_days = Vec::new();
        if start_date <= end_date {
            let num_days = (end_date - start_date).num_days();
            if num_days >= 0 {
                for day_offset in 0..=num_days {
                    let date = start_date + chrono::Duration::days(day_offset);
                    if date < old_start_date || date > old_end_date {
                        added_days.push(date);
                    }
                }
            }
        }

        self.start_date = start_date;
        self.end_date = end_date;
        (removed_days, added_days)
    }

    fn start_date(&self) -> &NaiveDate {
        &self.start_date
    }

    fn end_date(&self) -> &NaiveDate {
        &self.end_date
    }
}

impl<Timetables> TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
{
    fn remove_all_vehicles_on_date(&mut self, date: &NaiveDate) {
        if *date < self.start_date || *date > self.end_date {
            warn!(
                "Trying to remove all vehicles on day {}, which is invalid for the data. \
                    Allowed dates are between {} and {}",
                date, self.start_date, self.end_date
            );
            return;
        }
        self.timetables.remove_all_vehicle_on_day(date)
    }
}
