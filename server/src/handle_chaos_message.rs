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
    chrono::NaiveTime,
    models::real_time_disruption::{
        timestamp_to_datetime, ApplicationPattern, Cause, ChannelType, DateTimePeriod, Disruption,
        Effect, Impact, Impacted, Informed, LineDisruption, LineSectionDisruption, Message,
        NetworkDisruption, PtObjectType, RouteDisruption, Severity, StopAreaDisruption,
        StopPointDisruption, Tag, TimeSlot, TripDisruption,
    },
};

pub fn handle_chaos_protobuf(
    chaos_disruption: &chaos_proto::chaos::Disruption,
) -> Result<Disruption, Error> {
    chaos_disruption.try_into()
}

impl TryFrom<&chaos_proto::chaos::Disruption> for Disruption {
    type Error = Error;
    fn try_from(proto: &chaos_proto::chaos::Disruption) -> Result<Disruption, Error> {
        Ok(Disruption {
            id: proto.get_id().to_string(),
            reference: None,
            contributor: proto.get_contributor().to_string(),
            publication_period: proto.get_publication_period().try_into()?,
            created_at: timestamp_to_datetime(proto.get_created_at()),
            updated_at: timestamp_to_datetime(proto.get_created_at()),
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
        let severity: Severity = proto.get_severity().into();
        let effect = severity.effect;
        let mut impacted_pt_objects = vec![];
        let mut informed_pt_objects = vec![];
        for entity in proto.get_informed_entities() {
            let entity = from(entity, effect)?;
            if let PtObjectType::Impacted(p) = entity {
                impacted_pt_objects.push(p);
            } else if let PtObjectType::Informed(p) = entity {
                informed_pt_objects.push(p);
            }
        }

        Ok(Impact {
            id: proto.get_id().to_string(),
            created_at: timestamp_to_datetime(proto.get_created_at()),
            updated_at: timestamp_to_datetime(proto.get_created_at()),
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
            severity,
            messages: proto.get_messages().iter().map(|m| m.into()).collect(),
            impacted_pt_objects,
            informed_pt_objects,
        })
    }
}

fn from(proto: &chaos_proto::chaos::PtObject, effect: Effect) -> Result<PtObjectType, Error> {
    use chaos_proto::chaos::PtObject_Type;
    let id = proto.get_uri().to_string();
    let created_at = timestamp_to_datetime(proto.get_created_at());
    let updated_at = timestamp_to_datetime(proto.get_updated_at());

    let pt_object = match proto.get_pt_object_type() {
        PtObject_Type::network => match effect {
            Effect::NoService => {
                PtObjectType::Impacted(Impacted::NetworkDeleted(NetworkDisruption {
                    id,
                    created_at,
                    updated_at,
                }))
            }
            _ => PtObjectType::Informed(Informed::Network(NetworkDisruption {
                id,
                created_at,
                updated_at,
            })),
        },
        PtObject_Type::line => match effect {
            Effect::NoService => PtObjectType::Impacted(Impacted::LineDeleted(LineDisruption {
                id,
                created_at,
                updated_at,
            })),
            _ => PtObjectType::Informed(Informed::Line(LineDisruption {
                id,
                created_at,
                updated_at,
            })),
        },
        PtObject_Type::route => match effect {
            Effect::NoService => PtObjectType::Impacted(Impacted::RouteDeleted(RouteDisruption {
                id,
                created_at,
                updated_at,
            })),
            _ => PtObjectType::Informed(Informed::Route(RouteDisruption {
                id,
                created_at,
                updated_at,
            })),
        },
        PtObject_Type::trip => match effect {
            Effect::NoService => PtObjectType::Impacted(Impacted::TripDeleted(TripDisruption {
                id,
                stop_times: vec![],
                company_id: None,
                physical_mode_id: None,
                headsign: None,
                created_at,
                updated_at,
            })),
            _ => PtObjectType::Informed(Informed::Trip(TripDisruption {
                id,
                stop_times: vec![],
                company_id: None,
                physical_mode_id: None,
                headsign: None,
                created_at,
                updated_at,
            })),
        },
        PtObject_Type::stop_point => match effect {
            Effect::NoService => {
                PtObjectType::Impacted(Impacted::StopPointDeleted(StopPointDisruption {
                    id,
                    created_at,
                    updated_at,
                }))
            }
            _ => PtObjectType::Informed(Informed::StopPoint(StopPointDisruption {
                id,
                created_at,
                updated_at,
            })),
        },
        PtObject_Type::stop_area => match effect {
            Effect::NoService => {
                PtObjectType::Impacted(Impacted::StopAreaDeleted(StopAreaDisruption {
                    id,
                    created_at,
                    updated_at,
                }))
            }
            _ => PtObjectType::Informed(Informed::StopArea(StopAreaDisruption {
                id,
                created_at,
                updated_at,
            })),
        },
        PtObject_Type::line_section => {
            let ls = proto.get_pt_line_section();
            let line = ls.get_line();
            let start = ls.get_start_point();
            let end = ls.get_end_point();
            let routes = ls.get_routes();
            let line_section = LineSectionDisruption {
                line: LineDisruption {
                    id: line.get_uri().to_string(),
                    created_at: timestamp_to_datetime(line.get_created_at()),
                    updated_at: timestamp_to_datetime(line.get_updated_at()),
                },
                start_sa: StopAreaDisruption {
                    id: start.get_uri().to_string(),
                    created_at: timestamp_to_datetime(start.get_created_at()),
                    updated_at: timestamp_to_datetime(start.get_updated_at()),
                },
                stop_sa: StopAreaDisruption {
                    id: end.get_uri().to_string(),
                    created_at: timestamp_to_datetime(end.get_created_at()),
                    updated_at: timestamp_to_datetime(end.get_updated_at()),
                },
                routes: routes
                    .iter()
                    .map(|r| RouteDisruption {
                        id: r.get_uri().to_string(),
                        created_at: timestamp_to_datetime(r.get_created_at()),
                        updated_at: timestamp_to_datetime(r.get_updated_at()),
                    })
                    .collect(),
            };
            match effect {
                Effect::NoService => PtObjectType::Impacted(Impacted::LineSection(line_section)),
                _ => return Err(format_err!("Not handled entity")),
            }
        }
        PtObject_Type::rail_section => return Err(format_err!("Not handled entity")),
        PtObject_Type::unkown_type => return Err(format_err!("Unknown entity")),
    };
    Ok(pt_object)
}

impl From<&chaos_proto::chaos::Severity> for Severity {
    fn from(proto: &chaos_proto::chaos::Severity) -> Severity {
        Severity {
            id: proto.get_id().to_string(),
            wording: proto.get_wording().to_string(),
            color: proto.get_color().to_string(),
            priority: proto.get_priority() as u32,
            effect: proto.get_effect().into(),
            created_at: timestamp_to_datetime(proto.get_created_at()),
            updated_at: timestamp_to_datetime(proto.get_updated_at()),
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
            created_at: timestamp_to_datetime(proto.get_created_at()),
            updated_at: timestamp_to_datetime(proto.get_created_at()),
        }
    }
}

impl TryFrom<&chaos_proto::gtfs_realtime::TimeRange> for DateTimePeriod {
    type Error = Error;
    fn try_from(proto: &chaos_proto::gtfs_realtime::TimeRange) -> Result<DateTimePeriod, Error> {
        let start = timestamp_to_datetime(proto.get_start());
        let end = timestamp_to_datetime(proto.get_end());
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
            created_at: timestamp_to_datetime(proto.get_created_at()),
            updated_at: timestamp_to_datetime(proto.get_created_at()),
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
            created_at: timestamp_to_datetime(proto.get_created_at()),
            updated_at: timestamp_to_datetime(proto.get_created_at()),
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
        // TODO : check if get_start_date() and get_end_date() are really timestamps ?
        let begin = timestamp_to_datetime(u64::from(proto.get_start_date()));
        let end = timestamp_to_datetime(u64::from(proto.get_end_date()));
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
