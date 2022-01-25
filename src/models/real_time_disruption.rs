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

use crate::{time::SecondsSinceTimezonedDayStart, timetables::FlowDirection};
use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use serde::Deserialize;
use std::{
    cmp::{max, min},
    fmt::{Debug, Display},
    mem,
};

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
    pub cause: Cause,
    pub tags: Vec<Tag>,
    pub properties: Vec<DisruptionProperty>,
    pub impacts: Vec<Impact>,
}

#[derive(Debug, Clone)]
pub enum DisruptionError {
    NetworkAbsent(NetworkId),
    LineAbsent(LineId),
    RouteAbsent(RouteId),
    VehicleJourneyAbsent(VehicleJourneyId),
    TripAbsent(VehicleJourneyId, NaiveDate),
}

#[derive(Default, Debug, Clone)]
pub struct Cause {
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
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Severity {
    pub wording: Option<String>,
    pub color: Option<String>,
    pub priority: Option<i32>,
    pub effect: Effect,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub text: String,
    pub channel_id: Option<String>,
    pub channel_name: String,
    pub channel_content_type: Option<String>,
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
    pub updated_at: NaiveDateTime,
    pub application_periods: Vec<DateTimePeriod>,
    pub application_patterns: Vec<ApplicationPattern>,
    pub severity: Severity,
    pub messages: Vec<Message>,
    pub impacted_pt_objects: Vec<Impacted>,
    pub informed_pt_objects: Vec<Informed>,
}

#[derive(Debug, Clone)]
pub enum Impacted {
    NetworkDeleted(NetworkId),
    LineDeleted(LineId),
    RouteDeleted(RouteId),
    TripDeleted(VehicleJourneyId),
    BaseTripUpdated(TripDisruption),
    NewTripUpdated(TripDisruption),
    RailSection(RailSectionDisruption),
    LineSection(LineSectionDisruption),
    StopAreaDeleted(StopAreaId),
    StopPointDeleted(StopPointId),
}

#[derive(Debug, Clone)]
pub enum Informed {
    Network(NetworkId),
    Line(LineId),
    Route(RouteId),
    Trip(VehicleJourneyId),
    StopArea(StopAreaId),
    StopPoint(StopPointId),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct NetworkId {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct LineId {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct RouteId {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct StopPointId {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct StopAreaId {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct VehicleJourneyId {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct TripDisruption {
    pub trip_id: VehicleJourneyId,
    pub stop_times: Vec<StopTime>,
    pub company_id: Option<String>,
    pub physical_mode_id: Option<String>,
    pub headsign: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LineSectionDisruption {
    pub line: LineId,
    pub start: StopAreaId,
    pub end: StopAreaId,
    pub routes: Vec<RouteId>,
}

#[derive(Debug, Clone)]
pub struct RailSectionDisruption {
    pub line: LineId,
    pub start: StopAreaId,
    pub end: StopAreaId,
    pub routes: Vec<RouteId>,
    pub blocked_stop_area: Vec<BlockedStopArea>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BlockedStopArea {
    pub id: String,
    pub order: u32,
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
        <Self as Debug>::fmt(self, f)
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
