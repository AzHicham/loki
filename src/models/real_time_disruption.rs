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
    models::{base_model::BaseModel, RealTimeModel},
    time::SecondsSinceTimezonedDayStart,
    timetables::FlowDirection,
};
use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use std::{
    cmp::{max, min},
    fmt::{Debug, Display},
    mem,
};
use tracing::error;

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
pub struct Disruption {
    pub id: String,
    pub reference: Option<String>,
    pub contributor: Option<String>,
    pub publication_period: DateTimePeriod,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub cause: Cause,
    // localization ??
    pub tags: Vec<Tag>,
    pub properties: Vec<DisruptionProperty>,
    pub impacts: Vec<Impact>,
}

#[derive(Debug, Clone)]
pub enum DisruptionError {
    NetworkAbsentInModel(String),
    LineAbsentInModel(String),
    RouteAbsentInModel(String),
    TripAbsentInModel(String),
    UnhandledImpact,
}

impl Disruption {
    pub fn get_updates(
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

    let validity_period = base_model.validity_period();
    let calendar_period = DateTimePeriod::new(
        validity_period.0.and_hms(0, 0, 0),
        validity_period.1.and_hms(12, 59, 59),
    );

    if let Ok(calendar_period) = calendar_period {
        let application_periods: Vec<DateTimePeriod> = impact
            .application_periods
            .iter()
            .filter_map(|ap| intersection(ap, &calendar_period))
            .collect();

        for pt_object in &impact.impacted_pt_objects {
            let update = match pt_object {
                Impacted::NetworkDeleted(network) => {
                    delete_network(&network.id, &application_periods, base_model)
                }
                Impacted::LineDeleted(line) => {
                    delete_line(&line.id, &application_periods, base_model)
                }
                Impacted::RouteDeleted(route) => {
                    delete_route(&route.id, &application_periods, base_model)
                }
                Impacted::TripDeleted(trip) => delete_trip(&trip.id, &application_periods),
                Impacted::TripModified(trip) => update_trip(
                    &trip.id,
                    impact.severity.effect,
                    &application_periods,
                    base_model,
                    realtime_model,
                    trip.stop_times.clone(),
                ),
                _ => Err(DisruptionError::UnhandledImpact),
            };
            match update {
                Err(err) => error!("Error occurred while creating real time update. {:?}", err),
                Ok(update) => updates.extend(update),
            }
        }
    }
    updates
}

fn delete_trip(
    trip_id: &str,
    application_periods: &[DateTimePeriod],
) -> Result<Vec<Update>, DisruptionError> {
    Ok(application_periods
        .iter()
        .flatten()
        .map(|dt| {
            Update::Delete(Trip {
                vehicle_journey_id: trip_id.to_string(),
                reference_date: dt.date(),
            })
        })
        .collect())
}

fn update_trip(
    trip_id: &str,
    effect: Effect,
    application_periods: &[DateTimePeriod],
    base_model: &BaseModel,
    realtime_model: &RealTimeModel,
    stop_times: Vec<StopTime>,
) -> Result<Vec<Update>, DisruptionError> {
    use Effect::*;
    let f = |reference_date: NaiveDateTime| match effect {
        AdditionalService => {
            let trip = Trip {
                vehicle_journey_id: trip_id.to_string(),
                reference_date: reference_date.date(),
            };
            let trip_exists_in_base = {
                let has_vj_idx = base_model.vehicle_journey_idx(&trip.vehicle_journey_id);
                match has_vj_idx {
                    None => false,
                    Some(vj_idx) => base_model.trip_exists(vj_idx, trip.reference_date),
                }
            };
            if trip_exists_in_base {
                return Err(DisruptionError::TripAbsentInModel(format!(
                    "Additional service for trip {:?} that exists in the base schedule.",
                    trip
                )));
            }
            let trip_exists_in_realtime = realtime_model.is_present(&trip, base_model);
            if trip_exists_in_realtime {
                Ok(Update::Modify(trip, stop_times.clone()))
            } else {
                Ok(Update::Add(trip, stop_times.clone()))
            }
        }
        ReducedService | SignificantDelays | Detour | ModifiedService | OtherEffect
        | UnknownEffect => {
            let trip = Trip {
                vehicle_journey_id: trip_id.to_string(),
                reference_date: reference_date.date(),
            };
            // the trip should exists in the base schedule
            // for these effects
            if let Some(base_vj_idx) = base_model.vehicle_journey_idx(&trip.vehicle_journey_id) {
                if !base_model.trip_exists(base_vj_idx, trip.reference_date) {
                    return Err(DisruptionError::TripAbsentInModel(format!(
                            "Kirin effect {:?} on vehicle {} on day {} cannot be applied since this base schedule vehicle is not valid on the day.",
                            effect,
                            trip.vehicle_journey_id,
                            trip.reference_date
                        )));
                }
            } else {
                return Err(DisruptionError::TripAbsentInModel(format!(
                        "Kirin effect {:?} on vehicle {} cannot be applied since this vehicle does not exists in the base schedule.",
                        effect,
                        trip.vehicle_journey_id
                    )));
            }

            let trip_is_present = realtime_model.is_present(&trip, base_model);
            if trip_is_present {
                Ok(Update::Modify(trip, stop_times.clone()))
            } else {
                Ok(Update::Add(trip, stop_times.clone()))
            }
        }
        StopMoved | NoService => Ok(Update::NoEffect),
    };

    application_periods.iter().flatten().map(f).collect()
}

fn delete_route(
    route_id: &str,
    application_periods: &[DateTimePeriod],
    base_model: &BaseModel,
) -> Result<Vec<Update>, DisruptionError> {
    if base_model.contains_route_id(route_id) {
        Ok(base_model
            .vehicle_journeys()
            .filter(|vj_idx| base_model.route_name(*vj_idx) == route_id)
            .filter_map(|vj_idx| {
                delete_trip(base_model.vehicle_journey_name(vj_idx), application_periods).ok()
            })
            .flatten()
            .collect())
    } else {
        Err(DisruptionError::RouteAbsentInModel(format!(
            "route.uri {} does not exists in BaseModel",
            route_id
        )))
    }
}

fn delete_line(
    line_id: &str,
    application_periods: &[DateTimePeriod],
    base_model: &BaseModel,
) -> Result<Vec<Update>, DisruptionError> {
    if base_model.contains_line_id(line_id) {
        Ok(base_model
            .vehicle_journeys()
            .filter(|vj_idx| base_model.line_name(*vj_idx) == Some(line_id))
            .filter_map(|vj_idx| {
                delete_trip(base_model.vehicle_journey_name(vj_idx), application_periods).ok()
            })
            .flatten()
            .collect())
    } else {
        Err(DisruptionError::LineAbsentInModel(format!(
            "line.uri {} does not exists in BaseModel",
            line_id
        )))
    }
}

fn delete_network(
    network_id: &str,
    application_periods: &[DateTimePeriod],
    base_model: &BaseModel,
) -> Result<Vec<Update>, DisruptionError> {
    if base_model.contains_network_id(network_id) {
        Ok(base_model
            .vehicle_journeys()
            .filter(|vj_idx| base_model.network_name(*vj_idx) == Some(network_id))
            .filter_map(|vj_idx| {
                delete_trip(base_model.vehicle_journey_name(vj_idx), application_periods).ok()
            })
            .flatten()
            .collect())
    } else {
        Err(DisruptionError::NetworkAbsentInModel(format!(
            "network.uri {} does not exists in BaseModel",
            network_id
        )))
    }
}

#[derive(Default, Debug, Clone)]
pub struct Cause {
    pub id: String,
    pub wording: String,
    pub category: String,
}

#[derive(Default, Debug, Clone)]
pub struct DisruptionProperty {
    pub key: String,
    pub type_: String,
    pub value: String,
}

#[derive(Default, Debug, Clone)]
pub struct Tag {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Severity {
    pub id: String,
    pub wording: Option<String>,
    pub color: Option<String>,
    pub priority: Option<i32>,
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
}

#[derive(Debug, Clone)]
pub struct ApplicationPattern {
    pub begin_date: NaiveDate,
    pub end_date: NaiveDate,
    pub time_slots: Vec<TimeSlot>,
}

#[derive(Debug, Clone)]
pub struct TimeSlot {
    //TODO : determine in which timezone are these ?
    // can we use SecondsTimezoneDayStart/SecondsSinceUtcDayStart ?
    pub begin: NaiveTime,
    pub end: NaiveTime,
}

#[derive(Debug, Clone)]
pub struct Impact {
    pub id: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub application_periods: Vec<DateTimePeriod>,
    pub application_patterns: Vec<ApplicationPattern>,
    pub severity: Severity,
    pub messages: Vec<Message>,
    pub impacted_pt_objects: Vec<Impacted>,
    pub informed_pt_objects: Vec<Informed>,
}

pub enum PtObjectType {
    Impacted(Impacted),
    Informed(Informed),
}

#[derive(Debug, Clone)]
pub enum Impacted {
    NetworkDeleted(NetworkDisruption),
    LineDeleted(LineDisruption),
    RouteDeleted(RouteDisruption),
    TripDeleted(TripDisruption),
    TripModified(TripDisruption),
    RailSection(RailSectionDisruption),
    LineSection(LineSectionDisruption),
    StopAreaDeleted(StopAreaDisruption),
    StopPointDeleted(StopPointDisruption),
}

#[derive(Debug, Clone)]
pub enum Informed {
    Network(NetworkDisruption),
    Line(LineDisruption),
    Route(RouteDisruption),
    Trip(TripDisruption),
    StopArea(StopAreaDisruption),
    StopPoint(StopPointDisruption),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct NetworkDisruption {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct LineDisruption {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct RouteDisruption {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct TripDisruption {
    pub id: String,
    pub stop_times: Vec<StopTime>,
    pub company_id: Option<String>,
    pub physical_mode_id: Option<String>,
    pub headsign: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LineSectionDisruption {
    pub line: LineDisruption,
    pub start_sa: StopAreaDisruption,
    pub stop_sa: StopAreaDisruption,
    pub routes: Vec<RouteDisruption>,
}

#[derive(Debug, Clone)]
pub struct RailSectionDisruption {
    pub id: String,
    pub line_id: String,
    pub start_id: String,
    pub end_id: String,
    pub route_id: String,
}

#[derive(Debug, Clone)]
pub struct StopPointDisruption {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct StopAreaDisruption {
    pub id: String,
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

impl std::error::Error for DateTimePeriodError {}

impl Display for DateTimePeriodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(&self, f)
    }
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
