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
    timetables::generic_timetables,
    RealTimeLevel,
};

use super::{
    generic_timetables::{Timetables, Trip, Vehicle, VehicleTimesError},
    TimetablesIter,
};
use crate::{
    time::{
        Calendar, DaysSinceDatasetStart, SecondsSinceDatasetUTCStart,
        SecondsSinceTimezonedDayStart, SecondsSinceUTCDayStart, TimezonesPatterns,
    },
    timetables::generic_timetables::{Position, Timetable},
};
use chrono::NaiveDate;
use std::collections::{BTreeMap, HashMap};
use tracing::log::error;

use crate::timetables::{
    FlowDirection, Stop, Timetables as TimetablesTrait, Types as TimetablesTypes,
};

pub struct PeriodicSplitVjByTzTimetables {
    timetables: Timetables<SecondsSinceUTCDayStart, Load, (), VehicleData>,
    timezones_patterns: TimezonesPatterns,
}

#[derive(Debug, Clone)]
pub struct VehicleData {
    vehicle_journey_idx: VehicleJourneyIdx,
    base_days_pattern: DaysPattern,
    real_time_days_pattern: DaysPattern,
}

impl TimetablesTypes for PeriodicSplitVjByTzTimetables {
    type Mission = Timetable;
    type Position = Position;
    type Trip = Trip;
}

impl TimetablesTrait for PeriodicSplitVjByTzTimetables {
    fn new() -> Self {
        Self {
            timetables: Timetables::new(),
            timezones_patterns: TimezonesPatterns::new(),
        }
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

    fn day_of(&self, trip: &Self::Trip) -> DaysSinceDatasetStart {
        trip.day
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
        calendar: &Calendar,
    ) -> (SecondsSinceDatasetUTCStart, Load) {
        let (time_in_day, load) = self.timetables.arrival_time(&trip.vehicle, position);
        let time_utc = calendar.compose_utc(&trip.day, time_in_day);
        (time_utc, *load)
    }

    fn departure_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
        calendar: &Calendar,
    ) -> (SecondsSinceDatasetUTCStart, Load) {
        let (time_in_day, load) = self.timetables.departure_time(&trip.vehicle, position);
        let time_utc = calendar.compose_utc(&trip.day, time_in_day);
        (time_utc, *load)
    }

    fn debark_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
        calendar: &Calendar,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)> {
        let has_timeload_in_day = self.timetables.debark_time(&trip.vehicle, position);
        has_timeload_in_day.map(|(time_in_day, load)| {
            let day = &trip.day;
            let time = calendar.compose_utc(day, time_in_day);
            (time, *load)
        })
    }

    fn board_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
        calendar: &Calendar,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)> {
        let has_timeload_in_day = self.timetables.board_time(&trip.vehicle, position);
        has_timeload_in_day.map(|(time_in_day, load)| {
            let day = &trip.day;
            let time = calendar.compose_utc(day, time_in_day);
            (time, *load)
        })
    }

    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        real_time_level: &RealTimeLevel,
        calendar: &Calendar,
        days_patterns: &DaysPatterns,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        self.earliest_filtered_trip_to_board_at(
            waiting_time,
            mission,
            position,
            real_time_level,
            |_| true,
            calendar,
            days_patterns,
        )
    }

    fn earliest_filtered_trip_to_board_at<Filter>(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        real_time_level: &RealTimeLevel,
        filter: Filter,
        calendar: &Calendar,
        days_patterns: &DaysPatterns,
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

        let decompositions = calendar.decompositions_utc(waiting_time);

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
                    let days_pattern = match real_time_level {
                        RealTimeLevel::Base => vehicle_data.base_days_pattern,
                        RealTimeLevel::RealTime => vehicle_data.real_time_days_pattern,
                    };
                    days_patterns.is_allowed(&days_pattern, &waiting_day)
                        && filter(&vehicle_data.vehicle_journey_idx)
                },
            );
            if let Some((vehicle, arrival_time_in_day_at_next_stop, load)) = has_vehicle {
                let arrival_time_at_next_stop =
                    calendar.compose_utc(&waiting_day, arrival_time_in_day_at_next_stop);

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
        calendar: &Calendar,
        days_patterns: &DaysPatterns,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        self.latest_filtered_trip_that_debark_at(
            time,
            mission,
            position,
            real_time_level,
            |_| true,
            calendar,
            days_patterns,
        )
    }

    fn latest_filtered_trip_that_debark_at<Filter>(
        &self,
        time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        real_time_level: &RealTimeLevel,
        filter: Filter,
        calendar: &Calendar,
        days_patterns: &DaysPatterns,
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

        let decompositions = calendar.decompositions_utc(time);
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
                    let days_pattern = match real_time_level {
                        RealTimeLevel::Base => vehicle_data.base_days_pattern,
                        RealTimeLevel::RealTime => vehicle_data.real_time_days_pattern,
                    };
                    days_patterns.is_allowed(&days_pattern, &waiting_day)
                        && filter(&vehicle_data.vehicle_journey_idx)
                },
            );
            if let Some((vehicle, departure_time_in_day_at_previous_stop, load)) = has_vehicle {
                let departure_time_at_previous_stop =
                    calendar.compose_utc(&waiting_day, departure_time_in_day_at_previous_stop);

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

    fn insert<Stops, Flows, BoardTimes, DebarkTimes>(
        &mut self,
        stops: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads_data: &LoadsData,
        days: &DaysPattern,
        calendar: &Calendar,
        days_patterns: &mut DaysPatterns,
        timezone: &chrono_tz::Tz,
        vehicle_journey_idx: &VehicleJourneyIdx,
        real_time_level: &RealTimeLevel,
    ) -> Result<HashMap<Timetable, DaysPattern>, (VehicleTimesError, Vec<NaiveDate>)>
    where
        Stops: Iterator<Item = Stop> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        BoardTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
    {
        let mut load_patterns_dates: BTreeMap<&[Load], Vec<NaiveDate>> = BTreeMap::new();

        let nb_of_positions = stops.len();
        let default_loads = if nb_of_positions > 0 {
            vec![Load::default(); nb_of_positions - 1]
        } else {
            vec![Load::default(); 0]
        };
        for date in days_patterns.make_dates(days, calendar) {
            let loads = loads_data
                .loads(&vehicle_journey_idx.clone(), &date)
                .unwrap_or(default_loads.as_slice());
            load_patterns_dates
                .entry(loads)
                .or_insert_with(Vec::new)
                .push(date);
        }

        for (_loads, dates) in load_patterns_dates.iter() {
            let all_days_pattern = days_patterns.get_from_dates(dates.iter(), calendar);

            for (offset, timezone_days_pattern) in
                self.timezones_patterns
                    .fetch_or_insert(timezone, days_patterns, calendar)
            {
                let days_pattern =
                    days_patterns.get_intersection(all_days_pattern, *timezone_days_pattern);

                let apply_offset = |time_in_timezoned_day: SecondsSinceTimezonedDayStart| -> SecondsSinceUTCDayStart {
                    time_in_timezoned_day.to_utc(offset)
                };

                let board_times = board_times.clone().map(apply_offset);
                let debark_times = debark_times.clone().map(apply_offset);

                let inspect_result = generic_timetables::inspect(
                    flows.clone(),
                    board_times.clone(),
                    debark_times.clone(),
                );
                if let Err(err) = inspect_result {
                    let dates = days_patterns.make_dates(&days_pattern, calendar);
                    return Err((err, dates));
                }
            }
        }

        let mut result = HashMap::new();

        for (loads, dates) in load_patterns_dates.into_iter() {
            let all_days_pattern = days_patterns.get_from_dates(dates.iter(), calendar);

            for (offset, timezone_days_pattern) in
                self.timezones_patterns
                    .fetch_or_insert(timezone, days_patterns, calendar)
            {
                let days_pattern =
                    days_patterns.get_intersection(all_days_pattern, *timezone_days_pattern);

                let (base_days_pattern, real_time_days_pattern) = match real_time_level {
                    RealTimeLevel::Base => (days_pattern, days_pattern),
                    RealTimeLevel::RealTime => (days_patterns.empty_pattern(), days_pattern),
                };

                let vehicle_data = VehicleData {
                    vehicle_journey_idx: vehicle_journey_idx.clone(),
                    base_days_pattern,
                    real_time_days_pattern,
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
                        let pattern = result
                            .entry(mission)
                            .or_insert_with(|| days_patterns.empty_pattern());
                        *pattern = days_patterns.get_union(*pattern, days_pattern);
                    }
                    Err(times_error) => {
                        // this should not happen, since we inspect the times above
                        // an returns early with an error if insertion should fail.
                        // Let's log an error if this happens anyway
                        error!(
                            "An error occured while inserting a vehicle {:?}",
                            times_error
                        )
                    }
                }
            }
        }
        Ok(result)
    }

    fn remove(
        &mut self,
        timetable: &Timetable,
        day: &DaysSinceDatasetStart,
        vehicle_journey_idx: &VehicleJourneyIdx,
        real_time_level: &RealTimeLevel,
        _calendar: &Calendar,
        days_patterns: &mut DaysPatterns,
    ) {
        let timetable_data = self.timetables.timetable_data_mut(timetable);

        let nb_vehicle_updated =
            timetable_data.update_vehicles_data(|vehicle_data: &mut VehicleData| {
                let days_pattern = match real_time_level {
                    RealTimeLevel::Base => &mut vehicle_data.base_days_pattern,
                    RealTimeLevel::RealTime => &mut vehicle_data.real_time_days_pattern,
                };
                if vehicle_data.vehicle_journey_idx == *vehicle_journey_idx
                    && days_patterns.is_allowed(days_pattern, day)
                {
                    *days_pattern = days_patterns
                        .get_pattern_without_day(*days_pattern, day)
                        .unwrap(); // unwrap is safe, because we check above that
                                   // days_patterns.is_allowed(&days_pattern, &day)
                    true
                } else {
                    false
                }
            });

        if nb_vehicle_updated != 1 {
            error!("Updated {} vehicle during removal of one (vehicle_journey_idx, real_time_level, day).", nb_vehicle_updated);
        }

        // by removing a day from the day_pattern, the day_pattern may have become empty
        // in this case, we remove all vehicle with an empty day_pattern
        let nb_vehicle_removed = timetable_data.remove_vehicles(|vehicle_data| {
            days_patterns.is_empty_pattern(&vehicle_data.base_days_pattern)
                && days_patterns.is_empty_pattern(&vehicle_data.real_time_days_pattern)
        });

        if nb_vehicle_removed > 1 {
            error!("Removed {} vehicle during removal of one (vehicle_journey_idx, real_time_level, day).", nb_vehicle_removed);
        }
    }
}

impl<'a> TimetablesIter<'a> for PeriodicSplitVjByTzTimetables {
    type Positions = super::iters::PositionsIter;

    fn positions(&'a self, mission: &Self::Mission) -> Self::Positions {
        self.timetables.positions(mission)
    }

    type Trips = TripsIter<'a>;

    fn trips_of(
        &'a self,
        mission: &Self::Mission,
        real_time_level: &RealTimeLevel,
        days_patterns: &'a DaysPatterns,
    ) -> Self::Trips {
        TripsIter::new(self, mission, real_time_level, days_patterns)
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
    real_time_level: RealTimeLevel,
    days_patterns: &'a DaysPatterns,
}

impl<'a> TripsIter<'a> {
    fn new(
        periodic: &'a PeriodicSplitVjByTzTimetables,
        timetable: &super::generic_timetables::Timetable,
        real_time_level: &RealTimeLevel,
        days_patterns: &'a DaysPatterns,
    ) -> Self {
        let mut vehicles_iter = periodic.timetables.vehicles(timetable);
        let has_current_vehicle = vehicles_iter.next();
        let current_vehicle_days = has_current_vehicle.map(|vehicle| {
            let days_pattern = match real_time_level {
                RealTimeLevel::Base => periodic.timetables.vehicle_data(&vehicle).base_days_pattern,
                RealTimeLevel::RealTime => {
                    periodic
                        .timetables
                        .vehicle_data(&vehicle)
                        .real_time_days_pattern
                }
            };
            let days_iter = days_patterns.days_in_pattern(&days_pattern);
            (vehicle, days_iter)
        });

        Self {
            periodic,
            current_vehicle_days,
            vehicles_iter,
            real_time_level: real_time_level.clone(),
            days_patterns,
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
                            let days_pattern = match self.real_time_level {
                                RealTimeLevel::Base => {
                                    self.periodic
                                        .timetables
                                        .vehicle_data(&vehicle)
                                        .base_days_pattern
                                }
                                RealTimeLevel::RealTime => {
                                    self.periodic
                                        .timetables
                                        .vehicle_data(&vehicle)
                                        .real_time_days_pattern
                                }
                            };
                            let days_iter = self.days_patterns.days_in_pattern(&days_pattern);
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
