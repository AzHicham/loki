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

use crate::{chrono::NaiveDate, RealTimeLevel};

use super::{
    base_model::{BaseModel, BaseStopPointIdx, BaseVehicleJourneyIdx},
    real_time_model::{NewStopPointIdx, NewVehicleJourneyIdx, TripData},
    Coord, Rgb, StopPointIdx, StopTimeIdx, StopTimes, VehicleJourneyIdx,
};

use super::RealTimeModel;


#[derive(Clone)]
pub struct ModelRefs<'model> {
    pub base: &'model BaseModel,
    pub real_time: &'model RealTimeModel,
}

impl<'model> ModelRefs<'model> {
    pub fn new(base: &'model BaseModel, real_time: &'model RealTimeModel) -> Self {
        Self { base, real_time }
    }

    pub fn stop_point_idx(&self, stop_id: &str) -> Option<StopPointIdx> {
        if let Some(base_stop_point_idx) = self.base.stop_point_idx(stop_id) {
            Some(StopPointIdx::Base(base_stop_point_idx))
        } else {
            self.real_time
                .new_stop_id_to_idx
                .get(stop_id)
                .map(|idx| StopPointIdx::New(idx.clone()))
        }
    }

    pub fn stop_point_name<'a>(&'a self, stop_idx: &StopPointIdx) -> &'a str {
        match stop_idx {
            StopPointIdx::Base(idx) => &self.base.stop_point_name(*idx),
            StopPointIdx::New(idx) => &self.real_time.new_stops[idx.idx].name,
        }
    }

    pub fn stop_area_name<'a>(&'a self, stop_idx: &StopPointIdx) -> &'a str {
        match stop_idx {
            StopPointIdx::Base(idx) => &self.base.stop_area_name(*idx),
            StopPointIdx::New(_idx) => "unknown_stop_area",
        }
    }



    pub fn vehicle_journey_name<'a>(&'a self, vehicle_journey_idx: &VehicleJourneyIdx) -> &'a str {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => &self.base.vehicle_journey_name(*idx),
            VehicleJourneyIdx::New(idx) => &self.real_time.new_vehicle_journeys_history[idx.idx].0,
        }
    }

    pub fn line_name<'a>(&'a self, vehicle_journey_idx: &VehicleJourneyIdx) -> &'a str {
        let unknown_line = "unknown_line";
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => self.base.line_name(*idx).unwrap_or(unknown_line),
            VehicleJourneyIdx::New(_idx) => unknown_line,
        }
    }

    pub fn route_name<'a>(&'a self, vehicle_journey_idx: &VehicleJourneyIdx) -> &'a str {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => &self.base.route_name(*idx),
            VehicleJourneyIdx::New(_idx) => "unknown_route",
        }
    }

    pub fn network_name<'a>(&'a self, vehicle_journey_idx: &VehicleJourneyIdx) -> &'a str {
        let unknown_network = "unknown_network";
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                if let Some(name) = self.base.network_name(*idx) {
                    name
                } else {
                    unknown_network
                }
            }
            VehicleJourneyIdx::New(_idx) => unknown_network,
        }
    }

    pub fn physical_mode_name<'a>(&'a self, vehicle_journey_idx: &VehicleJourneyIdx) -> &'a str {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => &self.base.physical_mode_name(*idx),
            VehicleJourneyIdx::New(_idx) => "unknown_physical_mode",
        }
    }

    pub fn commercial_mode_name<'a>(&'a self, vehicle_journey_idx: &VehicleJourneyIdx) -> &'a str {
        let unknown_commercial_mode = "unknown_commercial_mode";
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                if let Some(name) = self.base.commercial_mode_name(*idx) {
                    name
                } else {
                    unknown_commercial_mode
                }
            }
            VehicleJourneyIdx::New(_idx) => unknown_commercial_mode,
        }
    }

    pub fn stop_point_at(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        stop_time_idx: StopTimeIdx,
        date: &NaiveDate,
        real_time_level: &RealTimeLevel,
    ) -> Option<StopPointIdx> {
        match real_time_level {
            &RealTimeLevel::Base => {
                if let VehicleJourneyIdx::Base(idx) = vehicle_journey_idx {
                    self.base
                        .stop_point_at(*idx, stop_time_idx)
                        .map(|idx| StopPointIdx::Base(idx))
                } else {
                    None
                }
            }
            &RealTimeLevel::RealTime => match vehicle_journey_idx {
                VehicleJourneyIdx::Base(idx) => {
                    let has_realtime = self.real_time.base_vehicle_journey_last_version(idx, date);
                    if let Some(trip_data) = has_realtime {
                        match trip_data {
                            TripData::Deleted() => None,
                            TripData::Present(stop_times) => stop_times
                                .get(stop_time_idx.idx)
                                .map(|stop_time| stop_time.stop.clone()),
                        }
                    } else {
                        self.base
                            .stop_point_at(*idx, stop_time_idx)
                            .map(|idx| StopPointIdx::Base(idx))
                    }
                }
                VehicleJourneyIdx::New(idx) => {
                    let has_realtime = self.real_time.new_vehicle_journey_last_version(idx, date);
                    if let Some(trip_data) = has_realtime {
                        match trip_data {
                            TripData::Deleted() => None,
                            TripData::Present(stop_times) => stop_times
                                .get(stop_time_idx.idx)
                                .map(|stop_time| stop_time.stop.clone()),
                        }
                    } else {
                        None
                    }
                }
            },
        }
    }

    pub fn nb_of_new_stops(&self) -> usize {
        self.real_time.new_stops.len()
    }

    pub fn new_stops(&self) -> impl Iterator<Item = NewStopPointIdx> {
        let range = 0..self.nb_of_new_stops();
        range.map(|idx| NewStopPointIdx { idx })
    }

    pub fn nb_of_base_stops(&self) -> usize {
        self.base.nb_of_stop_points()
    }

    pub fn base_stop_points(&self) -> impl Iterator<Item = BaseStopPointIdx> + 'model {
        self.base.stop_points()
    }

    pub fn nb_of_new_vehicle_journeys(&self) -> usize {
        self.real_time.new_vehicle_journeys_history.len()
    }

    pub fn new_vehicle_journeys(&self) -> impl Iterator<Item = NewVehicleJourneyIdx> {
        let range = 0..self.nb_of_new_vehicle_journeys();
        range.map(|idx| NewVehicleJourneyIdx { idx })
    }

    pub fn nb_of_base_vehicle_journeys(&self) -> usize {
        self.base.nb_of_vehicle_journeys()
    }

    pub fn base_vehicle_journeys(&self) -> impl Iterator<Item = BaseVehicleJourneyIdx> + 'model {
        self.base.vehicle_journeys()
    }

    pub fn contains_line_id(&self, id: &str) -> bool {
        self.base.contains_line_id(id)
    }

    pub fn contains_route_id(&self, id: &str) -> bool {
        self.base.contains_route_id(id)
    }

    pub fn contains_network_id(&self, id: &str) -> bool {
        self.base.contains_network_id(id)
    }

    pub fn contains_physical_mode_id(&self, id: &str) -> bool {
        self.base.contains_physical_mode_id(id)
    }

    pub fn contains_commercial_model_id(&self, id: &str) -> bool {
        self.base.contains_commercial_model_id(id)
    }

    pub fn contains_stop_point_id(&self, id: &str) -> bool {
        if self.base.contains_stop_point_id(id) {
            true
        } else {
            self.real_time.new_stop_id_to_idx.contains_key(id)
        }
    }

    pub fn contains_stop_area_id(&self, id: &str) -> bool {
        self.base.contains_stop_area_id(id)
    }
}

impl<'model> ModelRefs<'model> {
    pub fn stop_point_uri(&self, stop_point_idx: &StopPointIdx) -> String {
        match stop_point_idx {
            StopPointIdx::Base(idx) => self.base.stop_point_uri(*idx),
            StopPointIdx::New(idx) => {
                let id = &self.real_time.new_stops[idx.idx].name;
                format!("stop_point:{}", id)
            }
        }
    }

    pub fn house_number(&self, stop_point_idx: &StopPointIdx) -> Option<&'model str> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => self.base.house_number(*idx),
            StopPointIdx::New(_) => None,
        }
    }

    pub fn street_name(&self, stop_point_idx: &StopPointIdx) -> Option<&'model str> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => self.base.street_name(*idx),
            StopPointIdx::New(_) => None,
        }
    }

    pub fn coord(&self, stop_point_idx: &StopPointIdx) -> Option<Coord> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => Some(self.base.coord(*idx)),
            StopPointIdx::New(_) => None,
        }
    }

    pub fn platform_code(&self, stop_point_idx: &StopPointIdx) -> Option<&str> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => self.base.platform_code(*idx),
            StopPointIdx::New(_) => None,
        }
    }

    pub fn fare_zone_id(&self, stop_point_idx: &StopPointIdx) -> Option<&str> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => self.base.fare_zone_id(*idx),
            StopPointIdx::New(_) => None,
        }
    }

    pub fn codes(
        &self,
        stop_point_idx: &StopPointIdx,
    ) -> Option<impl Iterator<Item = &(String, String)> + '_> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => self.base.codes(*idx),
            StopPointIdx::New(_) => None,
        }
    }

    pub fn stop_area_coord(& self, id: &str) -> Option<Coord> {
        self.base.stop_area_coord(id)
    }

    pub fn stop_area_uri(& self, id: &str) -> Option<String> {
        self.base.stop_area_uri(id)
    }

    pub fn stop_area_codes(
        &self,
        id: &str,
    ) -> Option<impl Iterator<Item = &(String, String)> + '_> {
        self.base.stop_area_codes(id)
    }

    pub fn stop_area_timezone(&self, stop_area_id : &str,) -> Option<chrono_tz::Tz> {
        self.base.stop_area_timezone(stop_area_id)
    }


    pub fn timezone(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &NaiveDate,
    ) -> chrono_tz::Tz {
        if let VehicleJourneyIdx::Base(idx) = vehicle_journey_idx {
            if self
                .real_time
                .base_vehicle_journey_last_version(idx, date)
                .is_none()
            {
                return self
                    .base_vehicle_journey_timezone(idx)
                    .unwrap_or(chrono_tz::UTC);
            }
        }
        chrono_tz::UTC
    }

    fn base_vehicle_journey_timezone(&self, idx: &BaseVehicleJourneyIdx) -> Option<chrono_tz::Tz> {
        self.base.timezone(*idx)
    }

    pub fn stop_times(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &NaiveDate,
        from_stoptime_idx: StopTimeIdx,
        to_stoptime_idx: StopTimeIdx,
        real_time_level: &RealTimeLevel,
    ) -> Option<StopTimes> {
        match real_time_level {
            RealTimeLevel::Base => self.stop_times_base_schedule(
                vehicle_journey_idx,
                date,
                from_stoptime_idx,
                to_stoptime_idx,
            ),
            RealTimeLevel::RealTime => self.stop_times_real_time(
                vehicle_journey_idx,
                date,
                from_stoptime_idx,
                to_stoptime_idx,
            ),
        }
    }

    pub fn stop_times_base_schedule(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &NaiveDate,
        from_stoptime_idx: StopTimeIdx,
        to_stoptime_idx: StopTimeIdx,
    ) -> Option<StopTimes> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::New(_) => None,
            VehicleJourneyIdx::Base(idx) => {
                let timezone = self.timezone(vehicle_journey_idx, date);
                self.base
                    .stop_times_partial(*idx, from_stoptime_idx, to_stoptime_idx)
                    .map(|iter| StopTimes::Base(iter, date.clone(), timezone))
                    .ok()
            }
        }
    }

    pub fn stop_times_real_time(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &NaiveDate,
        from_stoptime_idx: StopTimeIdx,
        to_stoptime_idx: StopTimeIdx,
    ) -> Option<StopTimes> {
        match self.real_time.last_version(vehicle_journey_idx, date) {
            Some(TripData::Present(stop_times)) => {
                let range = from_stoptime_idx.idx..=to_stoptime_idx.idx;
                let inner = stop_times[range].iter();
                let iter = StopTimes::New(inner, date.clone());
                Some(iter)
            }
            Some(TripData::Deleted()) => None,
            None => {
                // there is no realtime data for this trip
                // so its base schedule IS the real time schedule
                if let VehicleJourneyIdx::Base(base_idx) = vehicle_journey_idx {
                    let inner = self
                        .base
                        .stop_times_partial(*base_idx, from_stoptime_idx, to_stoptime_idx)
                        .ok()?;
                    let timezone = self.base.timezone(*base_idx)?;
                    let iter = StopTimes::Base(inner, date.clone(), timezone);
                    Some(iter)
                } else {
                    None
                }
            }
        }
    }

    pub fn line_code(&self, vehicle_journey_idx: &VehicleJourneyIdx) -> Option<&str> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => self.base.line_code(*idx),
            VehicleJourneyIdx::New(_) => None,
        }
    }

    pub fn headsign(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &NaiveDate,
        real_time_level: &RealTimeLevel,
    ) -> Option<&str> {
        match (vehicle_journey_idx, real_time_level) {
            (VehicleJourneyIdx::Base(idx), RealTimeLevel::RealTime) => {
                let has_history = self.real_time.base_vehicle_journey_last_version(idx, date);
                if has_history.is_none() {
                    self.base.headsign(*idx)
                } else {
                    None
                }
            }
            (VehicleJourneyIdx::Base(idx), RealTimeLevel::Base) => self.base.headsign(*idx),
            (VehicleJourneyIdx::New(_idx), _) => None,
        }
    }

    pub fn direction(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &NaiveDate,
    ) -> Option<&str> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let has_history = self.real_time.base_vehicle_journey_last_version(idx, date);
                if has_history.is_none() {
                    self.base.direction(*idx)
                } else {
                    None
                }
            }
            VehicleJourneyIdx::New(_idx) => None,
        }
    }

    pub fn line_color(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &NaiveDate,
    ) -> Option<&Rgb> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let has_history = self.real_time.base_vehicle_journey_last_version(idx, date);
                if has_history.is_none() {
                    self.base.line_color(*idx)
                } else {
                    None
                }
            }
            VehicleJourneyIdx::New(_idx) => None,
        }
    }

    pub fn text_color(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &NaiveDate,
    ) -> Option<&transit_model::objects::Rgb> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let has_history = self.real_time.base_vehicle_journey_last_version(idx, date);
                if has_history.is_none() {
                    self.base.text_color(*idx)
                } else {
                    None
                }
            }
            VehicleJourneyIdx::New(_idx) => None,
        }
    }

    pub fn trip_short_name(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &NaiveDate,
    ) -> Option<&str> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let has_history = self.real_time.base_vehicle_journey_last_version(idx, date);
                if has_history.is_none() {
                    self.base.trip_short_name(*idx)
                } else {
                    None
                }
            }
            VehicleJourneyIdx::New(_idx) => None,
        }
    }




}
