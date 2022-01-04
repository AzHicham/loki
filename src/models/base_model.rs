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

use chrono::NaiveDate;
use typed_index_collection::Idx;

use crate::{time::SecondsSinceTimezonedDayStart, timetables::FlowDirection, LoadsData};

use super::{Coord, Rgb, StopPointIdx, StopTime, StopTimeIdx};

pub type Collections = transit_model::model::Collections;

pub struct BaseModel {
    collections: transit_model::model::Collections,
    loads_data: LoadsData,
}

pub type BaseVehicleJourneyIdx = Idx<transit_model::objects::VehicleJourney>;
pub type BaseStopPointIdx = Idx<transit_model::objects::StopPoint>;
pub type BaseTransferIdx = Idx<transit_model::objects::Transfer>;

pub type BaseStopTime = transit_model::objects::StopTime;

impl BaseModel {
    pub fn from_transit_model(model: transit_model::Model, loads_data: LoadsData) -> Self {
        Self {
            collections: model.into_collections(),
            loads_data,
        }
    }

    pub fn empty() -> Self {
        let mut collections = Collections::default();
        let dataset = transit_model::objects::Dataset::default();
        collections.datasets.push(dataset).unwrap();
        let loads_data = LoadsData::empty();
        Self {
            collections,
            loads_data,
        }
    }

    pub fn new(collections: transit_model::model::Collections, loads_data: LoadsData) -> Self {
        Self {
            collections,
            loads_data,
        }
    }

    pub fn loads_data(&self) -> &LoadsData {
        &self.loads_data
    }

    // stop_points

    pub fn nb_of_stop_points(&self) -> usize {
        self.collections.stop_points.len()
    }

    pub fn stop_points(&self) -> impl Iterator<Item = BaseStopPointIdx> + '_ {
        self.collections.stop_points.iter().map(|(idx, _)| idx)
    }

    pub fn stop_point_idx(&self, stop_id: &str) -> Option<BaseStopPointIdx> {
        self.collections.stop_points.get_idx(stop_id)
    }

    pub fn stop_point_name(&self, stop_idx: BaseStopPointIdx) -> &str {
        &self.collections.stop_points[stop_idx].name
    }

    pub fn stop_area_name(&self, stop_idx: BaseStopPointIdx) -> &str {
        &self.collections.stop_points[stop_idx].stop_area_id
    }

    pub fn stop_point_uri(&self, idx: BaseStopPointIdx) -> String {
        let id = &self.collections.stop_points[idx].id;
        format!("stop_point:{}", id)
    }

    pub fn house_number(&self, idx: BaseStopPointIdx) -> Option<&str> {
        let stop_point = &self.collections.stop_points[idx];
        let address_id = &stop_point.address_id?;
        let address = &self.collections.addresses.get(address_id)?;
        address.house_number.map(|s| s.as_str())
    }

    pub fn street_name(&self, idx: BaseStopPointIdx) -> Option<&str> {
        let stop_point = &self.collections.stop_points[idx];
        let address_id = &stop_point.address_id?;
        let address = &self.collections.addresses.get(address_id)?;
        Some(address.street_name.as_str())
    }

    pub fn coord(&self, idx: BaseStopPointIdx) -> Coord {
        let stop_point = &self.collections.stop_points[idx];
        Coord {
            lat: stop_point.coord.lat,
            lon: stop_point.coord.lon,
        }
    }

    pub fn platform_code(&self, idx: BaseStopPointIdx) -> Option<&str> {
        let stop_point = &self.collections.stop_points[idx];
        stop_point.platform_code.map(|s| s.as_str())
    }

    pub fn fare_zone_id(&self, idx: BaseStopPointIdx) -> Option<&str> {
        let stop_point = &self.collections.stop_points[idx];
        stop_point.fare_zone_id.map(|s| s.as_str())
    }

    pub fn codes(
        &self,
        idx: BaseStopPointIdx,
    ) -> Option<impl Iterator<Item = &(String, String)> + '_> {
        let stop_point = &self.collections.stop_points[idx];
        Some(stop_point.codes.iter())
    }

    // vehicle journey

    pub fn nb_of_vehicle_journeys(&self) -> usize {
        self.collections.vehicle_journeys.len()
    }

    pub fn vehicle_journeys(&self) -> impl Iterator<Item = BaseVehicleJourneyIdx> + '_ {
        self.collections.vehicle_journeys.iter().map(|(idx, _)| idx)
    }

    pub fn vehicle_journey_idx(&self, vehicle_journey_id: &str) -> Option<BaseVehicleJourneyIdx> {
        self.collections
            .vehicle_journeys
            .get_idx(vehicle_journey_id)
    }

    pub fn vehicle_journey_name(&self, vehicle_journey_idx: BaseVehicleJourneyIdx) -> &str {
        &self.collections.vehicle_journeys[vehicle_journey_idx].id
    }

    pub fn timezone(&self, idx: BaseVehicleJourneyIdx) -> Option<chrono_tz::Tz> {
        let line = self.vehicle_journey_line(idx)?;
        let network = self.collections.networks.get(&line.network_id)?;
        network.timezone
    }

    pub fn vehicle_journey_dates(
        &self,
        idx: BaseVehicleJourneyIdx,
    ) -> Option<impl Iterator<Item = NaiveDate> + '_ + Clone> {
        let vehicle_journey = &self.collections.vehicle_journeys[idx];
        self.collections
            .calendars
            .get(&vehicle_journey.service_id)
            .map(|calendar| calendar.dates.iter().map(|date| date.clone()))
    }

    fn vehicle_journey_route(
        &self,
        idx: BaseVehicleJourneyIdx,
    ) -> Option<&transit_model::objects::Route> {
        let route_id = &self.collections.vehicle_journeys[idx].route_id;
        self.collections.routes.get(route_id)
    }

    fn vehicle_journey_line(
        &self,
        idx: BaseVehicleJourneyIdx,
    ) -> Option<&transit_model::objects::Line> {
        self.vehicle_journey_route(idx)
            .and_then(|route| self.collections.lines.get(route.line_id.as_str()))
    }

    pub fn line_name(&self, idx: BaseVehicleJourneyIdx) -> Option<&str> {
        self.vehicle_journey_route(idx)
            .map(|route| route.line_id.as_str())
    }

    pub fn line_code(&self, idx: BaseVehicleJourneyIdx) -> Option<&str> {
        self.vehicle_journey_line(idx)
            .map(|line| line.code)
            .flatten()
            .map(|s| s.as_str())
    }

    pub fn headsign(&self, idx: BaseVehicleJourneyIdx) -> Option<&str> {
        self.collections.vehicle_journeys[idx]
            .headsign
            .map(|s| s.as_str())
    }

    pub fn direction(&self, idx: BaseVehicleJourneyIdx) -> Option<&str> {
        let route = self.vehicle_journey_route(idx)?;
        let destination_id = route.destination_id?;
        let stop_area = self.collections.stop_areas.get(&destination_id)?;
        Some(stop_area.name.as_str())
    }

    pub fn route_name(&self, idx: BaseVehicleJourneyIdx) -> &str {
        self.collections.vehicle_journeys[idx].route_id.as_str()
    }

    pub fn network_name(&self, idx: BaseVehicleJourneyIdx) -> Option<&str> {
        self.vehicle_journey_line(idx)
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
        let vj = &self.collections.vehicle_journeys[idx];
        vj.short_name.or(vj.headsign).map(|s| s.as_str())
    }

    pub fn physical_mode_name(&self, idx: BaseVehicleJourneyIdx) -> &str {
        self.collections.vehicle_journeys[idx]
            .physical_mode_id
            .as_str()
    }

    pub fn commercial_mode_name(&self, idx: BaseVehicleJourneyIdx) -> Option<&str> {
        self.vehicle_journey_line(idx)
            .map(|line| line.commercial_mode_id.as_str())
    }

    pub fn stop_point_at(
        &self,
        vehicle_journey_idx: BaseVehicleJourneyIdx,
        stop_time_idx: usize,
    ) -> Option<BaseStopPointIdx> {
        self.collections.vehicle_journeys[vehicle_journey_idx]
            .stop_times
            .get(stop_time_idx)
            .map(|stop_time| stop_time.stop_point_idx)
    }

    pub fn trip_exists(
        &self,
        vehicle_journey_idx: BaseVehicleJourneyIdx,
        date: &NaiveDate,
    ) -> bool {
        let vehicle_journey = &self.collections.vehicle_journeys[vehicle_journey_idx];
        let has_calendar = &self.collections.calendars.get(&vehicle_journey.service_id);
        if let Some(calendar) = has_calendar {
            calendar.dates.contains(date)
        } else {
            false
        }
    }

    // contains ids
    pub fn contains_line_id(&self, id: &str) -> bool {
        self.collections.lines.contains_id(id)
    }

    pub fn contains_route_id(&self, id: &str) -> bool {
        self.collections.routes.contains_id(id)
    }

    pub fn contains_network_id(&self, id: &str) -> bool {
        self.collections.networks.contains_id(id)
    }

    pub fn contains_physical_mode_id(&self, id: &str) -> bool {
        self.collections.physical_modes.contains_id(id)
    }

    pub fn contains_commercial_model_id(&self, id: &str) -> bool {
        self.collections.commercial_modes.contains_id(id)
    }

    pub fn contains_stop_point_id(&self, id: &str) -> bool {
        self.collections.stop_points.contains_id(id)
    }

    pub fn contains_stop_area_id(&self, id: &str) -> bool {
        self.collections.stop_areas.contains_id(id)
    }

    // stop_times
    pub fn stop_times(
        &self,
        vehicle_journey_idx: BaseVehicleJourneyIdx,
    ) -> Result<BaseStopTimes<'_>, (BadStopTime, StopTimeIdx)> {
        let vj = &self.collections.vehicle_journeys[vehicle_journey_idx];
        let stop_times = &vj.stop_times;
        let timezone = self.timezone(vehicle_journey_idx).unwrap_or(chrono_tz::UTC);
        let inner = stop_times.iter();
        BaseStopTimes::new(inner).map_err(|(err, idx)| (err, StopTimeIdx { idx: idx }))
    }

    // stop_times
    pub fn stop_times_partial(
        &self,
        vehicle_journey_idx: BaseVehicleJourneyIdx,
        from_stoptime_idx: StopTimeIdx,
        to_stoptime_idx: StopTimeIdx,
    ) -> Result<BaseStopTimes<'_>, (BadStopTime, StopTimeIdx)> {
        let vj = &self.collections.vehicle_journeys[vehicle_journey_idx];
        let stop_times = &vj.stop_times;
        let from_idx = from_stoptime_idx.idx;
        let to_idx = to_stoptime_idx.idx;
        let timezone = self.timezone(vehicle_journey_idx).unwrap_or(chrono_tz::UTC);
        let range = from_idx..=to_idx;
        let inner = stop_times[range].iter();
        BaseStopTimes::new(inner).map_err(|(err, idx)| {
            (
                err,
                StopTimeIdx {
                    idx: from_idx + idx,
                },
            )
        })
    }

    pub fn first_last_stop_time(
        &self,
        vehicle_journey_idx: BaseVehicleJourneyIdx,
    ) -> (StopTimeIdx, StopTimeIdx) {
        let first = StopTimeIdx { idx: 0 };
        let stop_times = &self.collections.vehicle_journeys[vehicle_journey_idx].stop_times;
        let last = StopTimeIdx {
            idx: stop_times.len(),
        };
        (first, last)
    }
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
            board_time(stop_time).ok_or_else(|| (BadStopTime::BoardTime, stop_time_idx))?;
            debark_time(stop_time).ok_or_else(|| (BadStopTime::DebarkTime, stop_time_idx))?;
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
