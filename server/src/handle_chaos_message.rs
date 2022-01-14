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
use launch::loki::{
    chrono::{NaiveDate, NaiveTime},
    models::{
        base_model::BaseModel,
        real_time_disruption::{
            intersection, ts_to_dt, ApplicationPattern, Cause, ChannelType, DateTimePeriod,
            Disrupt, Disruption, Effect, Impact, Line, LineSection, Message, Network, PtObject,
            Route, Severity, StopArea, StopPoint, Tag, TimeSlot, Trip, Trip_, Update,
        },
    },
    tracing::error,
    NaiveDateTime,
};

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
    let start = model.validity_period().0.and_hms(0, 0, 0);
    let end = model.validity_period().1.and_hms(12, 59, 59);
    let validity_period = DateTimePeriod::new(start, end).unwrap();
    let application_period = compute_application_period(impact, &validity_period);
    let mut updates: Vec<Update> = vec![];

    for entity in impact.get_informed_entities() {
        match entity.get_pt_object_type() {
            PtObject_Type::network => {
                let update = make_delete_network(entity.get_uri(), &application_period, model);
                updates.extend(update);
            }
            PtObject_Type::line => {
                let update = make_delete_line(entity.get_uri(), &application_period, model);
                updates.extend(update);
            }
            PtObject_Type::route => {
                let update = make_delete_route(entity.get_uri(), &application_period, model);
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
    model_validity_period: &DateTimePeriod,
) -> Vec<DateTimePeriod> {
    impact
        .get_application_periods()
        .iter()
        .filter_map(|range| {
            let start = NaiveDateTime::from_timestamp(range.get_start() as i64, 0);
            let end = NaiveDateTime::from_timestamp(range.get_end() as i64, 0);
            let period = DateTimePeriod::new(start, end).ok()?;
            intersection(&period, model_validity_period)
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
    application_periods: &[DateTimePeriod],
    model: &BaseModel,
) -> Vec<Update> {
    let mut updates = vec![];
    if model.vehicle_journey_idx(vj_id).is_some() {
        updates = application_periods
            .iter()
            .flatten()
            .map(|day| Update::Delete(make_trip(vj_id.to_string(), day.date())))
            .collect();
    } else {
        error!("vehicule.uri {} does not exists in BaseModel", vj_id);
    }
    updates
}

fn make_delete_route(
    route_name: &str,
    application_periods: &[DateTimePeriod],
    model: &BaseModel,
) -> Vec<Update> {
    let mut updates: Vec<Update> = vec![];
    if model.contains_route_id(route_name) {
        updates = model
            .vehicle_journeys()
            .filter(|vj_idx| model.route_name(*vj_idx) == route_name)
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
        error!("route.id: {} does not exists in BaseModel", route_name);
    }
    updates
}

fn make_delete_line(
    line_name: &str,
    application_periods: &[DateTimePeriod],
    model: &BaseModel,
) -> Vec<Update> {
    let mut updates: Vec<Update> = vec![];
    if model.contains_line_id(line_name) {
        updates = model
            .vehicle_journeys()
            .filter(|vj_idx| model.line_name(*vj_idx) == Some(line_name))
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
        error!("line.id: {} does not exists in BaseModel", line_name);
    }
    updates
}

fn make_delete_network(
    network_name: &str,
    application_periods: &[DateTimePeriod],
    model: &BaseModel,
) -> Vec<Update> {
    let mut updates: Vec<Update> = vec![];
    if model.contains_network_id(network_name) {
        updates = model
            .vehicle_journeys()
            .filter(|vj_idx| model.network_name(*vj_idx) == Some(network_name))
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
        error!("network.id: {} does not exists in BaseModel", network_name);
    }
    updates
}

impl TryFrom<&chaos_proto::chaos::Disruption> for Disrupt {
    type Error = Error;
    fn try_from(proto: &chaos_proto::chaos::Disruption) -> Result<Disrupt, Error> {
        Ok(Disrupt {
            id: proto.get_id().to_string(),
            reference: None,
            contributor: proto.get_contributor().to_string(),
            publication_period: proto.get_publication_period().try_into()?,
            created_at: ts_to_dt(proto.get_created_at()),
            updated_at: ts_to_dt(proto.get_created_at()),
            cause: proto.get_cause().into(),
            tags: proto.get_tags().iter().map(|t| t.into()).collect(),
            impacts: proto
                .get_impacts()
                .iter()
                .map(|i| i.try_into())
                .collect::<Result<_, _>>()?,
        })
    }
}

impl TryFrom<&chaos_proto::chaos::Impact> for Impact {
    type Error = Error;
    fn try_from(proto: &chaos_proto::chaos::Impact) -> Result<Impact, Error> {
        Ok(Impact {
            id: proto.get_id().to_string(),
            company_id: None,
            physical_mode_id: None,
            headsign: None,
            created_at: ts_to_dt(proto.get_created_at()),
            updated_at: ts_to_dt(proto.get_created_at()),
            application_periods: proto
                .get_application_periods()
                .iter()
                .map(|ap| ap.try_into())
                .collect::<Result<_, _>>()?,
            application_patterns: proto
                .get_application_patterns()
                .iter()
                .map(|ap| ap.try_into())
                .collect::<Result<_, _>>()?,
            severity: proto.get_severity().into(),
            messages: proto.get_messages().iter().map(|m| m.into()).collect(),
            pt_objects: vec![],
            vehicle_info: None,
        })
    }
}

impl From<&chaos_proto::chaos::PtObject> for PtObject {
    fn from(proto: &chaos_proto::chaos::PtObject) -> PtObject {
        use chaos_proto::chaos::PtObject_Type;
        let id = proto.get_uri().to_string();
        let created_at = ts_to_dt(proto.get_created_at());
        let updated_at = ts_to_dt(proto.get_updated_at());
        match proto.get_pt_object_type() {
            PtObject_Type::network => PtObject::Network(Network {
                id,
                created_at,
                updated_at,
            }),
            PtObject_Type::line => PtObject::Line(Line {
                id,
                created_at,
                updated_at,
            }),
            PtObject_Type::route => PtObject::Route(Route {
                id,
                created_at,
                updated_at,
            }),
            PtObject_Type::trip => PtObject::Trip_(Trip_ {
                id,
                created_at,
                updated_at,
            }),
            PtObject_Type::line_section => {
                let ls = proto.get_pt_line_section();
                let line = ls.get_line();
                let start = ls.get_start_point();
                let end = ls.get_end_point();
                let routes = ls.get_routes();
                PtObject::LineSection(LineSection {
                    line: Line {
                        id: line.get_uri().to_string(),
                        created_at: ts_to_dt(line.get_created_at()),
                        updated_at: ts_to_dt(line.get_updated_at()),
                    },
                    start_sa: StopArea {
                        id: start.get_uri().to_string(),
                        created_at: ts_to_dt(start.get_created_at()),
                        updated_at: ts_to_dt(start.get_updated_at()),
                    },
                    stop_sa: StopArea {
                        id: end.get_uri().to_string(),
                        created_at: ts_to_dt(end.get_created_at()),
                        updated_at: ts_to_dt(end.get_updated_at()),
                    },
                    routes: routes
                        .iter()
                        .map(|r| Route {
                            id: r.get_uri().to_string(),
                            created_at: ts_to_dt(r.get_created_at()),
                            updated_at: ts_to_dt(r.get_updated_at()),
                        })
                        .collect(),
                })
            }
            PtObject_Type::rail_section => PtObject::Unknown,
            PtObject_Type::stop_point => PtObject::StopPoint(StopPoint {
                id,
                created_at,
                updated_at,
            }),
            PtObject_Type::stop_area => PtObject::StopArea(StopArea {
                id,
                created_at,
                updated_at,
            }),
            PtObject_Type::unkown_type => PtObject::Unknown,
        }
    }
}

impl From<&chaos_proto::chaos::Severity> for Severity {
    fn from(proto: &chaos_proto::chaos::Severity) -> Severity {
        Severity {
            id: proto.get_id().to_string(),
            wording: proto.get_wording().to_string(),
            color: proto.get_color().to_string(),
            priority: proto.get_priority() as u32,
            effect: proto.get_effect().into(),
            created_at: ts_to_dt(proto.get_created_at()),
            updated_at: ts_to_dt(proto.get_updated_at()),
        }
    }
}

impl From<chaos_proto::gtfs_realtime::Alert_Effect> for Effect {
    fn from(proto: chaos_proto::gtfs_realtime::Alert_Effect) -> Effect {
        use chaos_proto::gtfs_realtime::Alert_Effect;
        match proto {
            Alert_Effect::NO_SERVICE => Effect::NoService,
            Alert_Effect::REDUCED_SERVICE => Effect::ReducedService,
            Alert_Effect::SIGNIFICANT_DELAYS => Effect::SignificantDelays,
            Alert_Effect::DETOUR => Effect::Detour,
            Alert_Effect::ADDITIONAL_SERVICE => Effect::AdditionalService,
            Alert_Effect::MODIFIED_SERVICE => Effect::ModifiedService,
            Alert_Effect::OTHER_EFFECT => Effect::OtherEffect,
            Alert_Effect::UNKNOWN_EFFECT => Effect::UnknownEffect,
            Alert_Effect::STOP_MOVED => Effect::StopMoved,
        }
    }
}

impl From<&chaos_proto::chaos::Cause> for Cause {
    fn from(proto: &chaos_proto::chaos::Cause) -> Cause {
        Cause {
            id: proto.get_id().to_string(),
            wording: proto.get_wording().to_string(),
            category: proto.get_category().get_name().to_string(),
            created_at: ts_to_dt(proto.get_created_at()),
            updated_at: ts_to_dt(proto.get_created_at()),
        }
    }
}

impl TryFrom<&chaos_proto::gtfs_realtime::TimeRange> for DateTimePeriod {
    type Error = Error;
    fn try_from(proto: &chaos_proto::gtfs_realtime::TimeRange) -> Result<DateTimePeriod, Error> {
        let start = ts_to_dt(proto.get_start());
        let end = ts_to_dt(proto.get_end());
        match (start, end) {
            (Some(start), Some(end)) => {
                DateTimePeriod::new(start, end).map_err(|err| format_err!("Error : {:?}", err))
            }
            _ => Err(format_err!("Failed converting timestamp to datetime")),
        }
    }
}

impl From<&chaos_proto::chaos::Tag> for Tag {
    fn from(proto: &chaos_proto::chaos::Tag) -> Tag {
        Tag {
            id: proto.get_id().to_string(),
            name: proto.get_id().to_string(),
            created_at: ts_to_dt(proto.get_created_at()),
            updated_at: ts_to_dt(proto.get_created_at()),
        }
    }
}

impl From<&chaos_proto::chaos::Message> for Message {
    fn from(proto: &chaos_proto::chaos::Message) -> Message {
        let channel = proto.get_channel();
        Message {
            text: proto.get_text().to_string(),
            channel_id: channel.get_id().to_string(),
            channel_name: channel.get_name().to_string(),
            channel_content_type: channel.get_content_type().to_string(),
            channel_types: channel.get_types().iter().map(|t| t.into()).collect(),
            created_at: ts_to_dt(proto.get_created_at()),
            updated_at: ts_to_dt(proto.get_created_at()),
        }
    }
}

impl From<&chaos_proto::chaos::Channel_Type> for ChannelType {
    fn from(proto: &chaos_proto::chaos::Channel_Type) -> ChannelType {
        use chaos_proto::chaos::Channel_Type;
        match proto {
            Channel_Type::web => ChannelType::Web,
            Channel_Type::sms => ChannelType::Sms,
            Channel_Type::email => ChannelType::Email,
            Channel_Type::mobile => ChannelType::Mobile,
            Channel_Type::notification => ChannelType::Notification,
            Channel_Type::twitter => ChannelType::Twitter,
            Channel_Type::facebook => ChannelType::Facebook,
            Channel_Type::unkown_type => ChannelType::UnknownType,
            Channel_Type::title => ChannelType::Title,
            Channel_Type::beacon => ChannelType::Beacon,
        }
    }
}

impl TryFrom<&chaos_proto::chaos::Pattern> for ApplicationPattern {
    type Error = Error;
    fn try_from(proto: &chaos_proto::chaos::Pattern) -> Result<ApplicationPattern, Error> {
        let time_slots = proto.get_time_slots().iter().map(|ts| ts.into()).collect();
        let begin = ts_to_dt(u64::from(proto.get_start_date()));
        let end = ts_to_dt(u64::from(proto.get_end_date()));
        match (begin, end) {
            (Some(begin), Some(end)) => Ok(ApplicationPattern {
                begin_date: begin.date(),
                end_date: end.date(),
                time_slots,
            }),
            _ => Err(format_err!("Failed converting timestamp to datetime")),
        }
    }
}

impl From<&chaos_proto::chaos::TimeSlot> for TimeSlot {
    fn from(proto: &chaos_proto::chaos::TimeSlot) -> TimeSlot {
        TimeSlot {
            begin: NaiveTime::from_num_seconds_from_midnight(proto.get_begin(), 0),
            end: NaiveTime::from_num_seconds_from_midnight(proto.get_end(), 0),
        }
    }
}
