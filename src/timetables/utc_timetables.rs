// Copyright  (C) 2020, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Hove (www.kisio.com).
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
    time::{
        calendar::DecomposeUTCResult,
        days_patterns::{DaysInPatternIter, DaysPattern, DaysPatterns},
    },
    timetables::generic_timetables::{inspect, VehicleTimesError},
    transit_data::Stop,
    RealTimeLevel,
};

use super::{
    day_to_timetable::LocalZone,
    generic_timetables::{GenericTimetables, Vehicle},
    timetable_iters::{PositionsIter, TimetableIter},
};
use crate::time::{
    Calendar, DaysSinceDatasetStart, SecondsSinceDatasetUTCStart, SecondsSinceTimezonedDayStart,
    SecondsSinceUTCDayStart, TimezonesPatterns,
};
use chrono::NaiveDate;
use std::collections::{BTreeMap, HashMap};
use tracing::log::error;

use crate::timetables::FlowDirection;

pub use super::generic_timetables::{Position, Timetable as Mission, Trip};

pub struct UTCTimetables {
    timetables: GenericTimetables<SecondsSinceUTCDayStart, Load, VehicleData>,
    timezones_patterns: TimezonesPatterns,
}

#[derive(Debug, Clone)]
pub struct VehicleData {
    vehicle_journey_idx: VehicleJourneyIdx,
    base_days_pattern: DaysPattern,
    real_time_days_pattern: DaysPattern,
    local_zone: LocalZone,
}

impl UTCTimetables {
    pub fn new() -> Self {
        Self {
            timetables: GenericTimetables::new(),
            timezones_patterns: TimezonesPatterns::new(),
        }
    }

    pub fn nb_of_missions(&self) -> usize {
        self.timetables.nb_of_timetables()
    }

    pub fn mission_id(&self, mission: &Mission) -> usize {
        mission.idx
    }

    pub fn vehicle_journey_idx(&self, trip: &Trip) -> VehicleJourneyIdx {
        self.timetables
            .vehicle_data(&trip.vehicle)
            .vehicle_journey_idx
            .clone()
    }

    pub fn stoptime_idx(&self, position: &Position, _trip: &Trip) -> usize {
        self.timetables.stoptime_idx(position)
    }

    pub fn day_of(&self, trip: &Trip) -> DaysSinceDatasetStart {
        trip.day
    }

    pub fn mission_of(&self, trip: &Trip) -> Mission {
        self.timetables.timetable_of(&trip.vehicle)
    }

    pub fn stop_at(&self, position: &Position, mission: &Mission) -> Stop {
        *self.timetables.stop_at(position, mission)
    }

    pub fn nb_of_trips(&self) -> usize {
        self.timetables.nb_of_trips()
    }

    pub fn is_upstream_in_mission(
        &self,
        upstream: &Position,
        downstream: &Position,
        mission: &Mission,
    ) -> bool {
        self.timetables.is_upstream(upstream, downstream, mission)
    }

    pub fn first_position(&self, mission: &Mission) -> Position {
        self.timetables.first_position(mission)
    }

    pub fn last_position(&self, mission: &Mission) -> Position {
        self.timetables.last_position(mission)
    }

    pub fn next_position(&self, position: &Position, mission: &Mission) -> Option<Position> {
        self.timetables.next_position(position, mission)
    }

    pub fn previous_position(&self, position: &Position, mission: &Mission) -> Option<Position> {
        self.timetables.previous_position(position, mission)
    }

    pub fn arrival_time_of(
        &self,
        trip: &Trip,
        position: &Position,
        calendar: &Calendar,
    ) -> SecondsSinceDatasetUTCStart {
        let time_in_day = self.timetables.arrival_time(&trip.vehicle, position);
        calendar.compose_utc(&trip.day, time_in_day)
    }

    pub fn load_before(&self, trip: &Trip, position: &Position) -> Load {
        *self.timetables.load_before(&trip.vehicle, position)
    }

    pub fn departure_time_of(
        &self,
        trip: &Trip,
        position: &Position,
        calendar: &Calendar,
    ) -> SecondsSinceDatasetUTCStart {
        let time_in_day = self.timetables.departure_time(&trip.vehicle, position);
        calendar.compose_utc(&trip.day, time_in_day)
    }

    pub fn load_after(&self, trip: &Trip, position: &Position) -> Load {
        *self.timetables.load_after(&trip.vehicle, position)
    }

    pub fn debark_time_of(
        &self,
        trip: &Trip,
        position: &Position,
        calendar: &Calendar,
    ) -> Option<SecondsSinceDatasetUTCStart> {
        self.timetables
            .debark_time(&trip.vehicle, position)
            .map(|time_in_day| {
                let day = &trip.day;

                calendar.compose_utc(day, time_in_day)
            })
    }

    pub fn board_time_of(
        &self,
        trip: &Trip,
        position: &Position,
        calendar: &Calendar,
    ) -> Option<SecondsSinceDatasetUTCStart> {
        self.timetables
            .board_time(&trip.vehicle, position)
            .map(|time_in_day| {
                let day = &trip.day;

                calendar.compose_utc(day, time_in_day)
            })
    }

    pub fn earliest_trip_to_board_at(
        &self,
        waiting_time: SecondsSinceDatasetUTCStart,
        mission: &Mission,
        position: &Position,
        real_time_level: RealTimeLevel,
        calendar: &Calendar,
        days_patterns: &DaysPatterns,
    ) -> Option<(Trip, SecondsSinceDatasetUTCStart, Load)> {
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

    pub fn earliest_filtered_trip_to_board_at<Filter>(
        &self,
        waiting_time: SecondsSinceDatasetUTCStart,
        mission: &Mission,
        position: &Position,
        real_time_level: RealTimeLevel,
        filter: Filter,
        calendar: &Calendar,
        days_patterns: &DaysPatterns,
    ) -> Option<(Trip, SecondsSinceDatasetUTCStart, Load)>
    where
        Filter: Fn(&VehicleJourneyIdx) -> bool,
    {
        let decompositions = calendar.decompositions_utc(waiting_time);

        // if there is not next position, we cannot board this mission at this posision
        // TODO : revise this comment when stay_ins are implemented
        let next_position = self.timetables.next_position(position, mission)?;

        let mut best_vehicle_day_and_its_arrival_time_at_next_position: Option<(
            Vehicle,
            DaysSinceDatasetStart,
            SecondsSinceDatasetUTCStart,
            Load,
        )> = None;

        for (waiting_day, waiting_time_in_day) in decompositions {
            let has_vehicle = self.timetables.earliest_vehicle_to_board(
                &waiting_time_in_day,
                mission,
                position,
                |vehicle_data| {
                    let days_pattern = match real_time_level {
                        RealTimeLevel::Base => vehicle_data.base_days_pattern,
                        RealTimeLevel::RealTime => vehicle_data.real_time_days_pattern,
                    };
                    days_patterns.is_allowed(&days_pattern, waiting_day)
                        && filter(&vehicle_data.vehicle_journey_idx)
                },
            );
            if let Some(vehicle) = has_vehicle {
                let arrival_time_in_day_at_next_stop =
                    self.timetables.arrival_time(&vehicle, &next_position);
                let load = self.timetables.load_before(&vehicle, &next_position);
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

    pub fn latest_trip_that_debark_at(
        &self,
        time: SecondsSinceDatasetUTCStart,
        mission: &Mission,
        position: &Position,
        real_time_level: RealTimeLevel,
        calendar: &Calendar,
        days_patterns: &DaysPatterns,
    ) -> Option<(Trip, SecondsSinceDatasetUTCStart, Load)> {
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

    pub fn latest_filtered_trip_that_debark_at<Filter>(
        &self,
        time: SecondsSinceDatasetUTCStart,
        mission: &Mission,
        position: &Position,
        real_time_level: RealTimeLevel,
        filter: Filter,
        calendar: &Calendar,
        days_patterns: &DaysPatterns,
    ) -> Option<(Trip, SecondsSinceDatasetUTCStart, Load)>
    where
        Filter: Fn(&VehicleJourneyIdx) -> bool,
    {
        // if there is not prev position, we cannot debark this mission at this posision
        // TODO : revise this comment when stay_ins are implemented
        let prev_position = self.timetables.previous_position(position, mission)?;
        let decompositions = calendar.decompositions_utc(time);
        let mut best_vehicle_day_and_its_departure_time_at_previous_position: Option<(
            Vehicle,
            DaysSinceDatasetStart,
            SecondsSinceDatasetUTCStart,
            Load,
        )> = None;
        for (waiting_day, waiting_time_in_day) in decompositions {
            let has_vehicle = self.timetables.latest_vehicle_that_debark(
                &waiting_time_in_day,
                mission,
                position,
                |vehicle_data| {
                    let days_pattern = match real_time_level {
                        RealTimeLevel::Base => vehicle_data.base_days_pattern,
                        RealTimeLevel::RealTime => vehicle_data.real_time_days_pattern,
                    };
                    days_patterns.is_allowed(&days_pattern, waiting_day)
                        && filter(&vehicle_data.vehicle_journey_idx)
                },
            );
            if let Some(vehicle) = has_vehicle {
                let departure_time_in_day_at_previous_stop =
                    self.timetables.departure_time(&vehicle, &prev_position);
                let departure_time_at_previous_stop =
                    calendar.compose_utc(&waiting_day, departure_time_in_day_at_previous_stop);

                let load = self.timetables.load_before(&vehicle, &position);
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

    pub fn insert<Stops, Flows, BoardTimes, DebarkTimes>(
        &mut self,
        stops: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads_data: &LoadsData,
        days: &DaysPattern,
        calendar: &Calendar,
        days_patterns: &mut DaysPatterns,
        timezone: chrono_tz::Tz,
        vehicle_journey_idx: &VehicleJourneyIdx,
        local_zone: LocalZone,
        real_time_level: RealTimeLevel,
    ) -> Result<HashMap<Mission, DaysPattern>, (VehicleTimesError, Vec<NaiveDate>)>
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

                if days_patterns.is_empty_pattern(&days_pattern) {
                    continue;
                }

                let apply_offset = |time_in_timezoned_day: SecondsSinceTimezonedDayStart| -> SecondsSinceUTCDayStart {
                    time_in_timezoned_day.to_utc(offset)
                };

                let board_times = board_times.clone().map(apply_offset);
                let debark_times = debark_times.clone().map(apply_offset);

                let inspect_result =
                    inspect(flows.clone(), board_times.clone(), debark_times.clone());
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

                if days_patterns.is_empty_pattern(&days_pattern) {
                    continue;
                }

                let (base_days_pattern, real_time_days_pattern) = match real_time_level {
                    RealTimeLevel::Base => (days_pattern, days_pattern),
                    RealTimeLevel::RealTime => (days_patterns.empty_pattern(), days_pattern),
                };

                let vehicle_data = VehicleData {
                    vehicle_journey_idx: vehicle_journey_idx.clone(),
                    base_days_pattern,
                    real_time_days_pattern,
                    local_zone,
                };

                let apply_offset = |time_in_timezoned_day: SecondsSinceTimezonedDayStart| -> SecondsSinceUTCDayStart {
                    time_in_timezoned_day.to_utc(offset)
                };

                let insert_result = self.timetables.insert(
                    stops.clone(),
                    flows.clone(),
                    board_times.clone().map(apply_offset),
                    debark_times.clone().map(apply_offset),
                    loads.iter().copied(),
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
                            "An error occured while inserting a vehicle. {:?}",
                            times_error
                        );
                    }
                }
            }
        }
        Ok(result)
    }

    pub fn find_trip(
        &self,
        timetable: &Mission,
        day: DaysSinceDatasetStart,
        vehicle_journey_idx: &VehicleJourneyIdx,
        local_zone: LocalZone,
        real_time_level: RealTimeLevel,
        days_patterns: &DaysPatterns,
    ) -> Option<Trip> {
        let timetable_data = self.timetables.timetable_data(timetable);

        let idx = timetable_data.find_vehicles(|vehicle_data: &VehicleData| {
            let days_pattern = match real_time_level {
                RealTimeLevel::Base => &vehicle_data.base_days_pattern,
                RealTimeLevel::RealTime => &vehicle_data.real_time_days_pattern,
            };
            vehicle_data.vehicle_journey_idx == *vehicle_journey_idx
                && vehicle_data.local_zone == local_zone
                && days_patterns.is_allowed(days_pattern, day)
        })?;
        let vehicle = Vehicle {
            timetable: timetable.clone(),
            idx,
        };
        Some(Trip { vehicle, day })
    }

    pub fn remove(
        &mut self,
        timetable: &Mission,
        day: DaysSinceDatasetStart,
        vehicle_journey_idx: &VehicleJourneyIdx,
        local_zone: LocalZone,
        real_time_level: RealTimeLevel,
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
                    && vehicle_data.local_zone == local_zone
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

    pub fn positions(&self, mission: &Mission) -> PositionsIter {
        self.timetables.positions(mission)
    }

    pub fn trips_of<'a>(
        &'a self,
        mission: &Mission,
        real_time_level: RealTimeLevel,
        days_patterns: &'a DaysPatterns,
    ) -> TripsIter<'a> {
        TripsIter::new(self, mission, real_time_level, days_patterns)
    }

    pub fn missions(&self) -> TimetableIter {
        self.timetables.timetables()
    }

    pub fn trips_boardable_between<'a>(
        &'a self,
        from_time: SecondsSinceDatasetUTCStart,
        until_time: SecondsSinceDatasetUTCStart,
        mission: &Mission,
        position: &Position,
        real_time_level: RealTimeLevel,
        days_patterns: &'a DaysPatterns,
        calendar: &'a Calendar,
    ) -> TripsBoardableBetween<'a> {
        debug_assert!(position.timetable == *mission);

        TripsBoardableBetween::new(
            self,
            real_time_level,
            days_patterns,
            calendar,
            mission.clone(),
            position.idx,
            from_time,
            until_time,
        )
    }

    pub fn trips_debarkable_between<'a>(
        &'a self,
        from_time: SecondsSinceDatasetUTCStart,
        until_time: SecondsSinceDatasetUTCStart,
        mission: &Mission,
        position: &Position,
        real_time_level: RealTimeLevel,
        days_patterns: &'a DaysPatterns,
        calendar: &'a Calendar,
    ) -> TripsDebarkableBetween<'a> {
        debug_assert!(position.timetable == *mission);

        TripsDebarkableBetween::new(
            self,
            real_time_level,
            days_patterns,
            calendar,
            mission.clone(),
            position.idx,
            from_time,
            until_time,
        )
    }
}

pub struct TripsIter<'a> {
    utc_timetables: &'a UTCTimetables,
    current_vehicle_days: Option<(Vehicle, DaysInPatternIter<'a>)>,
    vehicles_iter: super::timetable_iters::VehicleIter,
    real_time_level: RealTimeLevel,
    days_patterns: &'a DaysPatterns,
}

impl<'a> TripsIter<'a> {
    fn new(
        utc_timetables: &'a UTCTimetables,
        timetable: &super::generic_timetables::Timetable,
        real_time_level: RealTimeLevel,
        days_patterns: &'a DaysPatterns,
    ) -> Self {
        let mut vehicles_iter = utc_timetables.timetables.vehicles(timetable);
        let has_current_vehicle = vehicles_iter.next();
        let current_vehicle_days = has_current_vehicle.map(|vehicle| {
            let days_pattern = match real_time_level {
                RealTimeLevel::Base => {
                    utc_timetables
                        .timetables
                        .vehicle_data(&vehicle)
                        .base_days_pattern
                }
                RealTimeLevel::RealTime => {
                    utc_timetables
                        .timetables
                        .vehicle_data(&vehicle)
                        .real_time_days_pattern
                }
            };
            let days_iter = days_patterns.days_in_pattern(&days_pattern);
            (vehicle, days_iter)
        });

        Self {
            utc_timetables,
            current_vehicle_days,
            vehicles_iter,
            real_time_level,
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
                                    self.utc_timetables
                                        .timetables
                                        .vehicle_data(&vehicle)
                                        .base_days_pattern
                                }
                                RealTimeLevel::RealTime => {
                                    self.utc_timetables
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

pub type TripsBoardableBetween<'a> = TripsBetween<'a, true>;
pub type TripsDebarkableBetween<'a> = TripsBetween<'a, false>;
pub struct TripsBetween<'a, const BOARD_TIMES: bool> {
    // first iterate on days, and then iterate on NextBoardableVehicle on this day
    utc_timetables: &'a UTCTimetables,
    real_time_level: RealTimeLevel,
    days_patterns: &'a DaysPatterns,
    calendar: &'a Calendar,
    mission: Mission,
    position_idx: usize,
    from_time: SecondsSinceDatasetUTCStart,
    until_time: SecondsSinceDatasetUTCStart,

    // the iterator is exhausted when current_day.is_none()
    current_day: Option<DaysSinceDatasetStart>,
    current_vehicle_idx: usize,
    current_until_time_in_day: SecondsSinceUTCDayStart,
}

impl<'a, const BOARD_TIMES: bool> TripsBetween<'a, BOARD_TIMES> {
    fn new(
        utc_timetables: &'a UTCTimetables,
        real_time_level: RealTimeLevel,
        days_patterns: &'a DaysPatterns,
        calendar: &'a Calendar,
        mission: Mission,
        position_idx: usize,
        from_time: SecondsSinceDatasetUTCStart,
        until_time: SecondsSinceDatasetUTCStart,
    ) -> Self {
        let timetable_data = utc_timetables.timetables.timetable_data(&mission);
        let nb_of_vehicle = timetable_data.nb_of_vehicle();

        let empty_iterator = Self {
            utc_timetables,
            real_time_level,
            days_patterns,
            calendar,
            mission: mission.clone(),
            position_idx,
            from_time,
            until_time,
            current_day: None,
            current_vehicle_idx: nb_of_vehicle,
            current_until_time_in_day: SecondsSinceUTCDayStart::min(),
        };

        if until_time < from_time {
            // empty_iterator
            return empty_iterator;
        }

        let has_first_day = calendar
            .decompositions_utc(from_time)
            .min_by(|(day_a, _), (day_b, _)| day_a.days.cmp(&day_b.days));
        if let Some((from_day, from_time_in_day)) = has_first_day {
            // find first vehicle that depart after from_time_in_day
            let current_vehicle_idx = if BOARD_TIMES {
                timetable_data
                    .earliest_vehicle_to_board(&from_time_in_day, position_idx, |_| true)
                    .unwrap_or_else(|| timetable_data.nb_of_vehicle())
            } else {
                timetable_data
                    .earliest_vehicle_that_debark(&from_time_in_day, position_idx, |_| true)
                    .unwrap_or_else(|| timetable_data.nb_of_vehicle())
            };

            let until_time_in_day = match calendar.decompose_utc(until_time, from_day) {
                DecomposeUTCResult::BelowMin => {
                    // until_time_in_day < SecondsSinceUTCDayStart::min()
                    // so there will be no trip departing  after until_time
                    // on from_day and for all days after from_day,
                    // so the iterator is empty
                    return empty_iterator;
                }
                DecomposeUTCResult::Success(time_in_day) => time_in_day,
                DecomposeUTCResult::AboveMax => SecondsSinceUTCDayStart::max(),
            };

            Self {
                utc_timetables,
                real_time_level,
                days_patterns,
                calendar,
                mission,
                position_idx,
                from_time,
                until_time,
                current_day: Some(from_day),
                current_vehicle_idx,
                current_until_time_in_day: until_time_in_day,
            }
        } else {
            // empty_iterator
            empty_iterator
        }
    }
}

impl<'a, const BOARD_TIMES: bool> Iterator for TripsBetween<'a, BOARD_TIMES> {
    type Item = Trip;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // if self.current_day.is_none()
            // it means we have exhausted the iterator
            let day = self.current_day?;

            let timetable_data = self.utc_timetables.timetables.timetable_data(&self.mission);
            let nb_of_vehicle = timetable_data.nb_of_vehicle();
            if self.current_vehicle_idx < nb_of_vehicle {
                let vehicle_idx = self.current_vehicle_idx;
                self.current_vehicle_idx += 1;
                let time = if BOARD_TIMES {
                    &timetable_data.board_times_by_position[self.position_idx][vehicle_idx]
                } else {
                    &timetable_data.debark_times_by_position[self.position_idx][vehicle_idx]
                };

                if *time > self.current_until_time_in_day {
                    // since the vehicle are ordered by increasing board times
                    // it means that all subsequent vehicle will have a board_time > self.current_until_time_in_day
                    // So here we finished exploring vehicles on self.current_day
                    // let's loop and increase the day on the next loop iteration
                    self.current_vehicle_idx = nb_of_vehicle;
                    continue;
                } else {
                    let vehicle_data = timetable_data.vehicle_data(vehicle_idx);
                    let days_pattern = match self.real_time_level {
                        RealTimeLevel::Base => vehicle_data.base_days_pattern,
                        RealTimeLevel::RealTime => vehicle_data.real_time_days_pattern,
                    };
                    if !self.days_patterns.is_allowed(&days_pattern, day) {
                        continue;
                    }
                    return Some(Trip {
                        vehicle: Vehicle {
                            timetable: self.mission.clone(),
                            idx: vehicle_idx,
                        },
                        day,
                    });
                }
            }
            // here we have no more vehicle to explore on self.current_date
            // so let's increase the date
            else {
                // increase self.current_day
                self.current_day = self.calendar.next_day(day);

                if let Some(new_day) = self.current_day {
                    // decompose from_time and until_time wrt self.current_day
                    let from_time_in_day =
                        match self.calendar.decompose_utc(self.from_time, new_day) {
                            DecomposeUTCResult::BelowMin => SecondsSinceUTCDayStart::min(),
                            DecomposeUTCResult::Success(time_in_day) => time_in_day,
                            DecomposeUTCResult::AboveMax => SecondsSinceUTCDayStart::max(),
                        };

                    let until_time_in_day =
                        match self.calendar.decompose_utc(self.until_time, new_day) {
                            DecomposeUTCResult::BelowMin => {
                                // until_time_in_day < SecondsSinceUTCDayStart::min()
                                // so there will be no trip departing  after self.until_time
                                // on new_day and for all days after new_day,
                                // so the iterator is finished
                                self.current_day = None;
                                return None;
                            }
                            DecomposeUTCResult::Success(time_in_day) => time_in_day,
                            DecomposeUTCResult::AboveMax => SecondsSinceUTCDayStart::max(),
                        };

                    // find first vehicle that depart after from_time_in_day
                    if BOARD_TIMES {
                        self.current_vehicle_idx = timetable_data
                            .earliest_vehicle_to_board(&from_time_in_day, self.position_idx, |_| {
                                true
                            })
                            .unwrap_or_else(|| timetable_data.nb_of_vehicle());
                    } else {
                        self.current_vehicle_idx = timetable_data
                            .earliest_vehicle_that_debark(
                                &from_time_in_day,
                                self.position_idx,
                                |_| true,
                            )
                            .unwrap_or_else(|| timetable_data.nb_of_vehicle());
                    }

                    self.current_until_time_in_day = until_time_in_day;
                }
            }
        }
    }
}
