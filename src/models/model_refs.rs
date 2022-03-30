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
    chrono::NaiveDate,
    models::{
        base_model::{
            BaseTransferIdx, EquipmentPropertyKey, PathwayByIter, VehicleJourneyPropertyKey,
            PREFIX_ID_STOP_POINT,
        },
        TransferIdx,
    },
    RealTimeLevel,
};
use transit_model::objects::{
    CommercialMode, Equipment, Line, Network, PhysicalMode, Route, StopArea, VehicleJourney,
};
use typed_index_collection::Idx;

use super::{
    base_model::{BaseModel, BaseStopPointIdx, BaseVehicleJourneyIdx},
    real_time_model::{NewStopPointIdx, NewVehicleJourneyIdx, RealTimeStopTimes, TripVersion},
    Contributor, Coord, Rgb, StopPointIdx, StopTimeIdx, StopTimes, VehicleJourneyIdx,
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
                .map(|idx| StopPointIdx::New(*idx))
        }
    }

    pub fn stop_point_name(&self, stop_idx: &StopPointIdx) -> &str {
        match stop_idx {
            StopPointIdx::Base(idx) => self.base.stop_point_name(*idx),
            StopPointIdx::New(idx) => &self.real_time.new_stops[idx.idx].name,
        }
    }

    pub fn stop_point_id(&self, stop_idx: &StopPointIdx) -> &str {
        match stop_idx {
            StopPointIdx::Base(idx) => self.base.stop_point_id(*idx),
            StopPointIdx::New(idx) => &self.real_time.new_stops[idx.idx].name,
        }
    }

    pub fn stop_area_id(&self, stop_idx: &StopPointIdx) -> &str {
        match stop_idx {
            StopPointIdx::Base(idx) => self.base.stop_area_id(*idx),
            StopPointIdx::New(_idx) => "unknown_stop_area",
        }
    }

    pub fn stop_area_name(&self, stop_idx: &StopPointIdx) -> &str {
        match stop_idx {
            StopPointIdx::Base(idx) => self.base.stop_area_name(*idx),
            StopPointIdx::New(_idx) => "unknown_stop_area",
        }
    }

    pub fn vehicle_journey_name(&self, vehicle_journey_idx: &VehicleJourneyIdx) -> &str {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => self.base.vehicle_journey_name(*idx),
            VehicleJourneyIdx::New(idx) => &self.real_time.new_vehicle_journeys_history[idx.idx].0,
        }
    }

    pub fn line_name(&self, vehicle_journey_idx: &VehicleJourneyIdx) -> &str {
        let unknown_line = "unknown_line";
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => self.base.line_name(*idx).unwrap_or(unknown_line),
            VehicleJourneyIdx::New(_idx) => unknown_line,
        }
    }

    pub fn route_name(&self, vehicle_journey_idx: &VehicleJourneyIdx) -> &str {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => self.base.route_name(*idx),
            VehicleJourneyIdx::New(_idx) => "unknown_route",
        }
    }

    pub fn network_name(&self, vehicle_journey_idx: &VehicleJourneyIdx) -> &str {
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

    pub fn physical_mode_name(&self, vehicle_journey_idx: &VehicleJourneyIdx) -> &str {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => self.base.physical_mode_name(*idx),
            VehicleJourneyIdx::New(_idx) => "unknown_physical_mode",
        }
    }

    pub fn vehicle_journey_property(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        property_key: VehicleJourneyPropertyKey,
    ) -> bool {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => self.base.vehicle_journey_property(*idx, property_key),
            VehicleJourneyIdx::New(_idx) => false,
        }
    }

    pub fn stop_point_property(
        &self,
        stop_point_idx: &StopPointIdx,
        property_key: EquipmentPropertyKey,
    ) -> bool {
        match stop_point_idx {
            StopPointIdx::Base(idx) => self.base.stop_point_property(*idx, property_key),
            StopPointIdx::New(_idx) => false,
        }
    }

    pub fn transfer_property(
        &self,
        transfer_idx: &TransferIdx,
        property_key: EquipmentPropertyKey,
    ) -> bool {
        match transfer_idx {
            TransferIdx::Base(idx) => self.base.transfer_property(*idx, property_key),
            TransferIdx::New(_idx) => false,
        }
    }

    pub fn commercial_mode_name(&self, vehicle_journey_idx: &VehicleJourneyIdx) -> &str {
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

    pub fn co2_emission(&self, vehicle_journey_idx: &VehicleJourneyIdx) -> Option<f32> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => self.base.co2_emission(*idx),
            VehicleJourneyIdx::New(_) => None,
        }
    }

    pub fn stop_point_at(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        stop_time_idx: StopTimeIdx,
        date: NaiveDate,
        real_time_level: RealTimeLevel,
    ) -> Option<StopPointIdx> {
        match real_time_level {
            RealTimeLevel::Base => {
                if let VehicleJourneyIdx::Base(idx) = vehicle_journey_idx {
                    self.base
                        .stop_point_at(*idx, stop_time_idx)
                        .map(StopPointIdx::Base)
                } else {
                    None
                }
            }
            RealTimeLevel::RealTime => match vehicle_journey_idx {
                VehicleJourneyIdx::Base(idx) => {
                    let has_realtime = self.real_time.base_vehicle_journey_last_version(*idx, date);
                    if let Some(trip_data) = has_realtime {
                        match trip_data {
                            TripVersion::Deleted() => None,
                            TripVersion::Present(stop_times) => stop_times
                                .get(stop_time_idx.idx)
                                .map(|stop_time| stop_time.stop.clone()),
                        }
                    } else {
                        self.base
                            .stop_point_at(*idx, stop_time_idx)
                            .map(StopPointIdx::Base)
                    }
                }
                VehicleJourneyIdx::New(idx) => {
                    let has_realtime = self.real_time.new_vehicle_journey_last_version(*idx, date);
                    if let Some(trip_data) = has_realtime {
                        match trip_data {
                            TripVersion::Deleted() => None,
                            TripVersion::Present(stop_times) => stop_times
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

    pub fn nb_of_transfers(&self) -> usize {
        self.base.nb_of_transfers()
    }

    pub fn base_transfers(&self) -> impl Iterator<Item = BaseTransferIdx> + 'model {
        self.base.transfers()
    }

    pub fn nb_of_new_vehicle_journeys(&self) -> usize {
        self.real_time.nb_of_new_vehicle_journeys()
    }

    pub fn new_vehicle_journeys(&self) -> impl Iterator<Item = NewVehicleJourneyIdx> {
        self.real_time.new_vehicle_journeys()
    }

    pub fn nb_of_base_vehicle_journeys(&self) -> usize {
        self.base.nb_of_vehicle_journeys()
    }

    pub fn base_vehicle_journeys(&self) -> impl Iterator<Item = BaseVehicleJourneyIdx> + 'model {
        self.base.vehicle_journeys()
    }

    pub fn vehicle_journey_idx(&self, id: &str) -> Option<VehicleJourneyIdx> {
        self.real_time.vehicle_journey_idx(id, self.base)
    }

    // works only for base vehicle_journey at this time
    pub fn vehicle_journey(&self, vehicle_journey_idx: &BaseVehicleJourneyIdx) -> &VehicleJourney {
        self.base.vehicle_journey(*vehicle_journey_idx)
    }

    pub fn routes(&self) -> impl Iterator<Item = &Route> {
        self.base.routes()
    }

    pub fn line(&self, id: &str) -> Option<&Line> {
        self.base.line(id)
    }

    pub fn route(&self, id: &str) -> Option<&Route> {
        self.base.route(id)
    }

    pub fn network(&self, id: &str) -> Option<&Network> {
        self.base.network(id)
    }

    pub fn stop_area(&self, id: &str) -> Option<&StopArea> {
        self.base.stop_area(id)
    }

    pub fn commercial_mode(&self, id: &str) -> Option<&CommercialMode> {
        self.base.commercial_mode(id)
    }

    pub fn physical_mode(&self, id: &str) -> Option<&PhysicalMode> {
        self.base.physical_mode(id)
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

    pub fn physical_mode_id(&self, physical_mode_idx: Idx<PhysicalMode>) -> &str {
        self.base.physical_mode_id(physical_mode_idx)
    }

    pub fn physical_modes_of_route(&self, route_id: &str) -> Vec<Idx<PhysicalMode>> {
        self.base
            .physical_modes_of_route(route_id)
            .into_iter()
            .collect()
    }

    pub fn stop_points_of_stop_area(&self, stop_area_id: &str) -> Vec<StopPointIdx> {
        self.base
            .stop_points_of_stop_area(stop_area_id)
            .into_iter()
            .map(StopPointIdx::Base)
            .collect()
    }

    pub fn stop_points_of_route(&self, route_id: &str) -> Vec<StopPointIdx> {
        self.base
            .stop_points_of_route(route_id)
            .into_iter()
            .map(StopPointIdx::Base)
            .collect()
    }

    pub fn stop_points_of_line(&self, line_id: &str) -> Vec<StopPointIdx> {
        self.base
            .stop_points_of_line(line_id)
            .into_iter()
            .map(StopPointIdx::Base)
            .collect()
    }

    pub fn stop_points_of_network(&self, network_id: &str) -> Vec<StopPointIdx> {
        self.base
            .stop_points_of_network(network_id)
            .into_iter()
            .map(StopPointIdx::Base)
            .collect()
    }

    pub fn stop_points_of_physical_mode(&self, physical_mode_id: &str) -> Vec<StopPointIdx> {
        self.base
            .stop_points_of_physical_mode(physical_mode_id)
            .into_iter()
            .map(StopPointIdx::Base)
            .collect()
    }

    pub fn stop_points_of_commercial_mode(&self, commercial_mode_id: &str) -> Vec<StopPointIdx> {
        self.base
            .stop_points_of_commercial_mode(commercial_mode_id)
            .into_iter()
            .map(StopPointIdx::Base)
            .collect()
    }

    pub fn stop_point_pathways(&self, stop_point_idx: &BaseStopPointIdx) -> PathwayByIter {
        self.base.stop_point_pathways(stop_point_idx)
    }
}

impl<'model> ModelRefs<'model> {
    pub fn stop_point_uri(&self, stop_point_idx: &StopPointIdx) -> String {
        match stop_point_idx {
            StopPointIdx::Base(idx) => self.base.stop_point_uri(*idx),
            StopPointIdx::New(idx) => {
                let id = &self.real_time.new_stops[idx.idx].name;
                format!("{}{}", PREFIX_ID_STOP_POINT, id)
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

    pub fn equipment(&self, stop_point_idx: &StopPointIdx) -> Option<&'model Equipment> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => self.base.equipment(*idx),
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

    pub fn stop_area_coord(&self, id: &str) -> Option<Coord> {
        self.base.stop_area_coord(id)
    }

    pub fn stop_area_uri(&self, id: &str) -> Option<String> {
        self.base.stop_area_uri(id)
    }

    pub fn stop_area_codes(
        &self,
        id: &str,
    ) -> Option<impl Iterator<Item = &(String, String)> + '_> {
        self.base.stop_area_codes(id)
    }

    pub fn stop_area_timezone(&self, stop_area_id: &str) -> Option<chrono_tz::Tz> {
        self.base.stop_area_timezone(stop_area_id)
    }

    pub fn timezone(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: NaiveDate,
    ) -> chrono_tz::Tz {
        if let VehicleJourneyIdx::Base(idx) = vehicle_journey_idx {
            if self
                .real_time
                .base_vehicle_journey_last_version(*idx, date)
                .is_none()
            {
                return self
                    .base_vehicle_journey_timezone(*idx)
                    .unwrap_or(chrono_tz::UTC);
            }
        }
        chrono_tz::UTC
    }

    fn base_vehicle_journey_timezone(&self, idx: BaseVehicleJourneyIdx) -> Option<chrono_tz::Tz> {
        self.base.timezone(idx)
    }

    pub fn stop_times(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: NaiveDate,
        from_stoptime_idx: StopTimeIdx,
        to_stoptime_idx: StopTimeIdx,
        real_time_level: RealTimeLevel,
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
        date: NaiveDate,
        from_stoptime_idx: StopTimeIdx,
        to_stoptime_idx: StopTimeIdx,
    ) -> Option<StopTimes> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::New(_) => None,
            VehicleJourneyIdx::Base(idx) => {
                if !self.base.trip_exists(*idx, date) {
                    return None;
                }
                let base_stop_times = self
                    .base
                    .stop_times_partial(*idx, from_stoptime_idx, to_stoptime_idx)
                    .ok()?;
                let stop_times = StopTimes::Base(base_stop_times);
                Some(stop_times)
            }
        }
    }

    pub fn stop_times_real_time(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: NaiveDate,
        from_stoptime_idx: StopTimeIdx,
        to_stoptime_idx: StopTimeIdx,
    ) -> Option<StopTimes> {
        match self.real_time.last_version(vehicle_journey_idx, date) {
            Some(TripVersion::Present(stop_times)) => {
                let range = from_stoptime_idx.idx..=to_stoptime_idx.idx;
                let inner = stop_times[range].iter();
                let iter = StopTimes::New(RealTimeStopTimes { inner });
                Some(iter)
            }
            Some(TripVersion::Deleted()) => None,
            None => {
                // there is no realtime data for this trip
                // so its base schedule IS the real time schedule
                if let VehicleJourneyIdx::Base(base_idx) = vehicle_journey_idx {
                    let inner = self
                        .base
                        .stop_times_partial(*base_idx, from_stoptime_idx, to_stoptime_idx)
                        .ok()?;
                    let iter = StopTimes::Base(inner);
                    Some(iter)
                } else {
                    None
                }
            }
        }
    }

    pub fn has_datetime_estimated(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: NaiveDate,
        from_stoptime_idx: StopTimeIdx,
        to_stoptime_idx: StopTimeIdx,
        real_time_level: RealTimeLevel,
    ) -> bool {
        if self
            .stop_times(
                vehicle_journey_idx,
                date,
                from_stoptime_idx,
                to_stoptime_idx,
                real_time_level,
            )
            .is_none()
        {
            return false;
        }
        match (vehicle_journey_idx, real_time_level) {
            (VehicleJourneyIdx::Base(idx), RealTimeLevel::Base) => self
                .base
                .has_datetime_estimated(*idx, from_stoptime_idx, to_stoptime_idx),
            (VehicleJourneyIdx::Base(_), RealTimeLevel::RealTime) => false,
            (VehicleJourneyIdx::New(_), _) => false,
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
        date: NaiveDate,
        real_time_level: RealTimeLevel,
    ) -> Option<&str> {
        match (vehicle_journey_idx, real_time_level) {
            (VehicleJourneyIdx::Base(idx), RealTimeLevel::RealTime) => {
                let has_history = self.real_time.base_vehicle_journey_last_version(*idx, date);
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
        date: NaiveDate,
    ) -> Option<&str> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let has_history = self.real_time.base_vehicle_journey_last_version(*idx, date);
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
        date: NaiveDate,
    ) -> Option<&Rgb> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let has_history = self.real_time.base_vehicle_journey_last_version(*idx, date);
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
        date: NaiveDate,
    ) -> Option<&transit_model::objects::Rgb> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let has_history = self.real_time.base_vehicle_journey_last_version(*idx, date);
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
        date: NaiveDate,
    ) -> Option<&str> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let has_history = self.real_time.base_vehicle_journey_last_version(*idx, date);
                if has_history.is_none() {
                    self.base.trip_short_name(*idx)
                } else {
                    None
                }
            }
            VehicleJourneyIdx::New(_idx) => None,
        }
    }

    pub fn contributors(&self) -> impl Iterator<Item = Contributor> + '_ {
        self.base.contributors()
    }
}
