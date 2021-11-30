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

use tracing::log::error;

use crate::{
    loads_data::LoadsData,
    models::{StopPointIdx, VehicleJourneyIdx},
    timetables::{day_to_timetable::Unknown, InsertionError, ModifyError, RemovalError},
    transit_data::TransitData,
};

use crate::{
    time::SecondsSinceTimezonedDayStart,
    timetables::{FlowDirection, Timetables as TimetablesTrait, TimetablesIter},
};

use super::data_interface::{self, RealTimeLevel};

impl<Timetables> data_interface::DataUpdate for TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a>,
{
    fn remove_real_time_vehicle(
        &mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &chrono::NaiveDate,
    ) -> Result<(), RemovalError> {
        let day = self
            .calendar
            .date_to_days_since_start(date)
            .ok_or_else(|| RemovalError::UnknownDate(*date, vehicle_journey_idx.clone()))?;

        // We get the timetable, and then remove `date` from its real_time days_pattern
        let timetable = self
            .vehicle_journey_to_timetable
            .remove_real_time_vehicle(vehicle_journey_idx, &day, &mut self.days_patterns)
            .map_err(|err| match err {
                Unknown::VehicleJourneyIdx => {
                    RemovalError::UnknownVehicleJourney(vehicle_journey_idx.clone())
                }
                Unknown::DayForVehicleJourney => {
                    RemovalError::DateInvalidForVehicleJourney(*date, vehicle_journey_idx.clone())
                }
            })?;

        self.timetables.remove(
            &timetable,
            &day,
            vehicle_journey_idx,
            &RealTimeLevel::RealTime,
            &self.calendar,
            &mut self.days_patterns,
        );

        Ok(())
    }

    fn insert_real_time_vehicle<'date, Stops, Flows, Dates, BoardTimes, DebarkTimes>(
        &mut self,
        stop_points: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads_data: &LoadsData,
        valid_dates: Dates,
        timezone: &chrono_tz::Tz,
        vehicle_journey_idx: VehicleJourneyIdx,
    ) -> Result<(), InsertionError>
    where
        Stops: Iterator<Item = StopPointIdx> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        Dates: Iterator<Item = &'date chrono::NaiveDate> + Clone,
        BoardTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
    {
        self.insert_inner(
            stop_points,
            flows,
            board_times,
            debark_times,
            loads_data,
            valid_dates,
            timezone,
            vehicle_journey_idx,
            RealTimeLevel::RealTime,
        )
    }

    fn modify_real_time_vehicle<'date, Stops, Flows, Dates, BoardTimes, DebarkTimes>(
        &mut self,
        stops: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads_data: &LoadsData,
        valid_dates: Dates,
        timezone: &chrono_tz::Tz,
        vehicle_journey_idx: &VehicleJourneyIdx,
    ) -> Result<(), ModifyError>
    where
        Stops: Iterator<Item = StopPointIdx> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        Dates: Iterator<Item = &'date chrono::NaiveDate> + Clone,
        BoardTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
    {
        // - Get the real_time_vehicles that already exists on `valid_dates`,
        // - remove `valid_dates` from its real_time days_pattern
        // - insert a new vehicle, valid on `valid_dates` on the real_time level

        for date in valid_dates.clone() {
            let day = self
                .calendar
                .date_to_days_since_start(date)
                .ok_or_else(|| {
                    ModifyError::UnknownDate(date.clone(), vehicle_journey_idx.clone())
                })?;

            if !self.vehicle_journey_to_timetable.real_time_vehicle_exists(
                vehicle_journey_idx,
                &day,
                &self.days_patterns,
            ) {
                return Err(ModifyError::DateInvalidForVehicleJourney(
                    *date,
                    vehicle_journey_idx.clone(),
                ));
            }
        }

        for date in valid_dates.clone() {
            // unwrap is safe, because we checked above
            let day = self.calendar.date_to_days_since_start(date).unwrap();

            let timetable = self
                .vehicle_journey_to_timetable
                .remove_real_time_vehicle(vehicle_journey_idx, &day, &mut self.days_patterns)
                .unwrap(); // unwrap is safe, because we checked above that real_time_vehicle_exists()

            self.timetables.remove(
                &timetable,
                &day,
                vehicle_journey_idx,
                &RealTimeLevel::RealTime,
                &self.calendar,
                &mut self.days_patterns,
            );
        }

        let stops = self.create_stops(stops).into_iter();
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
                loads_data,
                &days,
                &self.calendar,
                &mut self.days_patterns,
                timezone,
                vehicle_journey_idx,
                &RealTimeLevel::RealTime,
            )
            .map_err(|(err, dates)| ModifyError::Times(vehicle_journey_idx.clone(), err, dates))?;

        for (timetable, days_pattern) in timetables.iter() {
            let result = self
                .vehicle_journey_to_timetable
                .insert_real_time_only_vehicle(
                    vehicle_journey_idx,
                    days_pattern,
                    timetable,
                    &mut self.days_patterns,
                );
            if let Err(err) = result {
                // at the beginning of this function, we removed the real_time_vehicle linked to this vehicle_journey_idx
                // in vehicle_journey_to_timetable.
                // So we should not obtain any error while inserting.
                // If this happens, let's just log an error and keep going.
                error!("Error while modifying a real time vehicle : {:?}", err);
            }
        }

        let missions = timetables.keys();

        for mission in missions {
            self.add_mission_to_stops(mission);
        }

        Ok(())
    }
}

impl<Timetables> TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a>,
{
    pub(super) fn insert_inner<'date, Stops, Flows, Dates, BoardTimes, DebarkTimes>(
        &mut self,
        stop_points: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads_data: &LoadsData,
        valid_dates: Dates,
        timezone: &chrono_tz::Tz,
        vehicle_journey_idx: VehicleJourneyIdx,
        real_time_level: RealTimeLevel,
    ) -> Result<(), InsertionError>
    where
        Stops: Iterator<Item = StopPointIdx> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        Dates: Iterator<Item = &'date chrono::NaiveDate> + Clone,
        BoardTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
    {
        // let's check that
        //  - the base vehicle does not exists
        //  - the real time vehicle does not exists
        if self
            .vehicle_journey_to_timetable
            .base_vehicle_exists(&vehicle_journey_idx)
        {
            return Err(InsertionError::BaseVehicleJourneyAlreadyExists(
                vehicle_journey_idx.clone(),
            ));
        }

        for date in valid_dates.clone() {
            let day = self
                .calendar
                .date_to_days_since_start(date)
                .ok_or_else(|| {
                    InsertionError::InvalidDate(date.clone(), vehicle_journey_idx.clone())
                })?;

            if self.vehicle_journey_to_timetable.real_time_vehicle_exists(
                &vehicle_journey_idx,
                &day,
                &self.days_patterns,
            ) {
                return Err(InsertionError::RealTimeVehicleJourneyAlreadyExistsOnDate(
                    *date,
                    vehicle_journey_idx.clone(),
                ));
            }
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
                loads_data,
                &days,
                &self.calendar,
                &mut self.days_patterns,
                timezone,
                &vehicle_journey_idx,
                &real_time_level,
            )
            .map_err(|(err, dates)| {
                InsertionError::Times(vehicle_journey_idx.clone(), err, dates)
            })?;

        for (timetable, days_pattern) in timetables.iter() {
            let result = match real_time_level {
                RealTimeLevel::Base => self
                    .vehicle_journey_to_timetable
                    .insert_base_and_realtime_vehicle(
                        &vehicle_journey_idx,
                        days_pattern,
                        timetable,
                        &mut self.days_patterns,
                    ),
                RealTimeLevel::RealTime => self
                    .vehicle_journey_to_timetable
                    .insert_real_time_only_vehicle(
                        &vehicle_journey_idx,
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
                error!("Error while inserting a real time only vehicle : {:?}", err);
            }
        }

        let missions = timetables.keys();

        for mission in missions {
            self.add_mission_to_stops(mission);
        }

        Ok(())
    }

    pub(super) fn add_mission_to_stops(&mut self, mission: &Timetables::Mission) {
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
