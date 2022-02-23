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

use chrono::{Duration, NaiveDate};
use std::collections::BTreeSet;
use tracing::warn;
use transit_model::objects::{
    Availability, CommercialMode, Equipment, Line, Network, PhysicalMode, Properties, Route,
    StopArea, VehicleJourney,
};

use typed_index_collection::Idx;

use crate::{
    time::{calendar, SecondsSinceTimezonedDayStart},
    timetables::FlowDirection,
    LoadsData, PositiveDuration,
};

use super::{
    real_time_disruption::time_periods::TimePeriod, Contributor, Coord, Rgb, StopPointIdx,
    StopTime, StopTimeIdx,
};

pub const PREFIX_ID_NETWORK: &str = "network:";
pub const PREFIX_ID_LINE: &str = "line:";
pub const PREFIX_ID_ROUTE: &str = "route:";
pub const PREFIX_ID_VEHICLE_JOURNEY: &str = "vehicle_journey:";
pub const PREFIX_ID_STOP_AREA: &str = "stop_area:";
pub const PREFIX_ID_STOP_POINT: &str = "stop_point:";
pub const PREFIX_ID_COMMERCIAL_MODE: &str = "commercial_mode:";
pub const PREFIX_ID_PHYSICAL_MODE: &str = "physical_mode:";
pub const PREFIX_ID_COORD: &str = "coord:";

pub fn strip_id_prefix<'a>(id: &'a str, prefix: &str) -> &'a str {
    id.strip_prefix(prefix).unwrap_or_else(|| {
        warn!(
            "Provided uri {} doesn't start with `{}`. I continue with it",
            id, prefix,
        );
        id
    })
}

pub type Collections = transit_model::model::Collections;
pub type Model = transit_model::model::Model;

pub struct BaseModel {
    model: Model,
    loads_data: LoadsData,
    validity_period: (NaiveDate, NaiveDate),
    default_transfer_duration: PositiveDuration,
}

pub type BaseVehicleJourneyIdx = Idx<transit_model::objects::VehicleJourney>;
pub type BaseStopPointIdx = Idx<transit_model::objects::StopPoint>;
pub type BaseTransferIdx = Idx<transit_model::objects::Transfer>;

pub type BaseStopTime = transit_model::objects::StopTime;

#[derive(Debug, Clone)]
pub enum BadModel {
    NoDataset,
    StartDateAfterEndDate,
}

pub enum VehicleJourneyPropertyKey {
    WheelChairAccessible,
    BikeAccepted,
    AirConditioned,
    VisualAnnouncement,
    AudibleAnnouncement,
    AppropriateEscort,
    AppropriateSignage,
    SchoolVehicle,
}

pub enum EquipmentPropertyKey {
    WheelChairBoarding,
    Sheltered,
    Elevator,
    Escalator,
    BikeAccepted,
    BikeDepot,
    VisualAnnouncement,
    AudibleAnnouncement,
    AppropriateEscort,
    AppropriateSignage,
}

impl BaseModel {
    pub fn from_transit_model(
        model: transit_model::Model,
        loads_data: LoadsData,
        default_transfer_duration: PositiveDuration,
    ) -> Result<Self, BadModel> {
        Self::new(model, loads_data, default_transfer_duration)
    }

    pub fn empty() -> Self {
        let model = Model::new(Collections::default()).unwrap();
        // let dataset = transit_model::objects::Dataset::default();
        // collections.datasets.push(dataset).unwrap();
        let loads_data = LoadsData::empty();
        let day = NaiveDate::from_ymd(1970, 1, 1);
        Self {
            model,
            loads_data,
            validity_period: (day, day),
            default_transfer_duration: PositiveDuration::zero(),
        }
    }

    pub fn new(
        model: transit_model::model::Model,
        loads_data: LoadsData,
        default_transfer_duration: PositiveDuration,
    ) -> Result<Self, BadModel> {
        let validity_period = model
            .calculate_validity_period()
            .map_err(|_| BadModel::NoDataset)?;
        if validity_period.0 > validity_period.1 {
            return Err(BadModel::StartDateAfterEndDate);
        }
        Ok(Self {
            model,
            loads_data,
            validity_period,
            default_transfer_duration,
        })
    }

    pub fn loads_data(&self) -> &LoadsData {
        &self.loads_data
    }

    pub fn validity_period(&self) -> (NaiveDate, NaiveDate) {
        self.validity_period
    }

    pub fn time_period(&self) -> TimePeriod {
        let start_datetime = self.validity_period.0.and_hms(0, 0, 0);
        let end_datetime = self.validity_period.1.and_hms(0, 0, 0) + Duration::days(1);
        TimePeriod::new(start_datetime, end_datetime).unwrap() // unwrap is safe here, because we check in new()
                                                               // that validity_period.0 <= validity_period.1
    }
}

// stop_points
impl BaseModel {
    pub fn nb_of_stop_points(&self) -> usize {
        self.model.stop_points.len()
    }

    pub fn stop_points(&self) -> BaseStopPoints<'_> {
        BaseStopPoints {
            inner: self.model.stop_points.iter(),
        }
    }

    pub fn stop_point_idx(&self, stop_id: &str) -> Option<BaseStopPointIdx> {
        self.model.stop_points.get_idx(stop_id)
    }

    pub fn stop_point_name(&self, stop_idx: BaseStopPointIdx) -> &str {
        &self.model.stop_points[stop_idx].name
    }

    pub fn stop_point_id(&self, stop_idx: BaseStopPointIdx) -> &str {
        &self.model.stop_points[stop_idx].id
    }

    pub fn stop_point_uri(&self, idx: BaseStopPointIdx) -> String {
        let id = &self.model.stop_points[idx].id;
        format!("{}{}", PREFIX_ID_STOP_POINT, id)
    }

    pub fn house_number(&self, idx: BaseStopPointIdx) -> Option<&str> {
        let stop_point = &self.model.stop_points[idx];
        let address_id = stop_point.address_id.as_ref()?;
        let address = self.model.addresses.get(address_id)?;
        let house_number = address.house_number.as_ref()?;
        Some(house_number.as_str())
    }

    pub fn street_name(&self, idx: BaseStopPointIdx) -> Option<&str> {
        let stop_point = &self.model.stop_points[idx];
        let address_id = &stop_point.address_id.as_ref()?;
        let address = &self.model.addresses.get(address_id)?;
        Some(address.id.as_str())
    }

    pub fn equipment(&self, idx: BaseStopPointIdx) -> Option<&Equipment> {
        let stop_point = &self.model.stop_points[idx];
        let equipment_id = &stop_point.equipment_id.as_ref()?;
        let equipment = &self.model.equipments.get(equipment_id)?;
        Some(equipment)
    }

    pub fn stop_point_property(
        &self,
        stop_point_idx: BaseStopPointIdx,
        property_key: EquipmentPropertyKey,
    ) -> bool {
        if let Some(equipments) = self.equipment(stop_point_idx) {
            equipment_property(equipments, &property_key)
        } else {
            false
        }
    }

    pub fn coord(&self, idx: BaseStopPointIdx) -> Coord {
        let stop_point = &self.model.stop_points[idx];
        Coord {
            lat: stop_point.coord.lat,
            lon: stop_point.coord.lon,
        }
    }

    pub fn platform_code(&self, idx: BaseStopPointIdx) -> Option<&str> {
        let stop_point = &self.model.stop_points[idx];
        stop_point.platform_code.as_deref()
    }

    pub fn fare_zone_id(&self, idx: BaseStopPointIdx) -> Option<&str> {
        let stop_point = &self.model.stop_points[idx];
        stop_point.fare_zone_id.as_deref()
    }

    pub fn codes(
        &self,
        idx: BaseStopPointIdx,
    ) -> Option<impl Iterator<Item = &(String, String)> + '_> {
        let stop_point = &self.model.stop_points[idx];
        Some(stop_point.codes.iter())
    }

    pub fn stop_area_name(&self, stop_idx: BaseStopPointIdx) -> &str {
        &self.model.stop_points[stop_idx].stop_area_id
    }

    pub fn stop_area_uri(&self, stop_area_id: &str) -> Option<String> {
        let stop_area = self.model.stop_areas.get(stop_area_id)?;
        Some(format!("{}{}", PREFIX_ID_STOP_AREA, stop_area.id))
    }

    pub fn stop_area_coord(&self, stop_area_id: &str) -> Option<Coord> {
        let stop_area = self.model.stop_areas.get(stop_area_id)?;
        Some(Coord {
            lat: stop_area.coord.lat,
            lon: stop_area.coord.lon,
        })
    }

    pub fn stop_area_codes(
        &self,
        stop_area_id: &str,
    ) -> Option<impl Iterator<Item = &(String, String)> + '_> {
        let stop_area = self.model.stop_areas.get(stop_area_id)?;
        Some(stop_area.codes.iter())
    }

    pub fn stop_area_timezone(&self, stop_area_id: &str) -> Option<chrono_tz::Tz> {
        let stop_area = self.model.stop_areas.get(stop_area_id)?;
        stop_area.timezone
    }

    pub fn stop_points_of_stop_area(&self, stop_area_id: &str) -> BTreeSet<BaseStopPointIdx> {
        match self.model.stop_areas.get_idx(stop_area_id) {
            Some(idx) => self.model.get_corresponding_from_idx(idx),
            None => BTreeSet::new(),
        }
    }

    pub fn stop_points_of_route(&self, route_id: &str) -> BTreeSet<BaseStopPointIdx> {
        match self.model.routes.get_idx(route_id) {
            Some(idx) => self.model.get_corresponding_from_idx(idx),
            None => BTreeSet::new(),
        }
    }

    pub fn stop_points_of_line(&self, line_id: &str) -> BTreeSet<BaseStopPointIdx> {
        match self.model.lines.get_idx(line_id) {
            Some(idx) => self.model.get_corresponding_from_idx(idx),
            None => BTreeSet::new(),
        }
    }

    pub fn stop_points_of_network(&self, network_id: &str) -> BTreeSet<BaseStopPointIdx> {
        match self.model.networks.get_idx(network_id) {
            Some(idx) => self.model.get_corresponding_from_idx(idx),
            None => BTreeSet::new(),
        }
    }

    pub fn stop_points_of_physical_mode(
        &self,
        physical_mode_id: &str,
    ) -> BTreeSet<BaseStopPointIdx> {
        match self.model.physical_modes.get_idx(physical_mode_id) {
            Some(idx) => self.model.get_corresponding_from_idx(idx),
            None => BTreeSet::new(),
        }
    }

    pub fn stop_points_of_commercial_mode(
        &self,
        commercial_mode_id: &str,
    ) -> BTreeSet<BaseStopPointIdx> {
        match self.model.commercial_modes.get_idx(commercial_mode_id) {
            Some(idx) => self.model.get_corresponding_from_idx(idx),
            None => BTreeSet::new(),
        }
    }

    pub fn physical_mode_id(&self, physical_mode_idx: Idx<PhysicalMode>) -> &str {
        &self.model.physical_modes[physical_mode_idx].id
    }

    pub fn physical_modes_of_route(&self, route_id: &str) -> BTreeSet<Idx<PhysicalMode>> {
        match self.model.routes.get_idx(route_id) {
            Some(idx) => self.model.get_corresponding_from_idx(idx),
            None => BTreeSet::new(),
        }
    }
}

// vehicle journey
impl BaseModel {
    pub fn nb_of_vehicle_journeys(&self) -> usize {
        self.model.vehicle_journeys.len()
    }

    pub fn vehicle_journey(&self, vehicle_journey_idx: BaseVehicleJourneyIdx) -> &VehicleJourney {
        &self.model.vehicle_journeys[vehicle_journey_idx]
    }

    pub fn vehicle_journeys(&self) -> impl Iterator<Item = BaseVehicleJourneyIdx> + '_ {
        self.model.vehicle_journeys.iter().map(|(idx, _)| idx)
    }

    pub fn vehicle_journey_idx(&self, vehicle_journey_id: &str) -> Option<BaseVehicleJourneyIdx> {
        self.model.vehicle_journeys.get_idx(vehicle_journey_id)
    }

    pub fn vehicle_journey_name(&self, vehicle_journey_idx: BaseVehicleJourneyIdx) -> &str {
        &self.model.vehicle_journeys[vehicle_journey_idx].id
    }

    pub fn timezone(&self, idx: BaseVehicleJourneyIdx) -> Option<chrono_tz::Tz> {
        let line = self.vehicle_journey_line(idx)?;
        let network = self.model.networks.get(&line.network_id)?;
        network.timezone
    }

    pub fn vehicle_journey_dates(
        &self,
        idx: BaseVehicleJourneyIdx,
    ) -> Option<impl Iterator<Item = NaiveDate> + '_ + Clone> {
        let vehicle_journey = &self.model.vehicle_journeys[idx];
        self.model
            .calendars
            .get(&vehicle_journey.service_id)
            .map(|calendar| calendar.dates.iter().copied())
    }

    fn vehicle_journey_route(
        &self,
        idx: BaseVehicleJourneyIdx,
    ) -> Option<&transit_model::objects::Route> {
        let route_id = &self.model.vehicle_journeys[idx].route_id;
        self.model.routes.get(route_id)
    }

    fn vehicle_journey_line(
        &self,
        idx: BaseVehicleJourneyIdx,
    ) -> Option<&transit_model::objects::Line> {
        self.vehicle_journey_route(idx)
            .and_then(|route| self.model.lines.get(route.line_id.as_str()))
    }

    pub fn line_name(&self, idx: BaseVehicleJourneyIdx) -> Option<&str> {
        self.vehicle_journey_route(idx)
            .map(|route| route.line_id.as_str())
    }

    pub fn vehicle_journey_property(
        &self,
        idx: BaseVehicleJourneyIdx,
        property_key: VehicleJourneyPropertyKey,
    ) -> bool {
        let string_key = match property_key {
            VehicleJourneyPropertyKey::WheelChairAccessible => "wheelchair_accessible",
            VehicleJourneyPropertyKey::BikeAccepted => "bike_accepted",
            VehicleJourneyPropertyKey::AirConditioned => "air_conditioned",
            VehicleJourneyPropertyKey::VisualAnnouncement => "visual_announcement",
            VehicleJourneyPropertyKey::AudibleAnnouncement => "audible_announcement",
            VehicleJourneyPropertyKey::AppropriateEscort => "appropriate_escort",
            VehicleJourneyPropertyKey::AppropriateSignage => "appropriate_signage",
            VehicleJourneyPropertyKey::SchoolVehicle => "school_vehicle_type",
        };
        let properties = &self.model.vehicle_journeys[idx].properties();
        let value = properties.get(string_key);
        match value {
            Some(value) => matches!(value.as_str(), "1"), // return true only if value == "1"
            None => false,
        }
    }

    pub fn line_code(&self, idx: BaseVehicleJourneyIdx) -> Option<&str> {
        self.vehicle_journey_line(idx)
            .and_then(|line| line.code.as_ref())
            .map(|s| s.as_str())
    }

    pub fn headsign(&self, idx: BaseVehicleJourneyIdx) -> Option<&str> {
        self.model.vehicle_journeys[idx].headsign.as_deref()
    }

    pub fn direction(&self, idx: BaseVehicleJourneyIdx) -> Option<&str> {
        let route = self.vehicle_journey_route(idx)?;
        let destination_id = route.destination_id.as_ref()?;
        let stop_area = self.model.stop_areas.get(destination_id)?;
        Some(stop_area.name.as_str())
    }

    pub fn route_name(&self, idx: BaseVehicleJourneyIdx) -> &str {
        self.model.vehicle_journeys[idx].route_id.as_str()
    }

    pub fn network_name(&self, vehicle_journey_idx: BaseVehicleJourneyIdx) -> Option<&str> {
        self.vehicle_journey_line(vehicle_journey_idx)
            .map(|line| line.network_id.as_str())
    }

    pub fn line_color(&self, idx: BaseVehicleJourneyIdx) -> Option<&Rgb> {
        let line = self.vehicle_journey_line(idx)?;
        line.color.as_ref()
    }

    pub fn text_color(&self, idx: BaseVehicleJourneyIdx) -> Option<&Rgb> {
        let line = self.vehicle_journey_line(idx)?;
        line.text_color.as_ref()
    }

    pub fn trip_short_name(&self, idx: BaseVehicleJourneyIdx) -> Option<&str> {
        let vj = &self.model.vehicle_journeys[idx];
        vj.short_name
            .as_ref()
            .or(vj.headsign.as_ref())
            .map(|s| s.as_str())
    }

    pub fn physical_mode_name(&self, idx: BaseVehicleJourneyIdx) -> &str {
        self.model.vehicle_journeys[idx].physical_mode_id.as_str()
    }

    pub fn commercial_mode_name(&self, idx: BaseVehicleJourneyIdx) -> Option<&str> {
        self.vehicle_journey_line(idx)
            .map(|line| line.commercial_mode_id.as_str())
    }

    pub fn stop_point_at(
        &self,
        vehicle_journey_idx: BaseVehicleJourneyIdx,
        stop_time_idx: StopTimeIdx,
    ) -> Option<BaseStopPointIdx> {
        self.model.vehicle_journeys[vehicle_journey_idx]
            .stop_times
            .get(stop_time_idx.idx)
            .map(|stop_time| stop_time.stop_point_idx)
    }

    pub fn trip_exists(&self, vehicle_journey_idx: BaseVehicleJourneyIdx, date: NaiveDate) -> bool {
        let vehicle_journey = &self.model.vehicle_journeys[vehicle_journey_idx];
        let has_calendar = &self.model.calendars.get(&vehicle_journey.service_id);
        if let Some(calendar) = has_calendar {
            calendar.dates.contains(&date)
        } else {
            false
        }
    }

    pub fn trip_time_period(
        &self,
        vehicle_journey_idx: BaseVehicleJourneyIdx,
        date: &NaiveDate,
    ) -> Option<TimePeriod> {
        if !self.trip_exists(vehicle_journey_idx, *date) {
            return None;
        }
        let stop_times = self.stop_times(vehicle_journey_idx).ok()?;
        let timezone = self
            .timezone(vehicle_journey_idx)
            .unwrap_or(chrono_tz::Tz::UTC);

        let earliest_local_time = stop_times
            .clone()
            .map(|stop_time| std::cmp::min(stop_time.board_time, stop_time.debark_time))
            .min()?;

        let latest_local_time = stop_times
            .clone()
            .map(|stop_time| std::cmp::max(stop_time.board_time, stop_time.debark_time))
            .max()?;

        let first_time_utc = calendar::compose(date, &earliest_local_time, &timezone);
        let last_time_utc = calendar::compose(date, &latest_local_time, &timezone);

        // since TimePeriod is open at the end, we add 1s to the last_time
        // so that the constructed time_period contains last_time
        let last_time_utc = last_time_utc + chrono::Duration::seconds(1);

        TimePeriod::new(first_time_utc, last_time_utc).ok()
    }

    pub fn co2_emission(&self, vehicle_journey_idx: BaseVehicleJourneyIdx) -> Option<f32> {
        let physical_mode_name = self.physical_mode_name(vehicle_journey_idx);
        let physical_mode = self.model.physical_modes.get(physical_mode_name)?;
        physical_mode.co2_emission
    }

    pub fn has_datetime_estimated(
        &self,
        _vehicle_journey_idx: BaseVehicleJourneyIdx,
        _from_stoptime_idx: StopTimeIdx,
        _to_stoptime_idx: StopTimeIdx,
    ) -> bool {
        false
        // TODO : update this function to use stop_time.precision
        //
        // let stop_times =
        //     self.stop_times_inner(vehicle_journey_idx, from_stoptime_idx, to_stoptime_idx);
        // let is_empty = stop_times.is_empty();
        // if is_empty {
        //     false
        // } else {
        //     let first = stop_times.first().unwrap(); // unwrap is safe, since we checked that !is_empty
        //     let last = stop_times.last().unwrap();

        //     first.datetime_estimated || last.datetime_estimated
        // }
    }
}

// contains ids
impl BaseModel {
    pub fn contains_line_id(&self, id: &str) -> bool {
        self.model.lines.contains_id(id)
    }

    pub fn contains_route_id(&self, id: &str) -> bool {
        self.model.routes.contains_id(id)
    }

    pub fn contains_network_id(&self, id: &str) -> bool {
        self.model.networks.contains_id(id)
    }

    pub fn contains_physical_mode_id(&self, id: &str) -> bool {
        self.model.physical_modes.contains_id(id)
    }

    pub fn contains_commercial_model_id(&self, id: &str) -> bool {
        self.model.commercial_modes.contains_id(id)
    }

    pub fn contains_stop_point_id(&self, id: &str) -> bool {
        self.model.stop_points.contains_id(id)
    }

    pub fn contains_stop_area_id(&self, id: &str) -> bool {
        self.model.stop_areas.contains_id(id)
    }
}

// stop_times
impl BaseModel {
    pub fn stop_times(
        &self,
        vehicle_journey_idx: BaseVehicleJourneyIdx,
    ) -> Result<BaseStopTimes<'_>, (BadStopTime, StopTimeIdx)> {
        let vj = &self.model.vehicle_journeys[vehicle_journey_idx];
        let stop_times = &vj.stop_times;
        let inner = stop_times.iter();
        BaseStopTimes::new(inner).map_err(|(err, idx)| (err, StopTimeIdx { idx }))
    }

    pub fn stop_times_partial(
        &self,
        vehicle_journey_idx: BaseVehicleJourneyIdx,
        from_stoptime_idx: StopTimeIdx,
        to_stoptime_idx: StopTimeIdx,
    ) -> Result<BaseStopTimes<'_>, (BadStopTime, StopTimeIdx)> {
        let inner = self.stop_times_inner(vehicle_journey_idx, from_stoptime_idx, to_stoptime_idx);
        BaseStopTimes::new(inner.iter()).map_err(|(err, idx)| {
            (
                err,
                StopTimeIdx {
                    idx: from_stoptime_idx.idx + idx,
                },
            )
        })
    }

    fn stop_times_inner(
        &self,
        vehicle_journey_idx: BaseVehicleJourneyIdx,
        from_stoptime_idx: StopTimeIdx,
        to_stoptime_idx: StopTimeIdx,
    ) -> &[transit_model::objects::StopTime] {
        let vj = &self.model.vehicle_journeys[vehicle_journey_idx];
        let stop_times = &vj.stop_times;
        let from_idx = from_stoptime_idx.idx;
        let to_idx = to_stoptime_idx.idx;
        let range = from_idx..=to_idx;
        &stop_times[range]
    }
}

// transfers
impl BaseModel {
    pub fn nb_of_transfers(&self) -> usize {
        self.model.transfers.len()
    }

    pub fn transfers(&self) -> impl Iterator<Item = BaseTransferIdx> + '_ + Clone {
        self.model.transfers.iter().map(|(idx, _)| idx)
    }

    pub fn from_stop(&self, transfer_idx: BaseTransferIdx) -> Option<BaseStopPointIdx> {
        let stop_id = self.model.transfers[transfer_idx].from_stop_id.as_str();
        self.model.stop_points.get_idx(stop_id)
    }

    pub fn from_stop_name(&self, transfer_idx: BaseTransferIdx) -> &str {
        self.model.transfers[transfer_idx].from_stop_id.as_str()
    }

    pub fn to_stop(&self, transfer_idx: BaseTransferIdx) -> Option<BaseStopPointIdx> {
        let stop_id = self.model.transfers[transfer_idx].to_stop_id.as_str();
        self.model.stop_points.get_idx(stop_id)
    }

    pub fn to_stop_name(&self, transfer_idx: BaseTransferIdx) -> &str {
        self.model.transfers[transfer_idx].to_stop_id.as_str()
    }

    pub fn transfer_duration(&self, transfer_idx: BaseTransferIdx) -> PositiveDuration {
        let seconds = self.model.transfers[transfer_idx]
            .real_min_transfer_time
            .unwrap_or(self.default_transfer_duration.seconds);
        PositiveDuration { seconds }
    }

    pub fn transfer_walking_duration(&self, transfer_idx: BaseTransferIdx) -> PositiveDuration {
        let seconds = self.model.transfers[transfer_idx]
            .min_transfer_time
            .unwrap_or(0u32);
        PositiveDuration { seconds }
    }

    pub fn transfer_property(
        &self,
        transfer_idx: BaseTransferIdx,
        property_key: EquipmentPropertyKey,
    ) -> bool {
        let transfer = &self.model.transfers[transfer_idx];
        if let Some(equipment_id) = &transfer.equipment_id {
            let equipments = &self.model.equipments.get(equipment_id);
            if let Some(equipments) = equipments {
                equipment_property(equipments, &property_key)
            } else {
                false
            }
        } else {
            false
        }
    }
}

// various
impl BaseModel {
    pub fn contributors(&self) -> impl Iterator<Item = Contributor> + '_ {
        self.model.contributors.values().map(|c| Contributor {
            id: c.id.clone(),
            name: c.name.clone(),
            license: c.license.clone(),
            url: c.website.clone(),
        })
    }

    pub fn line(&self, id: &str) -> Option<&Line> {
        self.model.lines.get(id)
    }

    pub fn route(&self, id: &str) -> Option<&Route> {
        self.model.routes.get(id)
    }

    pub fn routes(&self) -> impl Iterator<Item = &Route> {
        self.model.routes.iter().map(|(_, route)| route)
    }

    pub fn network(&self, id: &str) -> Option<&Network> {
        self.model.networks.get(id)
    }

    pub fn stop_area(&self, id: &str) -> Option<&StopArea> {
        self.model.stop_areas.get(id)
    }

    pub fn commercial_mode(&self, id: &str) -> Option<&CommercialMode> {
        self.model.commercial_modes.get(id)
    }

    pub fn physical_mode(&self, id: &str) -> Option<&PhysicalMode> {
        self.model.physical_modes.get(id)
    }
}

fn equipment_property(equipments: &Equipment, property_key: &EquipmentPropertyKey) -> bool {
    let property = match property_key {
        EquipmentPropertyKey::WheelChairBoarding => equipments.wheelchair_boarding,
        EquipmentPropertyKey::Sheltered => equipments.sheltered,
        EquipmentPropertyKey::Elevator => equipments.elevator,
        EquipmentPropertyKey::Escalator => equipments.escalator,
        EquipmentPropertyKey::BikeAccepted => equipments.bike_accepted,
        EquipmentPropertyKey::BikeDepot => equipments.bike_depot,
        EquipmentPropertyKey::VisualAnnouncement => equipments.visual_announcement,
        EquipmentPropertyKey::AudibleAnnouncement => equipments.audible_announcement,
        EquipmentPropertyKey::AppropriateEscort => equipments.appropriate_escort,
        EquipmentPropertyKey::AppropriateSignage => equipments.appropriate_signage,
    };
    property == Availability::Available
}

#[derive(Debug, Clone)]
pub struct BaseStopTimes<'a> {
    inner: std::slice::Iter<'a, transit_model::objects::StopTime>,
}

impl<'a> BaseStopTimes<'a> {
    pub fn new(
        inner: std::slice::Iter<'a, transit_model::objects::StopTime>,
    ) -> Result<Self, (BadStopTime, usize)> {
        let copy = inner.clone();
        // we check that every transit_model::objects::StopTime
        // can be transformed into a loki::models::StopTime
        for (stop_time_idx, stop_time) in copy.enumerate() {
            flow(stop_time).map_err(|err| (err, stop_time_idx))?;
            board_time(stop_time).ok_or((BadStopTime::BoardTime, stop_time_idx))?;
            debark_time(stop_time).ok_or((BadStopTime::DebarkTime, stop_time_idx))?;
        }
        Ok(Self { inner })
    }
}

impl<'a> Iterator for BaseStopTimes<'a> {
    type Item = StopTime;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|stop_time| {
            StopTime {
                stop: StopPointIdx::Base(stop_time.stop_point_idx),
                // unwraps are safe, beecause of checks in new()
                board_time: board_time(stop_time).unwrap(),
                debark_time: debark_time(stop_time).unwrap(),
                flow_direction: flow(stop_time).unwrap(),
                local_zone_id: stop_time.local_zone_id,
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> ExactSizeIterator for BaseStopTimes<'a> {}

#[derive(Debug)]
pub enum BadStopTime {
    PickupType,
    DropOffType,
    BoardTime,
    DebarkTime,
}

pub fn flow(stop_time: &transit_model::objects::StopTime) -> Result<FlowDirection, BadStopTime> {
    let can_board = match stop_time.pickup_type {
        0 => true,
        1 => false,
        _ => {
            return Err(BadStopTime::PickupType);
        }
    };
    let can_debark = match stop_time.drop_off_type {
        0 => true,
        1 => false,
        _ => {
            return Err(BadStopTime::DropOffType);
        }
    };
    match (can_board, can_debark) {
        (true, true) => Ok(FlowDirection::BoardAndDebark),
        (false, true) => Ok(FlowDirection::DebarkOnly),
        (true, false) => Ok(FlowDirection::BoardOnly),
        (false, false) => Ok(FlowDirection::NoBoardDebark),
    }
}

fn board_time(
    stop_time: &transit_model::objects::StopTime,
) -> Option<SecondsSinceTimezonedDayStart> {
    let departure_seconds = i32::try_from(stop_time.departure_time.total_seconds()).ok()?;
    let boarding_duration = i32::from(stop_time.boarding_duration);
    let seconds = departure_seconds.checked_sub(boarding_duration)?;
    SecondsSinceTimezonedDayStart::from_seconds(seconds)
}

fn debark_time(
    stop_time: &transit_model::objects::StopTime,
) -> Option<SecondsSinceTimezonedDayStart> {
    let arrival_seconds = i32::try_from(stop_time.arrival_time.total_seconds()).ok()?;
    let alighting_duration = i32::try_from(stop_time.alighting_duration).ok()?;
    let seconds = arrival_seconds.checked_add(alighting_duration)?;
    SecondsSinceTimezonedDayStart::from_seconds(seconds)
}

#[derive(Clone, Debug)]
pub struct BaseStopPoints<'a> {
    inner: typed_index_collection::Iter<'a, transit_model::objects::StopPoint>,
}

impl<'a> Iterator for BaseStopPoints<'a> {
    type Item = BaseStopPointIdx;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(idx, _)| idx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> ExactSizeIterator for BaseStopPoints<'a> {}
