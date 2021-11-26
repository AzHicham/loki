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

use crate::{
    loads_data::{Load, LoadsData},
    models::VehicleJourneyIdx,
    time::days_patterns::{DaysInPatternIter, DaysPattern, DaysPatterns},
    RealTimeLevel,
};

use super::{
    day_to_timetable::{Unknown, VehicleJourneyToTimetable},
    generic_timetables::{Timetables, Trip, Vehicle},
    InsertionError, RealTimeValidity, RemovalError, TimetablesIter,
};
use crate::time::{
    Calendar, DaysSinceDatasetStart, SecondsSinceDatasetUTCStart, SecondsSinceTimezonedDayStart,
    SecondsSinceUTCDayStart, TimezonesPatterns,
};
use crate::timetables::generic_timetables::{Position, Timetable};
use chrono::NaiveDate;
use core::cmp;
use std::collections::BTreeMap;
use tracing::log::error;

use crate::timetables::{
    FlowDirection, Stop, Timetables as TimetablesTrait, Types as TimetablesTypes,
};


pub struct PeriodicSplitVjByTzTimetables {
    timetables: Timetables<SecondsSinceUTCDayStart, Load, (), VehicleData>,
    calendar: Calendar,
    days_patterns: DaysPatterns,
    timezones_patterns: TimezonesPatterns,
    vehicle_journey_to_timetable: VehicleJourneyToTimetable,
}

// A vj can be
// BaseAndRealTime : une seule version par offset
// BaseOnly

#[derive(Debug, Clone)]
pub struct VehicleData {
    days_pattern: DaysPattern,
    vehicle_journey_idx: VehicleJourneyIdx,
    real_time_validity: RealTimeValidity,
}

impl TimetablesTypes for PeriodicSplitVjByTzTimetables {
    type Mission = Timetable;
    type Position = Position;
    type Trip = Trip;
}

impl TimetablesTrait for PeriodicSplitVjByTzTimetables {
    fn new(first_date: NaiveDate, last_date: NaiveDate) -> Self {
        let calendar = Calendar::new(first_date, last_date);
        let nb_of_days: usize = calendar.nb_of_days().into();
        Self {
            timetables: Timetables::new(),
            calendar,
            days_patterns: DaysPatterns::new(nb_of_days),
            timezones_patterns: TimezonesPatterns::new(),
            vehicle_journey_to_timetable: VehicleJourneyToTimetable::new(),
        }
    }

    fn calendar(&self) -> &Calendar {
        &self.calendar
    }

    fn nb_of_missions(&self) -> usize {
        self.timetables.nb_of_timetables()
    }

    fn mission_id(&self, mission: &Self::Mission) -> usize {
        mission.idx
    }

    fn vehicle_journey_idx(&self, trip: &Self::Trip) -> VehicleJourneyIdx {
        self.timetables
            .vehicle_data(&trip.vehicle)
            .vehicle_journey_idx
            .clone()
    }

    fn stoptime_idx(&self, position: &Self::Position, _trip: &Self::Trip) -> usize {
        self.timetables.stoptime_idx(position)
    }

    fn day_of(&self, trip: &Self::Trip) -> NaiveDate {
        self.calendar().to_naive_date(&trip.day)
    }

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission {
        self.timetables.timetable_of(&trip.vehicle)
    }

    fn stop_at(&self, position: &Self::Position, mission: &Self::Mission) -> super::Stop {
        *self.timetables.stop_at(position, mission)
    }

    fn nb_of_trips(&self) -> usize {
        self.timetables.nb_of_trips()
    }

    fn is_upstream_in_mission(
        &self,
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool {
        self.timetables.is_upstream(upstream, downstream, mission)
    }

    fn next_position(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.timetables.next_position(position, mission)
    }

    fn previous_position(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.timetables.previous_position(position, mission)
    }

    fn arrival_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load) {
        let (time_in_day, load) = self.timetables.arrival_time(&trip.vehicle, position);
        let time_utc = self.calendar.compose_utc(&trip.day, time_in_day);
        (time_utc, *load)
    }

    fn departure_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load) {
        let (time_in_day, load) = self.timetables.departure_time(&trip.vehicle, position);
        let time_utc = self.calendar.compose_utc(&trip.day, time_in_day);
        (time_utc, *load)
    }

    fn debark_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)> {
        let has_timeload_in_day = self.timetables.debark_time(&trip.vehicle, position);
        has_timeload_in_day.map(|(time_in_day, load)| {
            let day = &trip.day;
            let time = self.calendar.compose_utc(day, time_in_day);
            (time, *load)
        })
    }

    fn board_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)> {
        let has_timeload_in_day = self.timetables.board_time(&trip.vehicle, position);
        has_timeload_in_day.map(|(time_in_day, load)| {
            let day = &trip.day;
            let time = self.calendar.compose_utc(day, time_in_day);
            (time, *load)
        })
    }

    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        real_time_level: &RealTimeLevel,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        self.earliest_filtered_trip_to_board_at(
            waiting_time,
            mission,
            position,
            real_time_level,
            |_| true,
        )
    }

    fn earliest_filtered_trip_to_board_at<Filter>(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        real_time_level: &RealTimeLevel,
        filter: Filter,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)>
    where
        Filter: Fn(&VehicleJourneyIdx) -> bool,
    {
        let has_earliest_and_latest_board_time =
            self.timetables.earliest_and_latest_board_time(position);

        // if there is no earliest/latest board time, it means that this position cannot be boarded
        // and we return None
        let (_earliest_board_time_in_day, _latest_board_time_in_day) =
            has_earliest_and_latest_board_time?;

        let decompositions = self.calendar.decompositions_utc(waiting_time);

        let mut best_vehicle_day_and_its_arrival_time_at_next_position: Option<(
            Vehicle,
            DaysSinceDatasetStart,
            SecondsSinceDatasetUTCStart,
            Load,
        )> = None;

        for (waiting_day, waiting_time_in_day) in decompositions {
            let has_vehicle = self.timetables.earliest_filtered_vehicle_to_board(
                &waiting_time_in_day,
                mission,
                position,
                |vehicle_data| {
                    let days_pattern = vehicle_data.days_pattern;
                    self.days_patterns.is_allowed(&days_pattern, &waiting_day)
                        && filter(&vehicle_data.vehicle_journey_idx)
                        && vehicle_data
                            .real_time_validity
                            .is_valid_for(real_time_level)
                },
            );
            if let Some((vehicle, arrival_time_in_day_at_next_stop, load)) = has_vehicle {
                let arrival_time_at_next_stop = self
                    .calendar
                    .compose_utc(&waiting_day, arrival_time_in_day_at_next_stop);

                if let Some((_, _, best_arrival_time, best_load)) =
                    &best_vehicle_day_and_its_arrival_time_at_next_position
                {
                    if arrival_time_at_next_stop < *best_arrival_time
                        || (arrival_time_at_next_stop == *best_arrival_time && load < best_load)
                    {
                        best_vehicle_day_and_its_arrival_time_at_next_position =
                            Some((vehicle, waiting_day, arrival_time_at_next_stop, *load));
                    }
                } else {
                    best_vehicle_day_and_its_arrival_time_at_next_position =
                        Some((vehicle, waiting_day, arrival_time_at_next_stop, *load));
                }
            }
        }

        best_vehicle_day_and_its_arrival_time_at_next_position.map(
            |(vehicle, day, arrival_time_at_next_stop, load)| {
                let trip = Trip { vehicle, day };
                (trip, arrival_time_at_next_stop, load)
            },
        )
    }

    fn latest_trip_that_debark_at(
        &self,
        time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        real_time_level: &RealTimeLevel,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        self.latest_filtered_trip_that_debark_at(time, mission, position, real_time_level, |_| true)
    }

    fn latest_filtered_trip_that_debark_at<Filter>(
        &self,
        time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        real_time_level: &RealTimeLevel,
        filter: Filter,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)>
    where
        Filter: Fn(&VehicleJourneyIdx) -> bool,
    {
        let has_earliest_and_latest_debark_time =
            self.timetables.earliest_and_latest_debark_time(position);

        // if there is no earliest/latest debark time, it means that this position cannot be debarked
        // and we return None
        let (_earliest_debark_time_in_day, _latest_debark_time_in_day) =
            has_earliest_and_latest_debark_time?;

        let decompositions = self.calendar.decompositions_utc(time);
        let mut best_vehicle_day_and_its_departure_time_at_previous_position: Option<(
            Vehicle,
            DaysSinceDatasetStart,
            SecondsSinceDatasetUTCStart,
            Load,
        )> = None;
        for (waiting_day, waiting_time_in_day) in decompositions {
            let has_vehicle = self.timetables.latest_filtered_vehicle_that_debark(
                &waiting_time_in_day,
                mission,
                position,
                |vehicle_data| {
                    let days_pattern = vehicle_data.days_pattern;
                    self.days_patterns.is_allowed(&days_pattern, &waiting_day)
                        && filter(&vehicle_data.vehicle_journey_idx)
                        && vehicle_data
                            .real_time_validity
                            .is_valid_for(real_time_level)
                },
            );
            if let Some((vehicle, departure_time_in_day_at_previous_stop, load)) = has_vehicle {
                let departure_time_at_previous_stop = self
                    .calendar
                    .compose_utc(&waiting_day, departure_time_in_day_at_previous_stop);

                if let Some((_, _, best_departure_time, best_load)) =
                    &best_vehicle_day_and_its_departure_time_at_previous_position
                {
                    if departure_time_at_previous_stop >= *best_departure_time
                        || (departure_time_at_previous_stop == *best_departure_time
                            && load < best_load)
                    {
                        best_vehicle_day_and_its_departure_time_at_previous_position =
                            Some((vehicle, waiting_day, departure_time_at_previous_stop, *load));
                    }
                } else {
                    best_vehicle_day_and_its_departure_time_at_previous_position =
                        Some((vehicle, waiting_day, departure_time_at_previous_stop, *load));
                }
            }
        }

        best_vehicle_day_and_its_departure_time_at_previous_position.map(
            |(vehicle, day, departure_time_at_previous_stop, load)| {
                let trip = Trip { vehicle, day };
                (trip, departure_time_at_previous_stop, load)
            },
        )
    }

    fn insert<'date, Stops, Flows, Dates, BoardTimes, DebarkTimes>(
        &mut self,
        stops: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads_data: &LoadsData,
        valid_dates: Dates,
        timezone: &chrono_tz::Tz,
        vehicle_journey_idx: &VehicleJourneyIdx,
        real_time_level: &RealTimeLevel,
    ) -> (Vec<Timetable>, Vec<InsertionError>)
    where
        Stops: Iterator<Item = Stop> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        Dates: Iterator<Item = &'date chrono::NaiveDate> + Clone,
        BoardTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
    {
        let mut insertion_errors = Vec::new();
        for date in valid_dates.clone() {
            let has_day = self.calendar.date_to_days_since_start(date);
            if let Some(day) = has_day {
                if self
                    .vehicle_journey_to_timetable
                    .get_timetable(
                        vehicle_journey_idx,
                        &day,
                        &self.days_patterns,
                    )
                    .is_ok()
                {
                    let error = InsertionError::VehicleJourneyAlreadyExistsOnDate(
                        *date,
                        vehicle_journey_idx.clone(),
                        real_time_level.clone()
                    );
                    insertion_errors.push(error);
                }
            } else {
                let error = InsertionError::InvalidDate(*date, vehicle_journey_idx.clone());
                insertion_errors.push(error);
            }
        }

        let valid_dates = valid_dates.filter(|date| {
            let has_day = self.calendar.date_to_days_since_start(date);
            if let Some(day) = has_day {
                self.vehicle_journey_to_timetable
                    .get_timetable(
                        vehicle_journey_idx,
                        &day,
                        &self.days_patterns,
                    )
                    .is_err()
            } else {
                false
            }
        });

        // check validity of dates in self.vehicle_journey_to_timetable
        let (missions, insertion_errors) = self.do_insert(stops, flows, board_times, debark_times, loads_data, valid_dates, timezone, vehicle_journey_idx, real_time_validity);
        // update self.vehicle_journey_to_timetable with the returned missions

        (missions, insertion_errors) 

    }

    fn remove(
        &mut self,
        date: &chrono::NaiveDate,
        vehicle_journey_idx: &VehicleJourneyIdx,
    ) -> Result<(), super::RemovalError> {

        let day = self
            .calendar
            .date_to_days_since_start(date)
            .ok_or_else(|| {
                RemovalError::UnknownDate(
                    *date,
                    vehicle_journey_idx.clone(),
                )
            })?;

        let timetable = self
            .vehicle_journey_to_timetable
            .get_timetable(
                vehicle_journey_idx,
                &day,
                &self.days_patterns,
            )
            .map_err(|err| match err {
                Unknown::VehicleJourneyIdx => RemovalError::UnknownVehicleJourney(
                    vehicle_journey_idx.clone(),
                ),
                Unknown::DayForVehicleJourney => RemovalError::DateInvalidForVehicleJourney(
                    *date,
                    vehicle_journey_idx.clone(),
                ),
            })?;

        self.do_remove(timetable, &day, vehicle_journey_idx, real_time_validity);

     

        let removal_result = self.vehicle_journey_to_timetable.remove(
            vehicle_journey_idx,
            &day,
            &real_time_validity,
            &mut self.days_patterns,
        );

        match removal_result {
            Ok(_) => {}
            Err(err) => {
                // we checked at the beginning that self.vehicle_journey_to_timetable
                // does contains a timetable for (vehicle_journey_idx, &day, real_time_validity)
                // so this error should never occurs, but let's log a warning if it happens
                error!(
                    "Error occured while removing a vehicle journey in timetables : {:?}",
                    err
                );
            }
        }

        Ok(())
    }

    fn modify<'date, Stops, Flows, Dates, BoardTimes, DebarkTimes>(
            &mut self,
            stops: Stops,
            flows: Flows,
            board_times: BoardTimes,
            debark_times: DebarkTimes,
            loads_data: &LoadsData,
            valid_dates: Dates,
            timezone: &chrono_tz::Tz,
            vehicle_journey_idx: &VehicleJourneyIdx,
        ) -> (Vec<Self::Mission>, Vec<InsertionError>)
        where
            Stops: Iterator<Item = Stop> + ExactSizeIterator + Clone,
            Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
            Dates: Iterator<Item = &'date chrono::NaiveDate> + Clone,
            BoardTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
            DebarkTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone {
        todo!()
    }

}


impl PeriodicSplitVjByTzTimetables {

    fn do_insert<'date, Stops, Flows, Dates, BoardTimes, DebarkTimes>(
        &mut self,
        stops: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads_data: &LoadsData,
        valid_dates: Dates,
        timezone: &chrono_tz::Tz,
        vehicle_journey_idx: &VehicleJourneyIdx,
        real_time_validity: &RealTimeValidity,
    ) -> (Vec<Timetable>, Vec<InsertionError>)
    where
        Stops: Iterator<Item = Stop> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        Dates: Iterator<Item = &'date chrono::NaiveDate> + Clone,
        BoardTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
    {
        

        let mut load_patterns_dates: BTreeMap<&[Load], Vec<NaiveDate>> = BTreeMap::new();

        let nb_of_positions = stops.len();
        let default_loads = vec![Load::default(); cmp::max(nb_of_positions - 1, 0)];
        for date in valid_dates {
            let loads = loads_data
                .loads(&vehicle_journey_idx.clone(), date)
                .unwrap_or_else(|| default_loads.as_slice());
            load_patterns_dates
                .entry(loads)
                .or_insert_with(Vec::new)
                .push(*date);
        }

        let mut missions = Vec::new();
        let mut insertion_errors = Vec::new();

        for (loads, dates) in load_patterns_dates.into_iter() {
            let days_pattern = self
                .days_patterns
                .get_from_dates(dates.iter(), &self.calendar);

            for (offset, timezone_days_pattern) in self.timezones_patterns.fetch_or_insert(
                timezone,
                &mut self.days_patterns,
                &self.calendar,
            ) {
                let offset_days_pattern = self
                    .days_patterns
                    .get_intersection(days_pattern, *timezone_days_pattern);

                let vehicle_data = VehicleData {
                    days_pattern: offset_days_pattern,
                    vehicle_journey_idx: vehicle_journey_idx.clone(),
                    real_time_validity: real_time_validity.clone(),
                };

                let apply_offset = |time_in_timezoned_day: SecondsSinceTimezonedDayStart| -> SecondsSinceUTCDayStart {
                    time_in_timezoned_day.to_utc(offset)
                };

                let insert_result = self.timetables.insert(
                    stops.clone(),
                    flows.clone(),
                    board_times.clone().map(apply_offset),
                    debark_times.clone().map(apply_offset),
                    loads.iter().cloned(),
                    (),
                    vehicle_data,
                );
                match insert_result {
                    Ok(mission) => {
                        if !missions.contains(&mission) {
                            missions.push(mission.clone());
                        };
                    }
                    Err(times_error) => {
                        let dates = self
                            .days_patterns
                            .make_dates(&offset_days_pattern, &self.calendar);
                        let error =
                            InsertionError::Times(vehicle_journey_idx.clone(), times_error, dates);
                        insertion_errors.push(error);
                    }
                }
            }
        }
        (missions, insertion_errors)
    }

    fn do_remove(
        &mut self,
        timetable : &Timetable,
        day: &DaysSinceDatasetStart,
        vehicle_journey_idx: &VehicleJourneyIdx,
        real_time_validity : &RealTimeValidity,
    ) {


        let timetable_data = self.timetables.timetable_data_mut(&timetable);
        let days_patterns = &mut self.days_patterns;

       
        let nb_vehicle_updated = timetable_data.update_vehicles_data(|vehicle_data| {
            if vehicle_data.vehicle_journey_idx == *vehicle_journey_idx
                && vehicle_data.real_time_validity == *real_time_validity
                && days_patterns.is_allowed(&vehicle_data.days_pattern, &day)
            {
                vehicle_data.days_pattern = days_patterns
                    .get_pattern_without_day(vehicle_data.days_pattern, &day)
                    .unwrap(); // unwrap is safe, because we check above that
                               // days_patterns.is_allowed(&vehicle_data.days_pattern, &day)
                true
            } else {
                false
            }
        });
        if nb_vehicle_updated != 1 {
            error!("Updated {} vehicle during removal of one (vehicle_journey_idx, real_time_validity, day).", nb_vehicle_updated);
        }

        // by removing a day from the day_pattern, the day_pattern may have become empty
        // in this case, we remove all vehicle with an empty day_pattern
        let nb_vehicle_removed = timetable_data.remove_vehicles(|vehicle_data| {
            days_patterns.is_empty_pattern(&vehicle_data.days_pattern)
        });

        if nb_vehicle_removed > 1 {
            error!("Removed {} vehicle during removal of one (vehicle_journey_idx, real_time_validity, day).", nb_vehicle_removed);
        }

        
    }
}

impl<'a> TimetablesIter<'a> for PeriodicSplitVjByTzTimetables {
    type Positions = super::iters::PositionsIter;

    fn positions(&'a self, mission: &Self::Mission) -> Self::Positions {
        self.timetables.positions(mission)
    }

    type Trips = TripsIter<'a>;

    fn trips_of(&'a self, mission: &Self::Mission) -> Self::Trips {
        TripsIter::new(self, mission)
    }

    type Missions = super::iters::TimetableIter;

    fn missions(&'a self) -> Self::Missions {
        self.timetables.timetables()
    }
}

pub struct TripsIter<'a> {
    periodic: &'a PeriodicSplitVjByTzTimetables,
    current_vehicle_days: Option<(Vehicle, DaysInPatternIter<'a>)>,
    vehicles_iter: super::iters::VehicleIter,
}

impl<'a> TripsIter<'a> {
    fn new(
        periodic: &'a PeriodicSplitVjByTzTimetables,
        timetable: &super::generic_timetables::Timetable,
    ) -> Self {
        let mut vehicles_iter = periodic.timetables.vehicles(timetable);
        let has_current_vehicle = vehicles_iter.next();
        let current_vehicle_days = has_current_vehicle.map(|vehicle| {
            let days_pattern = periodic.timetables.vehicle_data(&vehicle).days_pattern;
            let days_iter = periodic.days_patterns.days_in_pattern(&days_pattern);
            (vehicle, days_iter)
        });

        Self {
            periodic,
            current_vehicle_days,
            vehicles_iter,
        }
    }
}

impl<'a> Iterator for TripsIter<'a> {
    type Item = Trip;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((vehicle, days_iter)) = &mut self.current_vehicle_days {
                match days_iter.next() {
                    Some(day) => {
                        let trip = Trip {
                            vehicle: vehicle.clone(),
                            day,
                        };
                        return Some(trip);
                    }
                    None => {
                        let has_current_vehicle = self.vehicles_iter.next();
                        self.current_vehicle_days = has_current_vehicle.map(|vehicle| {
                            let days_pattern =
                                self.periodic.timetables.vehicle_data(&vehicle).days_pattern;
                            let days_iter =
                                self.periodic.days_patterns.days_in_pattern(&days_pattern);
                            (vehicle, days_iter)
                        });
                    }
                }
            } else {
                return None;
            }
        }
    }
}
