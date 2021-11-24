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
    timetables::{InsertionError, RealTimeValidity, RemovalError},
    transit_data::TransitData,
};

use crate::{
    time::SecondsSinceTimezonedDayStart,
    timetables::{FlowDirection, Timetables as TimetablesTrait, TimetablesIter},
};

use super::data_interface::{self, RealTimeLevel};

impl<Timetables> data_interface::DataUpdate for TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
{
    fn remove_vehicle(
        &mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &chrono::NaiveDate,
        real_time_level : RealTimeLevel,
    ) -> Result<(), RemovalError> {
        let real_time_validity = match real_time_level {
            RealTimeLevel::Base => RealTimeValidity::BaseAndRealTime,
            RealTimeLevel::RealTime => RealTimeValidity::RealTimeOnly,
        };
        self.timetables
            .remove(date, vehicle_journey_idx, &real_time_validity)

    }

 

    fn add_real_time_vehicle<'date, Stops, Flows, Dates, BoardTimes, DebarkTimes>(
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
        self.add_vehicle_inner(
            stop_points,
            flows,
            board_times,
            debark_times,
            loads_data,
            valid_dates,
            timezone,
            vehicle_journey_idx,
            &RealTimeValidity::RealTimeOnly,
        )
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
        real_time_level: &RealTimeLevel,
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

        for date in valid_dates.clone() {
            let real_time_validity_to_remove = match real_time_level {
                RealTimeLevel::Base => RealTimeValidity::BaseAndRealTime,
                RealTimeLevel::RealTime => RealTimeValidity::RealTimeOnly,
            };
            let removal_result = self.timetables
                    .remove(date, &vehicle_journey_idx, &real_time_validity_to_remove);
            match removal_result {
                Ok(()) => {
                    let errors = self.add_vehicle_inner(
                        stops.clone(),
                        flows.clone(),
                        board_times.clone(),
                        debark_times.clone(),
                        loads_data,
                        valid_dates.clone(),
                        timezone,
                        vehicle_journey_idx.clone(),
                        &RealTimeValidity::RealTimeOnly,
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
}

impl<Timetables> TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
{
    pub(super) fn add_vehicle_inner<'date, Stops, Flows, Dates, BoardTimes, DebarkTimes>(
        &mut self,
        stop_points: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads_data: &LoadsData,
        valid_dates: Dates,
        timezone: &chrono_tz::Tz,
        vehicle_journey_idx: VehicleJourneyIdx,
        real_time_validity: &RealTimeValidity,
    ) -> Vec<InsertionError>
    where
        Stops: Iterator<Item = StopPointIdx> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        Dates: Iterator<Item = &'date chrono::NaiveDate> + Clone,
        BoardTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
    {
        let mut errors = Vec::new();

        let stops = self.create_stops(stop_points).into_iter();
        let (missions, insertion_errors) = self.timetables.insert(
            stops,
            flows,
            board_times,
            debark_times,
            loads_data,
            valid_dates,
            timezone,
            &vehicle_journey_idx,
            real_time_validity,
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


}
