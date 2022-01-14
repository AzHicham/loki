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
    models::{
        base_model::{BaseModel, BaseVehicleJourneyIdx},
        RealTimeModel,
    },
    time::SecondsSinceTimezonedDayStart,
    timetables::FlowDirection,
};
use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use std::{
    cmp::{max, min},
    fmt::Debug,
    mem,
};
use tracing::error;

#[derive(Debug, Clone)]
pub struct Disruption {
    pub id: String,
    pub updates: Vec<Update>,
}

#[derive(Debug, Clone)]
pub enum Update {
    Delete(Trip),
    Add(Trip, Vec<StopTime>),
    Modify(Trip, Vec<StopTime>),
    NoEffect,
}

#[derive(Debug, Clone)]
pub struct Trip {
    pub vehicle_journey_id: String,
    pub reference_date: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct StopTime {
    pub stop_id: String,
    pub arrival_time: SecondsSinceTimezonedDayStart,
    pub departure_time: SecondsSinceTimezonedDayStart,
    pub flow_direction: FlowDirection,
}

#[derive(Debug, Clone)]
pub struct Disrupt {
    pub id: String,
    pub reference: Option<String>,
    pub contributor: String,
    pub publication_period: DateTimePeriod,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub cause: Cause,
    // localization ??
    pub tags: Vec<Tag>,
    pub impacts: Vec<Impact>,
}

impl Disrupt {
    pub fn trip_update_iter(
        &self,
        base_model: &BaseModel,
        realtime_model: &RealTimeModel,
    ) -> Vec<Update> {
        let mut updates = vec![];
        for impact in &self.impacts {
            let update = read_impact(impact, base_model, realtime_model);
            updates.extend(update);
        }
        updates
    }
}

fn read_impact(
    impact: &Impact,
    base_model: &BaseModel,
    realtime_model: &RealTimeModel,
) -> Vec<Update> {
    let mut updates = vec![];
    for pt_object in &impact.pt_objects {
        let update = match pt_object {
            PtObject::Network(network) => handle_network_update(
                &network.id,
                impact.severity.effect,
                &impact.application_periods,
                base_model,
                realtime_model,
            ),
            PtObject::Line(line) => handle_line_update(
                &line.id,
                impact.severity.effect,
                &impact.application_periods,
                base_model,
                realtime_model,
            ),
            PtObject::Route(route) => handle_route_update(
                &route.id,
                impact.severity.effect,
                &impact.application_periods,
                base_model,
                realtime_model,
            ),
            PtObject::Trip_(trip) => handle_trip_update(
                &trip.id,
                impact.severity.effect,
                &impact.application_periods,
                base_model,
                realtime_model,
                &trip.stop_times,
            ),
            _ => vec![],
        };
        updates.extend(update);
    }
    updates
}

fn handle_trip_update(
    trip_id: &str,
    effect: Effect,
    application_periods: &[DateTimePeriod],
    _base_model: &BaseModel,
    realtime_model: &RealTimeModel,
    stop_times: &Option<Vec<StopTime>>,
) -> Vec<Update> {
    use Effect::*;
    let f = |reference_date: NaiveDateTime| match effect {
        NoService => Update::Delete(Trip {
            vehicle_journey_id: trip_id.to_string(),
            reference_date: reference_date.date(),
        }),
        AdditionalService => {
            let trip_exists_in_realtime =
                realtime_model.contains_new_vehicle_journey(trip_id, &reference_date.date());
            let trip = Trip {
                vehicle_journey_id: trip_id.to_string(),
                reference_date: reference_date.date(),
            };
            if trip_exists_in_realtime {
                Update::Modify(trip, stop_times.clone().unwrap())
            } else {
                Update::Add(trip, stop_times.clone().unwrap())
            }
        }
        ReducedService | SignificantDelays | Detour | ModifiedService => Update::Modify(
            Trip {
                vehicle_journey_id: trip_id.to_string(),
                reference_date: reference_date.date(),
            },
            stop_times.clone().unwrap(),
        ),
        OtherEffect | UnknownEffect | StopMoved => Update::NoEffect,
    };

    application_periods
        .iter()
        .flatten()
        .map(f)
        .map(|r| {
            println!("*********{:?}", r);
            r
        })
        .collect()
}

fn handle_route_update(
    route_id: &str,
    effect: Effect,
    application_periods: &[DateTimePeriod],
    base_model: &BaseModel,
    realtime_model: &RealTimeModel,
) -> Vec<Update> {
    use Effect::*;
    let f = |vj_idx: BaseVehicleJourneyIdx| match effect {
        NoService => handle_trip_update(
            base_model.vehicle_journey_name(vj_idx),
            effect,
            application_periods,
            base_model,
            realtime_model,
            &None,
        ),
        _ => vec![Update::NoEffect],
    };
    if base_model.contains_route_id(route_id) {
        base_model
            .vehicle_journeys()
            .filter(|vj_idx| base_model.route_name(*vj_idx) == route_id)
            .map(f)
            .flatten()
            .collect()
    } else {
        error!("route.uri {} does not exists in BaseModel", route_id);
        vec![]
    }
}

fn handle_line_update(
    line_id: &str,
    effect: Effect,
    application_periods: &[DateTimePeriod],
    base_model: &BaseModel,
    realtime_model: &RealTimeModel,
) -> Vec<Update> {
    use Effect::*;
    let f = |vj_idx: BaseVehicleJourneyIdx| match effect {
        NoService => handle_trip_update(
            base_model.vehicle_journey_name(vj_idx),
            effect,
            application_periods,
            base_model,
            realtime_model,
            &None,
        ),
        _ => vec![Update::NoEffect],
    };
    if base_model.contains_line_id(line_id) {
        base_model
            .vehicle_journeys()
            .filter(|vj_idx| base_model.line_name(*vj_idx) == Some(line_id))
            .map(f)
            .flatten()
            .collect()
    } else {
        error!("line.uri {} does not exists in BaseModel", line_id);
        vec![]
    }
}

fn handle_network_update(
    network_id: &str,
    effect: Effect,
    application_periods: &[DateTimePeriod],
    base_model: &BaseModel,
    realtime_model: &RealTimeModel,
) -> Vec<Update> {
    use Effect::*;
    let f = |vj_idx: BaseVehicleJourneyIdx| match effect {
        NoService => handle_trip_update(
            base_model.vehicle_journey_name(vj_idx),
            effect,
            application_periods,
            base_model,
            realtime_model,
            &None,
        ),
        _ => vec![Update::NoEffect],
    };
    if base_model.contains_network_id(network_id) {
        base_model
            .vehicle_journeys()
            .filter(|vj_idx| base_model.network_name(*vj_idx) == Some(network_id))
            .map(f)
            .flatten()
            .collect()
    } else {
        error!("network.uri {} does not exists in BaseModel", network_id);
        vec![]
    }
}

#[derive(Default, Debug, Clone)]
pub struct Cause {
    pub id: String,
    pub wording: String,
    pub category: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Default, Debug, Clone)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct Severity {
    pub id: String,
    pub wording: String,
    pub color: String,
    pub priority: u32,
    pub effect: Effect,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub text: String,
    pub channel_id: String,
    pub channel_name: String,
    pub channel_content_type: String,
    pub channel_types: Vec<ChannelType>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct ApplicationPattern {
    pub begin_date: NaiveDate,
    pub end_date: NaiveDate,
    pub time_slots: Vec<TimeSlot>,
}

#[derive(Debug, Clone)]
pub struct TimeSlot {
    pub begin: NaiveTime,
    pub end: NaiveTime,
}

#[derive(Debug, Clone)]
pub struct Impact {
    pub id: String,
    pub company_id: Option<String>,
    pub physical_mode_id: Option<String>,
    pub headsign: Option<String>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub application_periods: Vec<DateTimePeriod>,
    pub application_patterns: Vec<ApplicationPattern>,
    pub severity: Severity,
    pub messages: Vec<Message>,
    pub pt_objects: Vec<PtObject>,
}

#[derive(Debug, Clone)]
pub enum PtObject {
    Network(Network),
    Line(Line),
    Route(Route),
    Trip_(Trip_),
    RailSection(RailSection),
    LineSection(LineSection),
    StopArea(StopArea),
    StopPoint(StopPoint),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Network {
    pub id: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub id: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct Route {
    pub id: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct Trip_ {
    pub id: String,
    pub stop_times: Option<Vec<StopTime>>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct LineSection {
    pub line: Line,
    pub start_sa: StopArea,
    pub stop_sa: StopArea,
    pub routes: Vec<Route>,
}

#[derive(Debug, Clone)]
pub struct RailSection {
    pub id: String,
    pub line_id: String,
    pub start_id: String,
    pub end_id: String,
    pub route_id: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct StopPoint {
    pub id: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct StopArea {
    pub id: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Copy)]
pub enum Effect {
    NoService,
    ReducedService,
    SignificantDelays,
    Detour,
    AdditionalService,
    ModifiedService,
    OtherEffect,
    UnknownEffect,
    StopMoved,
}

#[derive(Debug, Clone)]
pub enum ChannelType {
    Web,
    Sms,
    Email,
    Mobile,
    Notification,
    Twitter,
    Facebook,
    UnknownType,
    Title,
    Beacon,
}

#[derive(Debug, Clone)]
pub struct DateTimePeriod {
    start: NaiveDateTime,
    end: NaiveDateTime,
}

impl DateTimePeriod {
    pub fn new(
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) -> Result<DateTimePeriod, DateTimePeriodError> {
        if start <= end {
            Ok(DateTimePeriod { start, end })
        } else {
            Err(DateTimePeriodError::DateTimePeriodError(start, end))
        }
    }

    pub fn start(&self) -> NaiveDateTime {
        self.start
    }

    pub fn end(&self) -> NaiveDateTime {
        self.end
    }
}

pub fn intersection(lhs: &DateTimePeriod, rhs: &DateTimePeriod) -> Option<DateTimePeriod> {
    DateTimePeriod::new(max(lhs.start, rhs.start), min(lhs.end, rhs.end)).ok()
}

pub enum DateTimePeriodError {
    DateTimePeriodError(NaiveDateTime, NaiveDateTime),
}

impl Debug for DateTimePeriodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DateTimePeriodError::DateTimePeriodError(start, end) => {
                write!(f, "Error DateTimePeriod, start must be less or equal to end, start : {}, end : {}",
                       start,
                       end)
            }
        }
    }
}

pub struct DateTimePeriodIterator<'a> {
    period: &'a DateTimePeriod,
    current: NaiveDateTime,
}

impl<'a> Iterator for DateTimePeriodIterator<'a> {
    type Item = NaiveDateTime;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current <= self.period.end {
            let next = self.current + Duration::days(1);
            Some(mem::replace(&mut self.current, next))
        } else {
            None
        }
    }
}

impl<'a> IntoIterator for &'a DateTimePeriod {
    type Item = NaiveDateTime;
    type IntoIter = DateTimePeriodIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        DateTimePeriodIterator {
            period: self,
            current: self.start,
        }
    }
}

pub fn ts_to_dt(timestamp: u64) -> Option<NaiveDateTime> {
    let timestamp = i64::try_from(timestamp);
    if let Ok(timestamp) = timestamp {
        match timestamp {
            0 => None,
            _ => Some(NaiveDateTime::from_timestamp(timestamp, 0)),
        }
    } else {
        None
    }
}
