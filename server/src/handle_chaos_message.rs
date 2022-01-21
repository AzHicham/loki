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

use crate::chaos_proto::{self};
use anyhow::{bail, Context, Error};
use launch::loki::{
    chrono::NaiveTime,
    models::real_time_disruption::{
        ApplicationPattern, Cause, ChannelType, DateTimePeriod, Disruption, Effect, Impact,
        Impacted, Informed, LineId, LineSectionDisruption, Message, NetworkId, RouteId, Severity,
        StopAreaId, StopPointId, Tag, TimeSlot, VehicleJourneyId,
    },
    NaiveDateTime,
};

pub fn handle_chaos_protobuf(proto: &chaos_proto::chaos::Disruption) -> Result<Disruption, Error> {
    let id = if proto.has_id() {
        proto.get_id().to_string()
    } else {
        bail!("Disruption has no id.");
    };

    let reference = if proto.has_reference() {
        Some(proto.get_reference().to_string())
    } else {
        None
    };

    let publication_period = if proto.has_publication_period() {
        make_datetime_period(proto.get_publication_period())
            .context("Could not parse disruption.publication_period")?
    } else {
        bail!("Disruption has no publication period");
    };

    let created_at = if proto.has_created_at() {
        let datetime = make_datetime(proto.get_created_at())
            .context(format!("Could not parse disruption.created_at"))?;
        Some(datetime)
    } else {
        None
    };

    let updated_at = if proto.has_updated_at() {
        let datetime = make_datetime(proto.get_updated_at())
            .context(format!("Could not parse disruption.updated_at"))?;
        Some(datetime)
    } else {
        None
    };

    let cause = if proto.has_cause() {
        make_cause(proto.get_cause())
    } else {
        bail!("Disruption has no cause");
    };

    let tags: Vec<_> = proto.get_tags().iter().map(make_tag).collect();

    let impacts = {
        let proto_impacts = proto.get_impacts();
        let mut impacts = Vec::with_capacity(proto_impacts.len());
        for (idx, proto_impact) in proto.get_impacts().iter().enumerate() {
            let impact = make_impact(proto_impact)
                .with_context(|| format!("Could not parse {}-th impact of disruption", idx))?;
            impacts.push(impact);
        }
        impacts
    };

    let contributor = if proto.has_contributor() {
        Some(proto.get_contributor().to_string())
    } else {
        None
    };

    let properties = make_properties(proto.get_properties())?;

    Ok(Disruption {
        id,
        reference,
        publication_period,
        created_at,
        updated_at,
        cause,
        tags,
        properties,
        impacts,
        contributor,
    })
}

fn make_impact(proto: &chaos_proto::chaos::Impact) -> Result<Impact, Error> {
    let id = if proto.has_id() {
        proto.get_id().to_string()
    } else {
        bail!("Impact has no id");
    };

    let created_at = if proto.has_created_at() {
        let datetime = make_datetime(proto.get_created_at())
            .context(format!("Could not parse impact.created_at"))?;
        Some(datetime)
    } else {
        None
    };

    let updated_at = if proto.has_updated_at() {
        let datetime = make_datetime(proto.get_updated_at())
            .context(format!("Could not parse impact.updated_at"))?;
        Some(datetime)
    } else {
        None
    };

    let severity = if proto.has_severity() {
        make_severity(proto.get_severity()).context(format!("Could not parse impact.severity"))?
    } else {
        bail!("Impact has no severity");
    };

    let application_periods = make_periods(proto.get_application_periods())
        .context(format!("Could not parse impact.application_periods"))?;

    let application_patterns = make_application_patterns(proto.get_application_patterns())
        .context(format!("Could not parse impact.application_patterns"))?;

    let messages =
        make_messages(proto.get_messages()).context(format!("Could not parse impact.messages"))?;

    let effect = severity.effect;
    let mut impacted_pt_objects = vec![];
    let mut informed_pt_objects = vec![];

    for entity in proto.get_informed_entities() {
        dispatch_pt_object(
            entity,
            &effect,
            &mut impacted_pt_objects,
            &mut informed_pt_objects,
        )
        .context("Failed to handle an informed entity")?;
    }

    Ok(Impact {
        id,
        created_at,
        updated_at,
        severity,
        application_periods,
        application_patterns,
        messages,
        impacted_pt_objects,
        informed_pt_objects,
    })
}

fn dispatch_pt_object(
    proto: &chaos_proto::chaos::PtObject,
    effect: &Effect,
    impacted: &mut Vec<Impacted>,
    informed: &mut Vec<Informed>,
) -> Result<(), Error> {
    use chaos_proto::chaos::PtObject_Type as Type;

    let id = if proto.has_uri() {
        proto.get_uri().to_string()
    } else {
        bail!("PtObject has no uri");
    };
    let pt_object_type = if proto.has_pt_object_type() {
        proto.get_pt_object_type()
    } else {
        bail!("PtObject has no pt_object_type");
    };

    match (pt_object_type, effect) {
        (Type::network, Effect::NoService) => {
            impacted.push(Impacted::NetworkDeleted(NetworkId { id }));
        }
        (Type::network, _) => {
            informed.push(Informed::Network(NetworkId { id }));
        }

        (Type::route, Effect::NoService) => {
            impacted.push(Impacted::RouteDeleted(RouteId { id }));
        }
        (Type::route, _) => {
            informed.push(Informed::Route(RouteId { id }));
        }

        (Type::line, Effect::NoService) => {
            impacted.push(Impacted::LineDeleted(LineId { id }));
        }
        (Type::line, _) => {
            informed.push(Informed::Line(LineId { id }));
        }

        (Type::stop_point, Effect::NoService) => {
            impacted.push(Impacted::StopPointDeleted(StopPointId { id }));
        }
        (Type::stop_point, _) => {
            informed.push(Informed::StopPoint(StopPointId { id }));
        }

        (Type::stop_area, Effect::NoService) => {
            impacted.push(Impacted::StopAreaDeleted(StopAreaId { id }));
        }
        (Type::stop_area, _) => {
            informed.push(Informed::StopArea(StopAreaId { id }));
        }

        (Type::trip, Effect::NoService) => {
            impacted.push(Impacted::TripDeleted(VehicleJourneyId { id }));
        }
        (Type::trip, _) => informed.push(Informed::Trip(VehicleJourneyId { id })),

        (Type::line_section, _) => {
            if !proto.has_pt_line_section() {
                bail!("PtObject has type line_section but the field pt_line_section is empty");
            }
            let proto_line_section = proto.get_pt_line_section();

            let line = proto_line_section.get_line();
            let start_stop_area = proto_line_section.get_start_point();
            let end_stop_area = proto_line_section.get_end_point();
            let routes = proto_line_section.get_routes();
            let line_section = LineSectionDisruption {
                line: LineId {
                    id: line.get_uri().to_string(),
                },
                start: StopAreaId {
                    id: start_stop_area.get_uri().to_string(),
                },
                end: StopAreaId {
                    id: end_stop_area.get_uri().to_string(),
                },
                routes: routes
                    .iter()
                    .map(|r| RouteId {
                        id: r.get_uri().to_string(),
                    })
                    .collect(),
            };
            impacted.push(Impacted::LineSection(line_section));
        }
        (Type::rail_section, _) => {
            bail!("RailSection is not handled");
        }
        (Type::unkown_type, _) => {
            bail!("PtObject with type unknown_type");
        }
    };

    Ok(())
}

fn make_severity(proto: &chaos_proto::chaos::Severity) -> Result<Severity, Error> {
    let id = if proto.has_id() {
        proto.get_id().to_string()
    } else {
        bail!("Severity has no id");
    };

    let created_at = if proto.has_created_at() {
        let datetime = make_datetime(proto.get_created_at())
            .context(format!("Could not parse severity.created_at"))?;
        Some(datetime)
    } else {
        None
    };

    let updated_at = if proto.has_updated_at() {
        let datetime = make_datetime(proto.get_updated_at())
            .context(format!("Could not parse severity.updated_at"))?;
        Some(datetime)
    } else {
        None
    };

    let effect = if proto.has_effect() {
        make_effect(proto.get_effect())
    } else {
        bail!("Severity has no effect");
    };

    let priority = if proto.has_priority() {
        Some(proto.get_priority())
    } else {
        None
    };

    let color = if proto.has_color() {
        Some(proto.get_color().to_string())
    } else {
        None
    };

    let wording = if proto.has_wording() {
        Some(proto.get_wording().to_string())
    } else {
        None
    };

    let result = Severity {
        id,
        wording,
        color,
        priority,
        effect,
        created_at,
        updated_at,
    };
    Ok(result)
}

pub fn make_effect(proto: chaos_proto::gtfs_realtime::Alert_Effect) -> Effect {
    use chaos_proto::gtfs_realtime::Alert_Effect;
    match proto {
        Alert_Effect::NO_SERVICE => Effect::NoService,
        Alert_Effect::UNKNOWN_EFFECT => Effect::UnknownEffect,
        Alert_Effect::SIGNIFICANT_DELAYS => Effect::SignificantDelays,
        Alert_Effect::MODIFIED_SERVICE => Effect::ModifiedService,
        Alert_Effect::DETOUR => Effect::Detour,
        Alert_Effect::REDUCED_SERVICE => Effect::ReducedService,
        Alert_Effect::ADDITIONAL_SERVICE => Effect::AdditionalService,
        Alert_Effect::OTHER_EFFECT => Effect::OtherEffect,
        Alert_Effect::STOP_MOVED => Effect::StopMoved,
    }
}

fn make_cause(proto: &chaos_proto::chaos::Cause) -> Cause {
    Cause {
        id: proto.get_id().to_string(),
        wording: proto.get_wording().to_string(),
        category: proto.get_category().get_name().to_string(),
    }
}

fn make_datetime_period(
    proto: &chaos_proto::gtfs_realtime::TimeRange,
) -> Result<DateTimePeriod, Error> {
    let start_timestamp = if proto.has_start() {
        proto.get_start()
    } else {
        bail!("No start timestamp");
    };

    let end_timestamp = if proto.has_end() {
        proto.get_end()
    } else {
        bail!("No end timestamp");
    };

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
    DateTimePeriod::new(start, end).map_err(|err| Error::from(err))
}

fn make_periods(
    time_ranges: &[chaos_proto::gtfs_realtime::TimeRange],
) -> Result<Vec<DateTimePeriod>, Error> {
    let mut result = Vec::with_capacity(time_ranges.len());
    for (idx, time_range) in time_ranges.iter().enumerate() {
        let period = make_datetime_period(time_range)
            .with_context(|| format!("Could not convert {}-th TimeRange", idx))?;
        result.push(period);
    }
    Ok(result)
}

fn make_tag(proto: &chaos_proto::chaos::Tag) -> Tag {
    Tag {
        id: proto.get_id().to_string(),
        name: proto.get_id().to_string(),
    }
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
    let text = if proto.has_text() {
        proto.get_text().to_string()
    } else {
        bail!("Message has no text");
    };
    let channel = if proto.has_channel() {
        proto.get_channel()
    } else {
        bail!("Message has no channel");
    };

    let result = Message {
        text,
        channel_id: channel.get_id().to_string(),
        channel_name: channel.get_name().to_string(),
        channel_content_type: channel.get_content_type().to_string(),
        channel_types: channel.get_types().iter().map(make_channel_type).collect(),
    };
    Ok(result)
}

fn make_channel_type(proto: &chaos_proto::chaos::Channel_Type) -> ChannelType {
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

fn make_application_pattern(
    proto: &chaos_proto::chaos::Pattern,
) -> Result<ApplicationPattern, Error> {
    let begin_date = if proto.has_start_date() {
        let timestamp = proto.get_start_date();
        let datetime = NaiveDateTime::from_timestamp(i64::from(timestamp), 0);
        datetime.date()
    } else {
        bail!("Pattern has no start_date");
    };
    let end_date = if proto.has_end_date() {
        let timestamp = proto.get_end_date();
        let datetime = NaiveDateTime::from_timestamp(i64::from(timestamp), 0);
        datetime.date()
    } else {
        bail!("Pattern has no end_date");
    };
    let time_slots = proto.get_time_slots().iter().map(make_timeslot).collect();
    Ok(ApplicationPattern {
        begin_date,
        end_date,
        time_slots,
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

fn make_timeslot(proto: &chaos_proto::chaos::TimeSlot) -> TimeSlot {
    TimeSlot {
        begin: NaiveTime::from_num_seconds_from_midnight(proto.get_begin(), 0),
        end: NaiveTime::from_num_seconds_from_midnight(proto.get_end(), 0),
    }
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
    let key = if proto.has_key() {
        proto.get_key()
    } else {
        bail!("DisruptionProperty has no key");
    };
    let value = if proto.has_value() {
        proto.get_value()
    } else {
        bail!("DisruptionProperty has no value");
    };
    let type_ = if proto.has_field_type() {
        proto.get_field_type()
    } else {
        bail!("DisruptionProperty has no type_");
    };
    Ok(DisruptionProperty {
        key: key.to_string(),
        type_: type_.to_string(),
        value: value.to_string(),
    })
}

pub fn make_datetime(timestamp: u64) -> Result<NaiveDateTime, Error> {
    let timestamp = i64::try_from(timestamp)?;
    Ok(NaiveDateTime::from_timestamp(timestamp, 0))
}

pub enum PtObject {
    Impacted(Impacted),
    Informed(Informed),
}
