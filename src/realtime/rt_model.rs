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

use crate::chrono::Duration;
use crate::realtime::rt_model::RealTimeUpdate::{LineUpdate, NetworkUpdate, RouteUpdate};
use crate::realtime::rt_model::UpdateType::Delete;
use crate::timetables::{RemovalError, Timetables as TimetablesTrait, TimetablesIter};
use crate::transit_model::{
    model::GetCorresponding,
    objects::{Line, Network, Route, StopPoint, VehicleJourney},
    Model,
};
use crate::{DataUpdate, NaiveDateTime, TransitData};
use chrono::NaiveDate;
use relational_types::IdxSet;
use std::cmp::{max, min};
use std::error::Error;
use std::fmt::Debug;
use std::mem;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub enum SeverityEffect {
    NoService,
    ReducedService,
    SignificantDelay,
    Detour,
    AdditionalService,
    ModifiedService,
    OtherEffect,
    UnknownEffect,
    StopMoved,
}

#[derive(Debug)]
pub struct UpdateInfo {
    pub disruption_id: String,
    pub id: String,
}

#[derive(Debug)]
pub struct DeleteInfo {
    pub disruption_id: String,
    pub pt_object_id: String,
    pub severity_effect: SeverityEffect,
    pub application_periods: Vec<DateTimePeriod>,
}

#[derive(Debug)]
pub enum UpdateType {
    Delete(DeleteInfo),
    Update(UpdateInfo),
}

#[derive(Debug)]
pub enum RealTimeUpdate {
    VehicleUpdate(UpdateType),
    RouteUpdate(UpdateType),
    LineUpdate(UpdateType),
    NetworkUpdate(UpdateType),
    LineSectionUpdate(UpdateType),
    RailSectionUpdate(UpdateType),
}

#[derive(Default)]
pub struct RealTimeModel {
    new_updates: Vec<RealTimeUpdate>,
    applied_updated: Vec<RealTimeUpdate>,
}

impl RealTimeModel {
    pub fn add_trip_update(&mut self, trip_update: Vec<RealTimeUpdate>) {
        self.new_updates.extend(trip_update)
    }

    pub fn update_data<Timetables>(&self, model: &Model, data: &mut TransitData<Timetables>)
    where
        Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
        Timetables::Mission: 'static,
        Timetables::Position: 'static,
    {
        for trip_update in self.new_updates.iter() {
            match trip_update {
                RouteUpdate(Delete(info)) => {
                    Self::delete_route(info, model, data);
                }
                LineUpdate(Delete(info)) => {
                    Self::delete_line(info, model, data);
                }
                NetworkUpdate(Delete(info)) => {
                    Self::delete_network(info, model, data);
                }
                _ => (),
            }
        }
    }

    fn delete_route<Timetables>(
        update_info: &DeleteInfo,
        model: &Model,
        data: &mut TransitData<Timetables>,
    ) where
        Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
        Timetables::Mission: 'static,
        Timetables::Position: 'static,
    {
        let (start, end) = model
            .calculate_validity_period()
            .expect("Invalid Validity period");
        let validity_period = DateTimePeriod {
            start: start.and_hms(0, 0, 0),
            end: end.and_hms(0, 0, 0),
        };

        if let SeverityEffect::NoService = update_info.severity_effect {
            if model.routes.contains_id(&update_info.pt_object_id) {
                for (vj_idx, vj) in model.vehicle_journeys.iter() {
                    if vj.route_id == update_info.pt_object_id {
                        for period in update_info.application_periods.iter() {
                            let clamp = clamp_date(&validity_period, period);
                            if let Some(clamp) = clamp {
                                for day in clamp.into_iter() {
                                    data.remove_vehicle(&vj_idx, &day.date());
                                }
                            }
                        }
                    }
                }
            } else {
                warn!("Route id:{} not found", update_info.pt_object_id);
            }
        } else {
            info!("Disruption has no effect on data");
        }
    }

    fn delete_line<Timetables>(
        update_info: &DeleteInfo,
        model: &Model,
        data: &mut TransitData<Timetables>,
    ) where
        Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
        Timetables::Mission: 'static,
        Timetables::Position: 'static,
    {
        if model.lines.contains_id(&update_info.pt_object_id) {
            for (_, route) in model.routes.iter() {
                if route.line_id == update_info.pt_object_id {
                    let delete_info = DeleteInfo {
                        disruption_id: update_info.disruption_id.clone(),
                        pt_object_id: route.id.clone(),
                        severity_effect: update_info.severity_effect.clone(),
                        application_periods: update_info.application_periods.clone(),
                    };
                    Self::delete_route(&delete_info, model, data);
                }
            }
        } else {
            warn!("Line not found");
        }
    }

    fn delete_network<Timetables>(
        update_info: &DeleteInfo,
        model: &Model,
        data: &mut TransitData<Timetables>,
    ) where
        Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
        Timetables::Mission: 'static,
        Timetables::Position: 'static,
    {
        if model.networks.contains_id(&update_info.pt_object_id) {
            for (_, line) in model.lines.iter() {
                if line.network_id == update_info.pt_object_id {
                    let delete_info = DeleteInfo {
                        disruption_id: update_info.disruption_id.clone(),
                        pt_object_id: line.id.clone(),
                        severity_effect: update_info.severity_effect.clone(),
                        application_periods: update_info.application_periods.clone(),
                    };
                    Self::delete_line(&delete_info, model, data);
                }
            }
        } else {
            warn!("Network not found");
        }
    }
}

#[derive(Debug, Clone)]
pub struct DateTimePeriod {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

fn intersection(lhs: &DateTimePeriod, rhs: &DateTimePeriod) -> Option<DateTimePeriod> {
    let start = min(lhs.start, rhs.start);
    let end = min(lhs.end, rhs.end);
    if start < end {
        Some(DateTimePeriod { start, end })
    } else {
        None
    }
}

fn clamp<T: PartialOrd<T>>(input: T, min_input: T, max_input: T) -> T
where
    T: std::cmp::Ord,
{
    min(max(input, min_input), max_input)
}

fn clamp_date(input: &DateTimePeriod, clamper: &DateTimePeriod) -> Option<DateTimePeriod> {
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
