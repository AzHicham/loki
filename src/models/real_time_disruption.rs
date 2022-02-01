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
    time::{SecondsSinceTimezonedDayStart, MAX_SECONDS_IN_UTC_DAY},
    timetables::FlowDirection,
};
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
    pub publication_period: TimePeriod,
    pub cause: Cause,
    pub tags: Vec<Tag>,
    pub properties: Vec<DisruptionProperty>,
    pub impacts: Vec<Impact>,
}

#[derive(Debug, Clone)]
pub enum DisruptionError {
    StopPointAbsent(StopPointId),
    StopAreaAbsent(StopAreaId),
    NetworkAbsent(NetworkId),
    LineAbsent(LineId),
    RouteAbsent(RouteId),
    VehicleJourneyAbsent(VehicleJourneyId),
    DeleteAbsentTrip(VehicleJourneyId, NaiveDate),
    ModifyAbsentTrip(VehicleJourneyId, NaiveDate),
    AddPresentTrip(VehicleJourneyId, NaiveDate),
    NewTripWithBaseId(VehicleJourneyId, NaiveDate),
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
    pub week_pattern: [bool; 7],
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
    pub application_periods: Vec<TimePeriod>,
    pub application_patterns: Vec<ApplicationPattern>,
    pub severity: Severity,
    pub messages: Vec<Message>,
    pub impacted_pt_objects: Vec<Impacted>,
    pub informed_pt_objects: Vec<Informed>,
}

#[derive(Debug, Clone)]
pub enum Impacted {
    // chaos
    NetworkDeleted(NetworkId),
    LineDeleted(LineId),
    RouteDeleted(RouteId),

    RailSection(RailSectionDisruption),
    LineSection(LineSectionDisruption),
    StopAreaDeleted(StopAreaId),
    StopPointDeleted(StopPointId),
    // delete from chaos
    BaseTripDeleted(VehicleJourneyId),

    //Kirin
    TripDeleted(VehicleJourneyId, NaiveDate),
    BaseTripUpdated(TripDisruption),
    NewTripUpdated(TripDisruption),
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
    pub trip_date: NaiveDate,
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

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Clone, Copy)]
pub enum Effect {
    // DO NOT change the order of effects !!
    // Effects are ordered from the least to the worst impact
    StopMoved,
    UnknownEffect,
    OtherEffect,
    ModifiedService,
    AdditionalService,
    Detour,
    SignificantDelays,
    ReducedService,
    NoService,
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

/// An half open interval of time.
/// A instant `t` is contained in it
/// if and only if
///  `start <= t < end`
///
#[derive(Debug, Clone)]
pub struct TimePeriod {
    start: NaiveDateTime,
    end: NaiveDateTime,
}

impl TimePeriod {
    pub fn new(start: NaiveDateTime, end: NaiveDateTime) -> Result<TimePeriod, TimePeriodError> {
        if start < end {
            Ok(TimePeriod { start, end })
        } else {
            Err(TimePeriodError::StartAfterEnd(start, end))
        }
    }

    pub fn start(&self) -> NaiveDateTime {
        self.start
    }

    pub fn end(&self) -> NaiveDateTime {
        self.end
    }

    pub fn contains(&self, t: &NaiveDateTime) -> bool {
        self.start <= *t && *t < self.end
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.contains(&other.start) || other.contains(&self.start)
    }

    // Returns an iterator that contains all dates D such that
    //  a vehicle_journey on D is "concerned" by this time_period,
    //  where "concerned" means that a stop_time of the vehicle_journey
    //   circulating on date D is contained in this time_period
    //
    // Note that the iterator may contains dates for which a vehicle
    // journey is *NOT* concerned. The caller should check by himself.
    pub fn dates_possibly_concerned(&self) -> DateIter {
        // since the vehicle journey stop_times are given in local time
        // and we accept values up to 48h, we use a 3 days offset
        // that account for both
        let offset = Duration::seconds(i64::from(MAX_SECONDS_IN_UTC_DAY));
        let first_date = (self.start - offset).date();
        let last_date = (self.end + offset).date();
        DateIter {
            current_date: first_date,
            last_date,
        }
    }
}

pub struct TimePeriods<'a> {
    periods: &'a [TimePeriod],
}

impl<'a> TimePeriods<'a> {
    pub fn new(periods: &'a [TimePeriod]) -> Option<Self> {
        if periods.is_empty() {
            None
        } else {
            Some(Self { periods })
        }
    }

    pub fn contains(&self, t: &NaiveDateTime) -> bool {
        for period in self.periods {
            if period.contains(t) {
                return true;
            }
        }
        false
    }

    pub fn intersects(&self, other: &TimePeriod) -> bool {
        for period in self.periods {
            if period.intersects(other) {
                return true;
            }
        }
        false
    }

    // Returns an iterator that contains all dates D such that
    //  a vehicle_journey on D is "concerned" by this time_periods,
    //  where "concerned" means that a stop_time of the vehicle_journey
    //   circulating on date D is contained in this time_periods
    //
    // Note that the iterator may contains dates for which a vehicle
    // journey is *NOT* concerned. The caller should check by himself.
    pub fn dates_possibly_concerned(&self) -> DateIter {
        let earliest_datetime = self
            .periods
            .iter()
            .map(|period| period.start)
            .min()
            .unwrap(); // unwrap safe here because we check in new() that ! periods.is_empty()

        let latest_datetime = self.periods.iter().map(|period| period.end).max().unwrap(); // unwrap safe here because we check in new() that ! periods.is_empty()

        // since the vehicle journey stop_times are given in local time
        // and we accept values up to 48h, we use a 3 days offset
        // that account for both
        let offset = Duration::seconds(i64::from(MAX_SECONDS_IN_UTC_DAY));
        let first_date = (earliest_datetime - offset).date();
        let last_date = (latest_datetime + offset).date();
        DateIter {
            current_date: first_date,
            last_date,
        }
    }
}

pub fn intersection(lhs: &TimePeriod, rhs: &TimePeriod) -> Option<TimePeriod> {
    TimePeriod::new(max(lhs.start, rhs.start), min(lhs.end, rhs.end)).ok()
}

pub enum TimePeriodError {
    StartAfterEnd(NaiveDateTime, NaiveDateTime),
}

impl std::error::Error for TimePeriodError {}

impl Display for TimePeriodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}

impl Debug for TimePeriodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimePeriodError::StartAfterEnd(start, end) => {
                write!(
                    f,
                    "Bad TimePeriod, start {} must be strictly greater than end {}",
                    start, end
                )
            }
        }
    }
}

pub struct DateTimePeriodIterator<'a> {
    period: &'a TimePeriod,
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

impl<'a> IntoIterator for &'a TimePeriod {
    type Item = NaiveDateTime;
    type IntoIter = DateTimePeriodIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        DateTimePeriodIterator {
            period: self,
            current: self.start,
        }
    }
}

// Yields all dates between current_date (included)
// and last_date (also included)
pub struct DateIter {
    current_date: NaiveDate,
    last_date: NaiveDate,
}

impl Iterator for DateIter {
    type Item = NaiveDate;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_date <= self.last_date {
            let result = self.current_date;
            self.current_date = self.current_date.succ();
            Some(result)
        } else {
            None
        }
    }
}
