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

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use std::cmp::{max, min};
use std::mem;

use crate::{time::SecondsSinceTimezonedDayStart, timetables::FlowDirection};

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

#[derive(Debug, Clone)]
pub struct Cause {
    pub id: String,
    pub wording: String,
    pub category: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
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
    pub company_id: String,
    pub physical_mode_id: String,
    pub headsign: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub application_periods: Vec<DateTimePeriod>,
    pub application_patterns: Vec<ApplicationPattern>,
    pub severity: Severity,
    pub messages: Vec<Message>,
    pub vehicle_info: Option<Vec<StopTime>>,
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
    pub is: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub id: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct Route {
    pub id: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct Trip_ {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct LineSection {
    pub line_id: String,
    pub start_id: String,
    pub end_id: String,
    pub route_id: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct RailSection {
    pub id: String,
    pub line_id: String,
    pub start_id: String,
    pub end_id: String,
    pub route_id: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct StopPoint {
    pub id: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct StopArea {
    pub id: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
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
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

fn clamp<T: PartialOrd<T>>(input: T, min_input: T, max_input: T) -> T
where
    T: std::cmp::Ord,
{
    min(max(input, min_input), max_input)
}

pub fn clamp_date(input: &DateTimePeriod, clamper: &DateTimePeriod) -> Option<DateTimePeriod> {
    let start = clamp(input.start, clamper.start, clamper.end);
    let end = clamp(input.end, clamper.start, clamper.end);
    if start <= end {
        Some(DateTimePeriod { start, end })
    } else {
        None
    }
}

pub struct DateTimePeriodIterator<'a> {
    period: &'a DateTimePeriod,
    current: NaiveDateTime,
}

impl<'a> Iterator for DateTimePeriodIterator<'a> {
    type Item = NaiveDateTime;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.period.end {
            let next = min(self.current + Duration::days(1), self.period.end);
            Some(mem::replace(&mut self.current, next))
        } else if self.current == self.period.end {
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
