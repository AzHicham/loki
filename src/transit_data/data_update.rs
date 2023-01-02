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

use tracing::error;

use crate::{
    models::{StopPointIdx, VehicleJourneyIdx},
    occupancy_data::OccupancyData,
    robustness::Regularity,
    timetables::{day_to_timetable::LocalZone, InsertionError, ModifyError, RemovalError},
    transit_data::TransitData,
};

use crate::{time::SecondsSinceTimezonedDayStart, timetables::FlowDirection};

use super::{data_interface::RealTimeLevel, Mission};

impl TransitData {
    pub fn remove_real_time_vehicle(
        &mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: chrono::NaiveDate,
    ) -> Result<(), RemovalError> {
        let day = self
            .calendar
            .date_to_days_since_start(date)
            .ok_or_else(|| RemovalError::UnknownDate(date, vehicle_journey_idx.clone()))?;

        let local_zones = self
            .vehicle_journey_to_timetable
            .get_vehicle_local_zones(vehicle_journey_idx);

        for local_zone in local_zones {
            let has_timetable = self.vehicle_journey_to_timetable.remove_real_time_vehicle(
                vehicle_journey_idx,
                local_zone,
                day,
                &mut self.days_patterns,
            );
            let timetable = match has_timetable {
                Err(err) => {
                    error!("Error while removing  real time vehicle {vehicle_journey_idx:?} on {date} on local zone {local_zone:?}. {err:?}");
                    continue;
                }
                Ok(timetable) => timetable,
            };

            self.timetables.remove(
                &timetable,
                day,
                vehicle_journey_idx,
                local_zone,
                RealTimeLevel::RealTime,
                &self.calendar,
                &mut self.days_patterns,
            );
        }

        Ok(())
    }

    pub fn insert_real_time_vehicle<Stops, Flows, Dates, BoardTimes, DebarkTimes>(
        &mut self,
        stop_points: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        occupancy_data: &OccupancyData,
        valid_dates: Dates,
        timezone: chrono_tz::Tz,
        vehicle_journey_idx: VehicleJourneyIdx,
        regularity: Regularity,
    ) -> Result<(), InsertionError>
    where
        Stops: Iterator<Item = StopPointIdx> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        Dates: Iterator<Item = chrono::NaiveDate> + Clone,
        BoardTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
    {
        self.insert_inner(
            stop_points,
            flows,
            board_times,
            debark_times,
            occupancy_data,
            valid_dates,
            timezone,
            vehicle_journey_idx,
            None,
            RealTimeLevel::RealTime,
            regularity,
        )
    }

    pub fn modify_real_time_vehicle<Stops, Flows, Dates, BoardTimes, DebarkTimes>(
        &mut self,
        stops: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        occupancy_data: &OccupancyData,
        valid_dates: Dates,
        timezone: chrono_tz::Tz,
        vehicle_journey_idx: &VehicleJourneyIdx,
        regularity: Regularity,
    ) -> Result<(), ModifyError>
    where
        Stops: Iterator<Item = StopPointIdx> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        Dates: Iterator<Item = chrono::NaiveDate> + Clone,
        BoardTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
    {
        // - Get the real_time_vehicles that already exists on `valid_dates`,
        // - remove `valid_dates` from its real_time days_pattern
        // - insert a new vehicle, valid on `valid_dates` on the real_time level

        // check validity of dates
        for date in valid_dates.clone() {
            self.calendar
                .date_to_days_since_start(date)
                .ok_or_else(|| ModifyError::UnknownDate(date, vehicle_journey_idx.clone()))?;
        }

        let local_zones = self
            .vehicle_journey_to_timetable
            .get_vehicle_local_zones(vehicle_journey_idx);

        for date in valid_dates.clone() {
            // unwrap is safe, because we checked above the validity of all dates
            let day = self.calendar.date_to_days_since_start(date).unwrap();

            for local_zone in local_zones.clone() {
                let has_timetable = self.vehicle_journey_to_timetable.remove_real_time_vehicle(
                    vehicle_journey_idx,
                    local_zone,
                    day,
                    &mut self.days_patterns,
                );

                let timetable = match has_timetable {
                    Ok(timetable) => timetable,
                    Err(err) => {
                        error!("Error while modifying real time vehicle {vehicle_journey_idx:?} on {date} on local zone {local_zone:?}. {err:?}");
                        continue;
                    }
                };

                self.timetables.remove(
                    &timetable,
                    day,
                    vehicle_journey_idx,
                    local_zone,
                    RealTimeLevel::RealTime,
                    &self.calendar,
                    &mut self.days_patterns,
                );
            }
        }

        for local_zone in local_zones {
            let stops = self.create_stops(stops.clone()).into_iter();
            let days = self
                .days_patterns
                .get_from_dates(valid_dates.clone(), &self.calendar);

            let timetables = self.timetables.insert(
                stops,
                flows.clone(),
                board_times.clone(),
                debark_times.clone(),
                occupancy_data,
                &days,
                &self.calendar,
                &mut self.days_patterns,
                timezone,
                vehicle_journey_idx,
                local_zone,
                RealTimeLevel::RealTime,
                regularity,
            );
            let timetables = match timetables {
                Err(err) => {
                    error!("Error while modifying real time vehicle {vehicle_journey_idx:?} on local zone {local_zone:?}. {err:?}");
                    continue;
                }
                Ok(timetables) => timetables,
            };
            for (timetable, days_pattern) in timetables.iter() {
                let result = self
                    .vehicle_journey_to_timetable
                    .insert_real_time_only_vehicle(
                        vehicle_journey_idx,
                        local_zone,
                        days_pattern,
                        timetable,
                        &mut self.days_patterns,
                    );
                if let Err(err) = result {
                    // at the beginning of this function, we removed the real_time_vehicle linked to this vehicle_journey_idx
                    // in vehicle_journey_to_timetable.
                    // So we should not obtain any error while inserting.
                    // If this happens, let's just log an error and keep going.
                    error!("Error while modifying real time vehicle {vehicle_journey_idx:?} on local zone {local_zone:?}. {err:?}");
                }
            }
            let missions = timetables.keys();
            for mission in missions {
                self.add_mission_to_stops(mission);
            }
        }

        Ok(())
    }
}

impl TransitData {
    pub(super) fn insert_inner<Stops, Flows, Dates, BoardTimes, DebarkTimes>(
        &mut self,
        stop_points: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        occupancy_data: &OccupancyData,
        valid_dates: Dates,
        timezone: chrono_tz::Tz,
        vehicle_journey_idx: VehicleJourneyIdx,
        local_zone: LocalZone,
        real_time_level: RealTimeLevel,
        regularity: Regularity,
    ) -> Result<(), InsertionError>
    where
        Stops: Iterator<Item = StopPointIdx> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        Dates: Iterator<Item = chrono::NaiveDate> + Clone,
        BoardTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
    {
        // if we add on the base level, let's check that
        // the vehicle does not exists in base vehicles
        if real_time_level == RealTimeLevel::Base
            && self
                .vehicle_journey_to_timetable
                .base_vehicle_exists(&vehicle_journey_idx, local_zone)
        {
            return Err(InsertionError::BaseVehicleJourneyAlreadyExists(
                vehicle_journey_idx.clone(),
            ));
        }

        // we always check that the real time vehicle does not exists
        for date in valid_dates.clone() {
            let day = self
                .calendar
                .date_to_days_since_start(date)
                .ok_or_else(|| InsertionError::InvalidDate(date, vehicle_journey_idx.clone()))?;

            if self.vehicle_journey_to_timetable.real_time_vehicle_exists(
                &vehicle_journey_idx,
                local_zone,
                day,
                &self.days_patterns,
            ) {
                return Err(InsertionError::RealTimeVehicleJourneyAlreadyExistsOnDate(
                    date,
                    vehicle_journey_idx.clone(),
                ));
            }
        }

        if valid_dates.clone().next().is_none() {
            return Err(InsertionError::NoValidDates(vehicle_journey_idx));
        }

        let stops = self.create_stops(stop_points).into_iter();
        let days = self
            .days_patterns
            .get_from_dates(valid_dates, &self.calendar);

        let timetables = self
            .timetables
            .insert(
                stops,
                flows,
                board_times,
                debark_times,
                occupancy_data,
                &days,
                &self.calendar,
                &mut self.days_patterns,
                timezone,
                &vehicle_journey_idx,
                local_zone,
                real_time_level,
                regularity,
            )
            .map_err(|(err, dates)| {
                InsertionError::Times(vehicle_journey_idx.clone(), real_time_level, err, dates)
            })?;

        for (timetable, days_pattern) in timetables.iter() {
            let result = match real_time_level {
                RealTimeLevel::Base => self
                    .vehicle_journey_to_timetable
                    .insert_base_and_realtime_vehicle(
                        &vehicle_journey_idx,
                        local_zone,
                        days_pattern,
                        timetable,
                        &mut self.days_patterns,
                    ),
                RealTimeLevel::RealTime => self
                    .vehicle_journey_to_timetable
                    .insert_real_time_only_vehicle(
                        &vehicle_journey_idx,
                        local_zone,
                        days_pattern,
                        timetable,
                        &mut self.days_patterns,
                    ),
            };

            if let Err(err) = result {
                // we checked at the beginning of this function that this vehicle_journey_idx has no base/real_time vehicle
                // in vehicle_journey_to_timetable.
                // So we should not obtain any error while inserting.
                // If this happens, let's just log an error and keep going.
                error!("Error while inserting a vehicle. {:?}", err);
            }
        }

        let missions = timetables.keys();

        for mission in missions {
            self.add_mission_to_stops(mission);
        }

        Ok(())
    }

    pub(super) fn add_mission_to_stops(&mut self, mission: &Mission) {
        for position in self.timetables.positions(mission) {
            let stop = self.timetables.stop_at(&position, mission);
            let stop_data = &mut self.stops_data[stop.idx];
            let position_in_timetables = &mut stop_data.position_in_timetables;
            if !position_in_timetables.contains(&(mission.clone(), position.clone())) {
                position_in_timetables.push((mission.clone(), position));
            }
        }
    }
}
