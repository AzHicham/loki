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

use std::collections::HashMap;

use super::{
    generic_timetables::{self, Position, Timetable, Timetables, Trip, VehicleTimesError},
    iters::{PositionsIter, TimetableIter, VehicleIter},
    FlowDirection, Stop, TimetablesIter,
};
use crate::{
    loads_data::{Load, LoadsData},
    models::VehicleJourneyIdx,
    time::{
        days_patterns::{DaysPattern, DaysPatterns},
        Calendar, DaysSinceDatasetStart, SecondsSinceDatasetUTCStart,
        SecondsSinceTimezonedDayStart,
    },
    timetables::{Timetables as TimetablesTrait, Types as TimetablesTypes},
    RealTimeLevel,
};
use chrono::NaiveDate;
use tracing::log::error;

pub type Time = SecondsSinceDatasetUTCStart;

pub struct DailyTimetables {
    timetables: Timetables<Time, Load, (), VehicleData>,
}

#[derive(Clone, Debug)]
pub struct VehicleData {
    vehicle_journey_idx: VehicleJourneyIdx,
    day: DaysSinceDatasetStart,
    is_base: bool,
    is_real_time: bool,
}

impl TimetablesTypes for DailyTimetables {
    type Mission = Timetable;
    type Position = Position;
    type Trip = Trip;
}

impl TimetablesTrait for DailyTimetables {
    fn new() -> Self {
        Self {
            timetables: Timetables::new(),
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
        self.timetables.vehicle_data(&trip.vehicle).day
    }

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission {
        self.timetables.timetable_of(&trip.vehicle)
    }

    fn stop_at(&self, position: &Self::Position, mission: &Self::Mission) -> Stop {
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
        _calendar: &Calendar,
    ) -> (Time, Load) {
        let (time, load) = self.timetables.arrival_time(&trip.vehicle, position);
        (*time, *load)
    }

    fn departure_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
        _calendar: &Calendar,
    ) -> (Time, Load) {
        let (time, load) = self.timetables.departure_time(&trip.vehicle, position);
        (*time, *load)
    }

    fn debark_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
        _calendar: &Calendar,
    ) -> Option<(Time, Load)> {
        self.timetables
            .debark_time(&trip.vehicle, position)
            .map(|(time, load)| (*time, *load))
    }

    fn board_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
        _calendar: &Calendar,
    ) -> Option<(Time, Load)> {
        self.timetables
            .board_time(&trip.vehicle, position)
            .map(|(time, load)| (*time, *load))
    }

    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        real_time_level: &RealTimeLevel,
        calendar: &Calendar,
        days_patterns: &DaysPatterns,
    ) -> Option<(Self::Trip, Time, Load)> {
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
        _calendar: &Calendar,
        _days_patterns: &DaysPatterns,
    ) -> Option<(Self::Trip, Time, Load)>
    where
        Filter: Fn(&VehicleJourneyIdx) -> bool,
    {
        let vehicle_data_filter = |vehicle_data: &VehicleData| {
            let is_valid = match real_time_level {
                RealTimeLevel::Base => vehicle_data.is_base,
                RealTimeLevel::RealTime => vehicle_data.is_real_time,
            };
            is_valid && filter(&vehicle_data.vehicle_journey_idx)
        };

        self.timetables
            .earliest_filtered_vehicle_to_board(
                waiting_time,
                mission,
                position,
                vehicle_data_filter,
            )
            .map(|(vehicle, time, load)| {
                let day = self.timetables.vehicle_data(&vehicle).day;
                let trip = Trip { vehicle, day };
                (trip, *time, *load)
            })
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
        _calendar: &Calendar,
        _days_patterns: &DaysPatterns,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)>
    where
        Filter: Fn(&VehicleJourneyIdx) -> bool,
    {
        let vehicle_data_filter = |vehicle_data: &VehicleData| {
            let is_valid = match real_time_level {
                RealTimeLevel::Base => vehicle_data.is_base,
                RealTimeLevel::RealTime => vehicle_data.is_real_time,
            };
            is_valid && filter(&vehicle_data.vehicle_journey_idx)
        };

        self.timetables
            .latest_filtered_vehicle_that_debark(time, mission, position, vehicle_data_filter)
            .map(|(vehicle, time, load)| {
                let day = self.timetables.vehicle_data(&vehicle).day;
                let trip = Trip { vehicle, day };
                (trip, *time, *load)
            })
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
        let nb_of_positions = stops.len();
        let default_loads = if nb_of_positions > 0 {
            vec![Load::default(); nb_of_positions - 1]
        } else {
            vec![Load::default(); 0]
        };

        for day in days_patterns.days_in_pattern(days) {
            let board_times_utc = board_times
                .clone()
                .map(|time| calendar.compose(&day, &time, timezone));
            let debark_times_utc = debark_times
                .clone()
                .map(|time| calendar.compose(&day, &time, timezone));
            let date = calendar.to_naive_date(&day);
            let inspect_result =
                generic_timetables::inspect(flows.clone(), board_times_utc, debark_times_utc);
            if let Err(err) = inspect_result {
                let dates = vec![date];
                return Err((err, dates));
            }
        }

        let mut result = HashMap::new();

        let days: Vec<_> = days_patterns.days_in_pattern(days).collect();

        for day in days {
            let board_times_utc = board_times
                .clone()
                .map(|time| calendar.compose(&day, &time, timezone));
            let debark_times_utc = debark_times
                .clone()
                .map(|time| calendar.compose(&day, &time, timezone));
            let date = calendar.to_naive_date(&day);
            let loads = loads_data
                .loads(vehicle_journey_idx, &date)
                .unwrap_or_else(|| default_loads.as_slice());

            let (is_base, is_real_time) = match real_time_level {
                RealTimeLevel::Base => (true, true),
                RealTimeLevel::RealTime => (false, true),
            };

            let vehicle_data = VehicleData {
                vehicle_journey_idx: vehicle_journey_idx.clone(),
                day,
                is_base,
                is_real_time,
            };

            let insert_result = self.timetables.insert(
                stops.clone(),
                flows.clone(),
                board_times_utc,
                debark_times_utc,
                loads.iter().cloned(),
                (),
                vehicle_data,
            );

            match insert_result {
                Ok(mission) => {
                    let pattern = result
                        .entry(mission)
                        .or_insert_with(|| days_patterns.empty_pattern());
                    *pattern = days_patterns.get_pattern_with_additional_day(*pattern, &day);
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
        Ok(result)
    }

    fn remove(
        &mut self,
        mission: &Self::Mission,
        day: &DaysSinceDatasetStart,
        vehicle_journey_idx: &VehicleJourneyIdx,
        real_time_level: &RealTimeLevel,
        _calendar: &Calendar,
        _days_patterns: &mut DaysPatterns,
    ) {
        let timetable_data = self.timetables.timetable_data_mut(mission);

        let nb_vehicle_updated = timetable_data.update_vehicles_data(|vehicle_data| {
            let is_valid = match real_time_level {
                RealTimeLevel::Base => vehicle_data.is_base,
                RealTimeLevel::RealTime => vehicle_data.is_real_time,
            };

            if is_valid
                && vehicle_data.day == *day
                && vehicle_data.vehicle_journey_idx == *vehicle_journey_idx
            {
                match real_time_level {
                    RealTimeLevel::Base => vehicle_data.is_base = false,
                    RealTimeLevel::RealTime => vehicle_data.is_real_time = false,
                };
                true
            } else {
                false
            }
        });
        if nb_vehicle_updated != 1 {
            error!("Updated {} vehicle during removal of one (vehicle_journey_idx, real_time_level, day).", nb_vehicle_updated);
        }

        let nb_vehicle_removed = timetable_data
            .remove_vehicles(|vehicle_data| !vehicle_data.is_base && !vehicle_data.is_real_time);
        if nb_vehicle_removed > 1 {
            error!("Removed {} vehicle during removal of one (vehicle_journey_idx, real_time_level, day).", nb_vehicle_removed);
        }
    }
}

impl<'a> TimetablesIter<'a> for DailyTimetables {
    type Positions = PositionsIter;

    fn positions(&'a self, mission: &Self::Mission) -> Self::Positions {
        self.timetables.positions(mission)
    }

    type Trips = TripIter<'a>;

    fn trips_of(
        &'a self,
        mission: &Self::Mission,
        real_time_level: &RealTimeLevel,
        _days_patterns: &'a DaysPatterns,
    ) -> Self::Trips {
        let vehicle_iter = self.timetables.vehicles(mission);
        TripIter {
            vehicle_iter,
            timetables: &self.timetables,
            real_time_level: real_time_level.clone(),
        }
    }

    type Missions = TimetableIter;

    fn missions(&'a self) -> Self::Missions {
        self.timetables.timetables()
    }
}

pub struct TripIter<'a> {
    vehicle_iter: VehicleIter,
    timetables: &'a Timetables<Time, Load, (), VehicleData>,
    real_time_level: RealTimeLevel,
}

impl Iterator for TripIter<'_> {
    type Item = Trip;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.vehicle_iter.next() {
                None => {
                    return None;
                }
                Some(vehicle) => {
                    let vehicle_data = self.timetables.vehicle_data(&vehicle);
                    let is_valid = match self.real_time_level {
                        RealTimeLevel::Base => vehicle_data.is_base,
                        RealTimeLevel::RealTime => vehicle_data.is_real_time,
                    };
                    if is_valid {
                        let day = vehicle_data.day;
                        let trip = Trip { vehicle, day };
                        return Some(trip);
                    }
                }
            }
        }
    }
}
