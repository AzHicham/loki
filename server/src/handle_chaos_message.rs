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

use crate::chaos_proto;
use anyhow::{format_err, Error};
use chaos_proto::gtfs_realtime::TimeRange;
use launch::loki::chrono::{Duration, NaiveDate};
use launch::loki::models::base_model::{BaseModel, BaseVehicleJourneyIdx};
use launch::loki::models::real_time_disruption::{Disruption, Trip, Update};
use launch::loki::tracing::{error, info};
use launch::loki::NaiveDateTime;
use serde_json::to_string;
use std::cmp::{max, min};
use std::mem;

pub fn handle_chaos_protobuf(
    chaos_disruption: &chaos_proto::chaos::Disruption,
    model: &BaseModel,
) -> Result<Disruption, Error> {
    let mut updates: Vec<Update> = vec![];

    for impact in chaos_disruption.get_impacts() {
        if let Ok(update) = read_impact(impact, model) {
            updates.extend(update);
        }
    }

    let result = Disruption {
        id: chaos_disruption.get_id().to_string(),
        updates,
    };
    Ok(result)
}

fn read_impact(
    impact: &chaos_proto::chaos::Impact,
    model: &BaseModel,
) -> Result<Vec<Update>, Error> {
    use chaos_proto::gtfs_realtime::Alert_Effect::*;
    match impact.get_severity().get_effect() {
        NO_SERVICE => Ok(read_pt_object(impact, model)),
        _ => Err(format_err!("Disruption without impact")),
    }
}

fn read_pt_object(impact: &chaos_proto::chaos::Impact, model: &BaseModel) -> Vec<Update> {
    use chaos_proto::chaos::PtObject_Type;
    let validity_period = DatePeriod {
        start: model.validity_period().0,
        end: model.validity_period().1,
    };
    let application_period = compute_application_period(impact, &validity_period);
    let mut updates: Vec<Update> = vec![];

    for entity in impact.get_informed_entities() {
        match entity.get_pt_object_type() {
            PtObject_Type::network => {
                let update = make_delete_network(entity, &application_period, model);
                updates.extend(update);
            }
            PtObject_Type::line => {
                let update = make_delete_line(entity, &application_period, model);
                updates.extend(update);
            }
            PtObject_Type::route => {
                let update = make_delete_route(entity, &application_period, model);
                updates.extend(update);
            }
            PtObject_Type::trip => {
                let update = make_delete_vehicle(entity.get_uri(), &application_period, model);
                updates.extend(update);
            }
            PtObject_Type::stop_area => {}
            PtObject_Type::stop_point => {}
            PtObject_Type::unkown_type => {}
            PtObject_Type::line_section => {}
            PtObject_Type::rail_section => {}
        }
    }
    updates
}

fn compute_application_period(
    impact: &chaos_proto::chaos::Impact,
    model_validity_period: &DatePeriod,
) -> Vec<DatePeriod> {
    impact
        .get_application_periods()
        .iter()
        .filter_map(|range| {
            let period = DatePeriod {
                start: NaiveDateTime::from_timestamp(range.get_start() as i64, 0).date(),
                end: NaiveDateTime::from_timestamp(range.get_end() as i64, 0).date(),
            };
            clamp_date(&period, model_validity_period)
        })
        .collect()
}

fn make_trip(vehicle_journey_id: String, date: NaiveDate) -> Trip {
    Trip {
        vehicle_journey_id,
        reference_date: date,
    }
}

fn make_delete_vehicle(
    vj_id: &str,
    application_periods: &[DatePeriod],
    model: &BaseModel,
) -> Vec<Update> {
    let mut updates = vec![];
    if model.vehicle_journey_idx(vj_id).is_some() {
        updates = application_periods
            .iter()
            .flatten()
            .map(|day| Update::Delete(make_trip(vj_id.to_string(), day)))
            .collect();
    } else {
        error!("vehicule.uri {} does not exists in BaseModel", vj_id);
    }
    updates
}

fn make_delete_route(
    pt_object: &chaos_proto::chaos::PtObject,
    application_periods: &[DatePeriod],
    model: &BaseModel,
) -> Vec<Update> {
    let mut updates: Vec<Update> = vec![];
    if model.contains_route_id(pt_object.get_uri()) {
        updates = model
            .vehicle_journeys()
            .filter(|vj_idx| model.route_name(*vj_idx) == pt_object.get_uri())
            .map(|vj_idx| {
                make_delete_vehicle(
                    model.vehicle_journey_name(vj_idx),
                    application_periods,
                    model,
                )
            })
            .flatten()
            .collect();
    } else {
        error!(
            "route.id: {} does not exists in BaseModel",
            pt_object.get_uri()
        );
    }
    updates
}

fn make_delete_line(
    pt_object: &chaos_proto::chaos::PtObject,
    application_periods: &[DatePeriod],
    model: &BaseModel,
) -> Vec<Update> {
    let mut updates: Vec<Update> = vec![];
    if model.contains_line_id(pt_object.get_uri()) {
        updates = model
            .vehicle_journeys()
            .filter(|vj_idx| model.line_name(*vj_idx) == Some(pt_object.get_uri()))
            .map(|vj_idx| {
                make_delete_vehicle(
                    model.vehicle_journey_name(vj_idx),
                    application_periods,
                    model,
                )
            })
            .flatten()
            .collect();
    } else {
        error!(
            "line.id: {} does not exists in BaseModel",
            pt_object.get_uri()
        );
    }
    updates
}

fn make_delete_network(
    pt_object: &chaos_proto::chaos::PtObject,
    application_periods: &[DatePeriod],
    model: &BaseModel,
) -> Vec<Update> {
    let mut updates: Vec<Update> = vec![];
    if model.contains_network_id(pt_object.get_uri()) {
        updates = model
            .vehicle_journeys()
            .filter(|vj_idx| model.network_name(*vj_idx) == Some(pt_object.get_uri()))
            .map(|vj_idx| {
                make_delete_vehicle(
                    model.vehicle_journey_name(vj_idx),
                    application_periods,
                    model,
                )
            })
            .flatten()
            .collect();
    } else {
        error!(
            "network.id: {} does not exists in BaseModel",
            pt_object.get_uri()
        );
    }
    updates
}

#[derive(Debug, Clone)]
pub struct DatePeriod {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

fn clamp<T: PartialOrd<T>>(input: T, min_input: T, max_input: T) -> T
where
    T: std::cmp::Ord,
{
    min(max(input, min_input), max_input)
}

fn clamp_date(input: &DatePeriod, clamper: &DatePeriod) -> Option<DatePeriod> {
    let start = clamp(input.start, clamper.start, clamper.end);
    let end = clamp(input.end, clamper.start, clamper.end);
    if start <= end {
        Some(DatePeriod { start, end })
    } else {
        None
    }
}

pub struct DatePeriodIterator<'a> {
    period: &'a DatePeriod,
    current: NaiveDate,
}

impl<'a> Iterator for DatePeriodIterator<'a> {
    type Item = NaiveDate;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current <= self.period.end {
            let next = self.current + Duration::days(1);
            Some(mem::replace(&mut self.current, next))
        } else {
            None
        }
    }
}

impl<'a> IntoIterator for &'a DatePeriod {
    type Item = NaiveDate;
    type IntoIter = DatePeriodIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        DatePeriodIterator {
            period: self,
            current: self.start,
        }
    }
}
