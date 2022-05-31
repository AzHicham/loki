// Copyright  (C) 2020, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Hove (www.kisio.com).
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
use anyhow::{bail, format_err, Context, Error};
use launch::loki::{
    chrono::NaiveTime,
    models::{
        base_model::{
            strip_id_prefix, PREFIX_ID_LINE, PREFIX_ID_NETWORK, PREFIX_ID_ROUTE,
            PREFIX_ID_STOP_AREA, PREFIX_ID_STOP_POINT, PREFIX_ID_VEHICLE_JOURNEY,
        },
        real_time_disruption::{
            chaos_disruption::{
                ApplicationPattern, BlockedStopArea, Cause, ChannelType, ChaosDisruption,
                ChaosImpact, DisruptionProperty, Impacted, Informed, LineId, LineSection, Message,
                NetworkId, RailSection, RouteId, Severity, StopAreaId, StopPointId, Tag, TimeSlot,
            },
            time_periods::TimePeriod,
            Effect, VehicleJourneyId,
        },
    },
    NaiveDateTime,
};

pub fn handle_chaos_protobuf(
    proto: &chaos_proto::chaos::Disruption,
) -> Result<ChaosDisruption, Error> {
    let id = proto
        .id
        .as_ref()
        .ok_or_else(|| format_err!("Disruption has no id."))?
        .to_string();

    let reference = proto.reference.as_ref().map(ToString::to_string);

    let publication_period = proto
        .publication_period
        .as_ref()
        .ok_or_else(|| format_err!("Disruption has no publication period"))?;
    let publication_period = make_datetime_period(publication_period)
        .context("Could not parse disruption.publication_period")?;

    let cause = proto
        .cause
        .as_ref()
        .ok_or_else(|| format_err!("Disruption has no cause"))?;
    let cause = make_cause(cause)?;

    let tags: Vec<_> = proto.tags.iter().map(make_tag).collect::<Result<_, _>>()?;

    let impacts = {
        let mut impacts = Vec::with_capacity(proto.impacts.len());
        for (idx, proto_impact) in proto.impacts.iter().enumerate() {
            let impact = make_impact(proto_impact)
                .with_context(|| format!("Could not parse {}-th impact of disruption", idx))?;
            impacts.push(impact);
        }
        impacts
    };

    let contributor = proto.contributor.as_ref().map(ToString::to_string);

    let properties = make_properties(&proto.properties)?;

    Ok(ChaosDisruption {
        id,
        reference,
        contributor,
        publication_period,
        cause,
        tags,
        properties,
        impacts,
    })
}

fn make_impact(proto: &chaos_proto::chaos::Impact) -> Result<ChaosImpact, Error> {
    let id = proto
        .id
        .as_ref()
        .ok_or_else(|| format_err!("Impact has no id"))?
        .to_string();

    let created_at = proto
        .created_at
        .ok_or_else(|| format_err!("Impact has no created_at datetime"))?;
    let created_at = make_datetime(created_at).context("Could not parse impact.created_at")?;

    let updated_at = proto
        .updated_at
        .map(|updated_at| make_datetime(updated_at).context("Could not parse impact.updated_at"))
        .transpose()?
        .unwrap_or(created_at);

    let severity = proto
        .severity
        .as_ref()
        .ok_or_else(|| format_err!("Impact has no severity"))?;
    let severity = make_severity(severity).context("Could not parse impact.severity")?;

    let application_periods = make_periods(&proto.application_periods)
        .context("Could not parse impact.application_periods")?;

    let application_patterns = make_application_patterns(&proto.application_patterns)
        .context("Could not parse impact.application_patterns")?;

    let messages = make_messages(&proto.messages).context("Could not parse impact.messages")?;

    let effect = severity.effect;
    let mut impacted_pt_objects = vec![];
    let mut informed_pt_objects = vec![];

    for entity in &proto.informed_entities {
        dispatch_pt_object(
            entity,
            effect,
            &mut impacted_pt_objects,
            &mut informed_pt_objects,
        )
        .context("Failed to handle an informed entity")?;
    }

    Ok(ChaosImpact {
        id,
        updated_at,
        application_periods,
        application_patterns,
        severity,
        messages,
        impacted_pt_objects,
        informed_pt_objects,
    })
}

fn dispatch_pt_object(
    proto: &chaos_proto::chaos::PtObject,
    effect: Effect,
    impacted: &mut Vec<Impacted>,
    informed: &mut Vec<Informed>,
) -> Result<(), Error> {
    use chaos_proto::chaos::pt_object::Type;

    let id = proto
        .uri
        .as_ref()
        .ok_or_else(|| format_err!("PtObject has no uri"))?
        .to_string();
    let pt_object_type = proto
        .pt_object_type
        .as_ref()
        .ok_or_else(|| format_err!("PtObject has no pt_object_type"))?
        .enum_value()
        .map_err(|value| format_err!("'{}' is not a valid 'PtObjectType'", value))?;

    match (pt_object_type, effect) {
        (Type::network, Effect::NoService) => {
            impacted.push(Impacted::NetworkDeleted(NetworkId {
                id: strip_id_prefix(&id, PREFIX_ID_NETWORK).to_string(),
            }));
        }
        (Type::network, _) => {
            informed.push(Informed::Network(NetworkId {
                id: strip_id_prefix(&id, PREFIX_ID_NETWORK).to_string(),
            }));
        }

        (Type::route, Effect::NoService) => {
            impacted.push(Impacted::RouteDeleted(RouteId {
                id: strip_id_prefix(&id, PREFIX_ID_ROUTE).to_string(),
            }));
        }
        (Type::route, _) => {
            informed.push(Informed::Route(RouteId {
                id: strip_id_prefix(&id, PREFIX_ID_ROUTE).to_string(),
            }));
        }

        (Type::line, Effect::NoService) => {
            impacted.push(Impacted::LineDeleted(LineId {
                id: strip_id_prefix(&id, PREFIX_ID_LINE).to_string(),
            }));
        }
        (Type::line, _) => {
            informed.push(Informed::Line(LineId {
                id: strip_id_prefix(&id, PREFIX_ID_LINE).to_string(),
            }));
        }

        (Type::stop_point, Effect::NoService | Effect::Detour) => {
            impacted.push(Impacted::StopPointDeleted(StopPointId {
                id: strip_id_prefix(&id, PREFIX_ID_STOP_POINT).to_string(),
            }));
        }
        (Type::stop_point, _) => {
            informed.push(Informed::StopPoint(StopPointId {
                id: strip_id_prefix(&id, PREFIX_ID_STOP_POINT).to_string(),
            }));
        }

        (Type::stop_area, Effect::NoService | Effect::Detour) => {
            impacted.push(Impacted::StopAreaDeleted(StopAreaId {
                id: strip_id_prefix(&id, PREFIX_ID_STOP_AREA).to_string(),
            }));
        }
        (Type::stop_area, _) => {
            informed.push(Informed::StopArea(StopAreaId {
                id: strip_id_prefix(&id, PREFIX_ID_STOP_AREA).to_string(),
            }));
        }

        (Type::trip, Effect::NoService) => {
            impacted.push(Impacted::BaseTripDeleted(VehicleJourneyId {
                id: strip_id_prefix(&id, PREFIX_ID_VEHICLE_JOURNEY).to_string(),
            }));
        }
        (Type::trip, _) => informed.push(Informed::Trip(VehicleJourneyId {
            id: strip_id_prefix(&id, PREFIX_ID_VEHICLE_JOURNEY).to_string(),
        })),

        (Type::line_section, _) => {
            let proto_line_section = proto.pt_line_section.as_ref().ok_or_else(|| {
                format_err!("PtObject has type line_section but the field pt_line_section is empty")
            })?;

            let line_uri = proto_line_section
                .line
                .as_ref()
                .ok_or_else(|| format_err!("'LineSection' has no 'line'"))?
                .uri
                .as_ref()
                .ok_or_else(|| format_err!("'PtObject' (line) has no 'uri'"))?;
            let start_stop_area_uri = proto_line_section
                .start_point
                .as_ref()
                .ok_or_else(|| format_err!("'LineSection' has no 'start_point'"))?
                .uri
                .as_ref()
                .ok_or_else(|| format_err!("'PtObject' (stop_point) has no 'uri'"))?;
            let end_stop_area_uri = proto_line_section
                .end_point
                .as_ref()
                .ok_or_else(|| format_err!("'LineSection' has no 'end_point'"))?
                .uri
                .as_ref()
                .ok_or_else(|| format_err!("'PtObject' (stop_point) has no 'uri'"))?;
            let line_section = LineSection {
                line: LineId {
                    id: strip_id_prefix(line_uri, PREFIX_ID_LINE).to_string(),
                },
                start: StopAreaId {
                    id: strip_id_prefix(start_stop_area_uri, PREFIX_ID_STOP_AREA).to_string(),
                },
                end: StopAreaId {
                    id: strip_id_prefix(end_stop_area_uri, PREFIX_ID_STOP_AREA).to_string(),
                },
                routes: proto_line_section
                    .routes
                    .iter()
                    .map(|route| {
                        let route_uri = route
                            .uri
                            .as_ref()
                            .ok_or_else(|| format_err!("'PtObject' (route) has no 'uri'"))?;
                        let route_id = RouteId {
                            id: strip_id_prefix(route_uri, PREFIX_ID_ROUTE).to_string(),
                        };
                        Ok::<_, Error>(route_id)
                    })
                    .collect::<Result<_, _>>()?,
            };
            impacted.push(Impacted::LineSection(line_section));
        }
        (Type::rail_section, _) => {
            let proto_rail_section = proto.pt_rail_section.as_ref().ok_or_else(|| {
                format_err!("PtObject has type line_section but the field pt_line_section is empty")
            })?;

            let line_uri = proto_rail_section
                .line
                .as_ref()
                .ok_or_else(|| format_err!("'LineSection' has no 'line'"))?
                .uri
                .as_ref()
                .ok_or_else(|| format_err!("'PtObject' (line) has no 'uri'"))?;
            let start_stop_area_uri = proto_rail_section
                .start_point
                .as_ref()
                .ok_or_else(|| format_err!("'LineSection' has no 'start_point'"))?
                .uri
                .as_ref()
                .ok_or_else(|| format_err!("'PtObject' (stop_point) has no 'uri'"))?;
            let end_stop_area_uri = proto_rail_section
                .end_point
                .as_ref()
                .ok_or_else(|| format_err!("'LineSection' has no 'end_point'"))?
                .uri
                .as_ref()
                .ok_or_else(|| format_err!("'PtObject' (stop_point) has no 'uri'"))?;
            let rail_section = RailSection {
                line: LineId {
                    id: strip_id_prefix(line_uri, PREFIX_ID_LINE).to_string(),
                },
                start: StopAreaId {
                    id: strip_id_prefix(start_stop_area_uri, PREFIX_ID_STOP_AREA).to_string(),
                },
                end: StopAreaId {
                    id: strip_id_prefix(end_stop_area_uri, PREFIX_ID_STOP_AREA).to_string(),
                },
                routes: proto_rail_section
                    .routes
                    .iter()
                    .map(|route| {
                        let route_uri = route
                            .uri
                            .as_ref()
                            .ok_or_else(|| format_err!("'PtObject' (route) has no 'uri'"))?;
                        let route_id = RouteId {
                            id: strip_id_prefix(route_uri, PREFIX_ID_ROUTE).to_string(),
                        };
                        Ok::<_, Error>(route_id)
                    })
                    .collect::<Result<_, _>>()?,
                blocked_stop_area: proto_rail_section
                    .blocked_stop_areas
                    .iter()
                    .map(|stop_area| {
                        let stop_area_uri = stop_area
                            .uri
                            .as_ref()
                            .ok_or_else(|| format_err!("'PtObject' (stop_area) has no 'uri'"))?;
                        let stop_area_order = stop_area
                            .order
                            .ok_or_else(|| format_err!("'PtObject' (stop_area) has no 'order'"))?;
                        let blocked_stop_area = BlockedStopArea {
                            id: strip_id_prefix(stop_area_uri, PREFIX_ID_STOP_AREA).to_string(),
                            order: stop_area_order,
                        };
                        Ok::<_, Error>(blocked_stop_area)
                    })
                    .collect::<Result<_, _>>()?,
            };
            impacted.push(Impacted::RailSection(rail_section));
        }
        (Type::unkown_type, _) => {
            bail!("PtObject with type unknown_type");
        }
    };

    Ok(())
}

fn make_severity(proto: &chaos_proto::chaos::Severity) -> Result<Severity, Error> {
    let effect = proto
        .effect
        .as_ref()
        .ok_or_else(|| format_err!("'Severity' has no 'effect'"))?
        .enum_value()
        .map_err(|value| format_err!("'{}' is not a valid 'Effect'", value))?;
    let effect = make_effect(effect);

    let result = Severity {
        wording: proto.wording.clone(),
        color: proto.color.clone(),
        priority: proto.priority,
        effect,
    };
    Ok(result)
}

pub fn make_effect(proto: chaos_proto::gtfs_realtime::alert::Effect) -> Effect {
    use chaos_proto::gtfs_realtime::alert;
    match proto {
        alert::Effect::NO_SERVICE => Effect::NoService,
        alert::Effect::UNKNOWN_EFFECT => Effect::UnknownEffect,
        alert::Effect::SIGNIFICANT_DELAYS => Effect::SignificantDelays,
        alert::Effect::MODIFIED_SERVICE => Effect::ModifiedService,
        alert::Effect::DETOUR => Effect::Detour,
        alert::Effect::REDUCED_SERVICE => Effect::ReducedService,
        alert::Effect::ADDITIONAL_SERVICE => Effect::AdditionalService,
        alert::Effect::OTHER_EFFECT => Effect::OtherEffect,
        alert::Effect::STOP_MOVED => Effect::StopMoved,
    }
}

fn make_cause(proto: &chaos_proto::chaos::Cause) -> Result<Cause, Error> {
    let wording = proto
        .wording
        .as_ref()
        .ok_or_else(|| format_err!("'Cause' has no 'wording'"))?
        .to_string();
    let category = proto
        .category
        .as_ref()
        .ok_or_else(|| format_err!("'Cause' has no 'category'"))?
        .name
        .as_ref()
        .ok_or_else(|| format_err!("'Category' has no 'name'"))?
        .to_string();
    let cause = Cause { wording, category };
    Ok(cause)
}

fn make_datetime_period(
    proto: &chaos_proto::gtfs_realtime::TimeRange,
) -> Result<TimePeriod, Error> {
    let start_timestamp = proto
        .start
        .ok_or_else(|| format_err!("'TimeRange' has no 'start'"))?;
    let end_timestamp = proto
        .end
        .ok_or_else(|| format_err!("'TimeRange' has no 'end'"))?;

    let start = make_datetime(start_timestamp).with_context(|| {
        format!(
            "Could not convert start timestamp {} to datetime",
            start_timestamp
        )
    })?;
    let end = make_datetime(end_timestamp).with_context(|| {
        format!(
            "Could not convert end timestamp {} to datetime",
            end_timestamp
        )
    })?;
    TimePeriod::new(start, end).map_err(Error::from)
}

fn make_periods(
    time_ranges: &[chaos_proto::gtfs_realtime::TimeRange],
) -> Result<Vec<TimePeriod>, Error> {
    let mut result = Vec::with_capacity(time_ranges.len());
    for (idx, time_range) in time_ranges.iter().enumerate() {
        let period = make_datetime_period(time_range)
            .with_context(|| format!("Could not convert {}-th TimeRange", idx))?;
        result.push(period);
    }
    Ok(result)
}

fn make_tag(proto: &chaos_proto::chaos::Tag) -> Result<Tag, Error> {
    proto
        .name
        .as_ref()
        .ok_or_else(|| format_err!("'Tag' has no 'name'"))
        .map(|name| Tag {
            name: name.to_string(),
        })
}

fn make_messages(proto_messages: &[chaos_proto::chaos::Message]) -> Result<Vec<Message>, Error> {
    let mut result = Vec::with_capacity(proto_messages.len());
    for (idx, proto_message) in proto_messages.iter().enumerate() {
        let message = make_message(proto_message)
            .with_context(|| format!("Could not convert {}-th Message", idx))?;
        result.push(message);
    }
    Ok(result)
}

fn make_message(proto: &chaos_proto::chaos::Message) -> Result<Message, Error> {
    let text = proto
        .text
        .as_ref()
        .ok_or_else(|| format_err!("'Message' has no 'text'"))?
        .to_string();
    let channel = proto
        .channel
        .as_ref()
        .ok_or_else(|| format_err!("'Message' has no 'channel'"))?;
    let channel_id = channel.id.clone();
    let channel_name = channel
        .name
        .as_ref()
        .ok_or_else(|| format_err!("'Channel' has no 'name'"))?
        .to_string();
    let channel_content_type = channel.content_type.clone();

    let result = Message {
        text,
        channel_id,
        channel_name,
        channel_content_type,
        channel_types: channel
            .types
            .iter()
            .map(|channel_type| {
                let channel_type = channel_type
                    .enum_value()
                    .map_err(|value| format_err!("'{}' is not a valid 'channel::Type'", value))?;
                let channel_type = make_channel_type(channel_type);
                Ok::<_, Error>(channel_type)
            })
            .collect::<Result<_, _>>()?,
    };
    Ok(result)
}

fn make_channel_type(proto: chaos_proto::chaos::channel::Type) -> ChannelType {
    use chaos_proto::chaos::channel;
    match proto {
        channel::Type::web => ChannelType::Web,
        channel::Type::sms => ChannelType::Sms,
        channel::Type::email => ChannelType::Email,
        channel::Type::mobile => ChannelType::Mobile,
        channel::Type::notification => ChannelType::Notification,
        channel::Type::twitter => ChannelType::Twitter,
        channel::Type::facebook => ChannelType::Facebook,
        channel::Type::unkown_type => ChannelType::UnknownType,
        channel::Type::title => ChannelType::Title,
        channel::Type::beacon => ChannelType::Beacon,
    }
}

fn make_application_pattern(
    proto: &chaos_proto::chaos::Pattern,
) -> Result<ApplicationPattern, Error> {
    let begin_date = proto
        .start_date
        .ok_or_else(|| format_err!("'Pattern' has no 'start_date'"))?;
    let begin_date = make_datetime(u64::from(begin_date))?.date(); // u32 can always be coerced to u64
    let end_date = proto
        .end_date
        .ok_or_else(|| format_err!("'Pattern' has no 'end_date'"))?;
    let end_date = make_datetime(u64::from(end_date))?.date(); // u32 can always be coerced to u64
    let time_slots = proto
        .time_slots
        .iter()
        .map(make_timeslot)
        .collect::<Result<_, _>>()?;
    let mut week_pattern = [false; 7];
    let proto_week_pattern = proto
        .week_pattern
        .as_ref()
        .ok_or_else(|| format_err!("'Pattern' has no 'week_pattern'"))?;
    week_pattern[0] = matches!(proto_week_pattern.monday, Some(true));
    week_pattern[1] = matches!(proto_week_pattern.tuesday, Some(true));
    week_pattern[2] = matches!(proto_week_pattern.wednesday, Some(true));
    week_pattern[3] = matches!(proto_week_pattern.thursday, Some(true));
    week_pattern[4] = matches!(proto_week_pattern.friday, Some(true));
    week_pattern[5] = matches!(proto_week_pattern.saturday, Some(true));
    week_pattern[6] = matches!(proto_week_pattern.sunday, Some(true));

    Ok(ApplicationPattern {
        begin_date,
        end_date,
        time_slots,
        week_pattern,
    })
}

fn make_application_patterns(
    proto_patterns: &[chaos_proto::chaos::Pattern],
) -> Result<Vec<ApplicationPattern>, Error> {
    let mut result = Vec::with_capacity(proto_patterns.len());
    for (idx, proto_pattern) in proto_patterns.iter().enumerate() {
        let pattern = make_application_pattern(proto_pattern)
            .with_context(|| format!("Could not convert {}-th Pattern", idx))?;
        result.push(pattern);
    }
    Ok(result)
}

fn make_timeslot(proto: &chaos_proto::chaos::TimeSlot) -> Result<TimeSlot, Error> {
    let begin = proto
        .begin
        .ok_or_else(|| format_err!("'TimeSlot' has no 'begin'"))?;
    let end = proto
        .end
        .ok_or_else(|| format_err!("'TimeSlot' has no 'end'"))?;
    let time_slot = TimeSlot {
        begin: NaiveTime::from_num_seconds_from_midnight(begin, 0),
        end: NaiveTime::from_num_seconds_from_midnight(end, 0),
    };
    Ok(time_slot)
}

fn make_properties(
    proto_properties: &[chaos_proto::chaos::DisruptionProperty],
) -> Result<Vec<DisruptionProperty>, Error> {
    let mut result = Vec::with_capacity(proto_properties.len());
    for (idx, proto_property) in proto_properties.iter().enumerate() {
        let property = make_property(proto_property)
            .with_context(|| format!("Could not convert {}-th DisruptionProperty", idx))?;
        result.push(property);
    }
    Ok(result)
}

fn make_property(
    proto: &chaos_proto::chaos::DisruptionProperty,
) -> Result<DisruptionProperty, Error> {
    let key = proto
        .key
        .as_ref()
        .ok_or_else(|| format_err!("'DisruptionProperty' has no 'key'"))?
        .to_string();
    let value = proto
        .value
        .as_ref()
        .ok_or_else(|| format_err!("'DisruptionProperty' has no 'value'"))?
        .to_string();
    let type_ = proto
        .type_
        .as_ref()
        .ok_or_else(|| format_err!("'DisruptionProperty' has no 'type'"))?
        .to_string();
    Ok(DisruptionProperty { key, type_, value })
}

pub fn make_datetime(timestamp: u64) -> Result<NaiveDateTime, Error> {
    let timestamp = i64::try_from(timestamp)?;
    Ok(NaiveDateTime::from_timestamp(timestamp, 0))
}
