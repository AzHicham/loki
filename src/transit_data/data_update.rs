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


use crate::{loads_data::LoadsData, models::{StopPointIdx, VehicleJourneyIdx}, timetables::{InsertionError, ModifyError, RemovalError}, transit_data::TransitData};

use crate::{
    time::SecondsSinceTimezonedDayStart,
    timetables::{FlowDirection, Timetables as TimetablesTrait, TimetablesIter},
};

use super::data_interface::{self, RealTimeLevel};

impl<Timetables> data_interface::DataUpdate for TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> ,
{
    fn remove_real_time_vehicle(
        &mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &chrono::NaiveDate,
    ) -> Result<(), RemovalError> {

    self.timetables
        .remove_real_time_vehicle(date, vehicle_journey_idx)
  
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

       self.insert_inner(stop_points, flows, board_times, debark_times, loads_data, valid_dates, timezone, vehicle_journey_idx, RealTimeLevel::RealTime)

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
        DebarkTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone {
            let stops = self.create_stops(stops).into_iter();
            let missions = self.timetables.modify_real_time_vehicle(stops, flows, board_times, debark_times, loads_data, valid_dates, timezone, vehicle_journey_idx)?;

            for mission in missions.iter() {
                self.add_mission_to_stops(mission);
            }
    
            Ok(())
        }
}


impl<Timetables>  TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> ,
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


        let stops = self.create_stops(stop_points).into_iter();
        let missions= self.timetables.insert_vehicle(
            stops,
            flows,
            board_times,
            debark_times,
            loads_data,
            valid_dates,
            timezone,
            &vehicle_journey_idx,
            &real_time_level,
        )?;

        for mission in missions.iter() {
            self.add_mission_to_stops(mission);
        }

        Ok(())

    }

    pub(super) fn add_mission_to_stops(&mut self, mission : & Timetables::Mission) {
        for position in self.timetables.positions(mission) {
            let stop = self.timetables.stop_at(&position, mission);
            let stop_data = &mut self.stops_data[stop.idx];
            let position_in_timetables = &mut stop_data.position_in_timetables;
            if ! position_in_timetables.contains(&(mission.clone(), position.clone())) {
                position_in_timetables.push((mission.clone(), position));
            }
        }
    }

}
