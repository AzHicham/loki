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

use crate::{
    chaos::sql_types::{ChannelType as ChannelTypeSQL, PtObjectType, SeverityEffect},
    chaos_proto, info,
    server_config::ChaosParams,
};
use anyhow::{bail, Context, Error};
use diesel::{
    pg::types::sql_types::Array,
    prelude::*,
    sql_types::{Bit, Date, Int4, Int8, Nullable, Text, Time, Timestamp, Uuid},
};
use launch::loki::{
    chrono::{NaiveDate, NaiveTime, Timelike},
    models::real_time_disruption::chaos_disruption::BlockedStopArea,
    tracing::error,
    NaiveDateTime,
};
use std::collections::{hash_map::Entry::Vacant, HashMap, HashSet};
use uuid::Uuid as Uid;

pub fn read_chaos_disruption_from_database(
    chaos_params: &ChaosParams,
    publication_period: (NaiveDate, NaiveDate),
    contributors: &[String],
) -> Result<Vec<chaos_proto::chaos::Disruption>, Error> {
    let connection = PgConnection::establish(&chaos_params.database)
        .context("Connection to chaos database failed")?;

    let mut disruption_maker = DisruptionMaker::default();

    let mut offset_query = 0_u32;

    info!("Querying chaos database {}", &chaos_params.database);
    loop {
        let res = diesel::sql_query(include_str!("query.sql"))
            .bind::<Date, _>(publication_period.1)
            .bind::<Date, _>(publication_period.0)
            .bind::<Date, _>(publication_period.1)
            .bind::<Array<Text>, _>(contributors)
            .bind::<Int8, _>(i64::from(chaos_params.batch_size))
            .bind::<Int8, _>(i64::from(offset_query))
            .load::<ChaosRow>(&connection);
        // Increment offset in query
        offset_query += chaos_params.batch_size;

        let rows = res?;
        if rows.is_empty() {
            break;
        }

        for row in rows {
            if let Err(ref err) = disruption_maker.read_disruption(&row) {
                error!("Error while handling a row from chaos database : {:?}", err);
            }
        }
    }
    let disruptions: Vec<_> = disruption_maker.disruptions.into_values().collect();
    info!(
        "Obtained {} disruptions from chaos database {}",
        disruptions.len(),
        &chaos_params.database
    );
    Ok(disruptions)
}

// In the SQL query to the Chaos database, we ask to sort the response's rows by disruption id.
// Then, we feed the rows to DisruptionMaker **in the order we receive them**.
//
// The information concerning all disruptions encountered is stored in the HashMap `disruptions`
// that maps a disruption_id to a protobuf object.
// The protobuf object is *incomplete* until all rows concerning this disruption are received.
//
// Note that we may receive the same "part" (i.e. tag/message/impact/...) of a disruption in several rows. But we don't want to "duplicate" this information in the protobuf object.
// To have an "easy" way to determine if a "part" has already been stored, we use the HashSet/HashMaps `tags_set` `properties_set` `impact_set` and `impact_object_set`
//
// Hence, when the disruption id changes (i.e. the row has a `disruption_id` not present in `disruptions.keys()`, this means  we have receive all information
// regarding the previous disruption.
// So when a new disruption id arrives, we "clear" these HashMap/HashSets.
#[derive(Default)]
struct DisruptionMaker {
    pub(crate) disruptions: HashMap<Uid, chaos_proto::chaos::Disruption>,

    tags_set: HashSet<Uid>,
    properties_set: HashSet<(String, String, String)>, //(type, key, value)
    impacts_set: HashMap<Uid, usize>,

    impact_object_set: ImpactMaker,
}

impl DisruptionMaker {
    pub fn read_disruption(&mut self, row: &ChaosRow) -> Result<(), Error> {
        let find_disruption = self.disruptions.entry(row.disruption_id);
        if let Vacant(entry) = find_disruption {
            let disruption = DisruptionMaker::make_disruption(row)?;

            // clear all set related to disruption
            self.impacts_set.clear();
            self.tags_set.clear();
            self.properties_set.clear();
            self.impact_object_set.clear();

            entry.insert(disruption);
        }
        // after previous insert unwrap is safe here!
        let disruption = self.disruptions.get_mut(&row.disruption_id).unwrap();

        DisruptionMaker::update_tags(&mut self.tags_set, row, disruption);
        DisruptionMaker::update_properties(&mut self.properties_set, row, disruption)?;
        DisruptionMaker::update_impacts(
            &mut self.impact_object_set,
            &mut self.impacts_set,
            row,
            disruption,
        )?;
        Ok(())
    }

    fn make_disruption(row: &ChaosRow) -> Result<chaos_proto::chaos::Disruption, Error> {
        let mut disruption = chaos_proto::chaos::Disruption::new();
        disruption.set_id(row.disruption_id.to_string());
        disruption.set_contributor(row.contributor.clone());
        if let Some(reference) = &row.disruption_reference {
            disruption.set_reference(reference.clone());
        }
        // Fill cause
        let cause = disruption.cause.mut_or_insert_default();
        cause.set_wording(row.cause_wording.clone());
        if let Some(category_name) = &row.category_name {
            cause
                .category
                .mut_or_insert_default()
                .set_name(category_name.clone());
        }
        // Fill publication_period
        let publication_period = disruption.publication_period.mut_or_insert_default();
        if let Some(start) = &row.disruption_start_publication_date {
            publication_period.set_start(u64::try_from(start.timestamp())?);
        }
        if let Some(end) = &row.disruption_end_publication_date {
            publication_period.set_end(u64::try_from(end.timestamp())?);
        }
        Ok(disruption)
    }

    fn update_tags(
        tags_set: &mut HashSet<Uid>,
        row: &ChaosRow,
        disruption: &mut chaos_proto::chaos::Disruption,
    ) {
        if let Some(tag_id) = row.tag_id {
            if tags_set.insert(tag_id) {
                let mut tag = chaos_proto::chaos::Tag::new();
                tag.set_id(tag_id.to_string());
                if let Some(name) = &row.tag_name {
                    tag.set_name(name.clone());
                }
                disruption.tags.push(tag);
            }
        }
    }

    fn update_properties(
        properties_set: &mut HashSet<(String, String, String)>,
        row: &ChaosRow,
        disruption: &mut chaos_proto::chaos::Disruption,
    ) -> Result<(), Error> {
        // type_ is here like an Uuid
        if let Some(r#type) = &row.property_type {
            let key = if let Some(key) = &row.property_key {
                key
            } else {
                bail!("Property has no key");
            };
            let value = if let Some(value) = &row.property_value {
                value
            } else {
                bail!("Property has no value");
            };
            let tuple = (r#type.clone(), key.clone(), value.clone());
            if properties_set.insert(tuple) {
                let mut property = chaos_proto::chaos::DisruptionProperty::new();
                property.set_type(r#type.clone());
                property.set_key(key.clone());
                property.set_value(value.clone());
                disruption.properties.push(property);
            }
        }
        Ok(())
    }

    fn update_impacts(
        impact_object_set: &mut ImpactMaker,
        impacts_set: &mut HashMap<Uid, usize>,
        row: &ChaosRow,
        disruption: &mut chaos_proto::chaos::Disruption,
    ) -> Result<(), Error> {
        let impact = if let Some(idx) = impacts_set.get(&row.impact_id) {
            // Impact already in disruption
            disruption.impacts.get_mut(*idx).unwrap()
        } else {
            // Or create a new impact We must then clear all  sub-objects sets belonging to impact
            let impact = ImpactMaker::make_impact(row)?;
            impact_object_set.clear();

            disruption.impacts.push(impact);
            let idx: usize = disruption.impacts.len() - 1;
            impacts_set.insert(row.impact_id, idx);
            disruption.impacts.last_mut().unwrap()
        };

        ImpactMaker::update_application_period(
            &mut impact_object_set.application_periods_set,
            row,
            impact,
        )?;
        ImpactMaker::update_messages(&mut impact_object_set.messages_set, row, impact);
        ImpactMaker::update_application_pattern(
            &mut impact_object_set.application_pattern_set,
            row,
            impact,
        )?;
        ImpactMaker::update_pt_objects(&mut impact_object_set.pt_object_set, row, impact)?;

        Ok(())
    }

    fn make_severity_effect(effect: &SeverityEffect) -> chaos_proto::gtfs_realtime::alert::Effect {
        use chaos_proto::gtfs_realtime::alert::Effect;
        match effect {
            SeverityEffect::NoService => Effect::NO_SERVICE,
            SeverityEffect::OtherEffect => Effect::OTHER_EFFECT,
            SeverityEffect::ModifiedService => Effect::MODIFIED_SERVICE,
            SeverityEffect::AdditionalService => Effect::ADDITIONAL_SERVICE,
            SeverityEffect::StopMoved => Effect::STOP_MOVED,
            SeverityEffect::SignificantDelays => Effect::SIGNIFICANT_DELAYS,
            SeverityEffect::ReducedService => Effect::REDUCED_SERVICE,
            SeverityEffect::UnknownEffect => Effect::UNKNOWN_EFFECT,
            SeverityEffect::Detour => Effect::DETOUR,
        }
    }
}

#[derive(Default)]
struct ImpactMaker {
    application_periods_set: HashSet<Uid>,
    application_pattern_set: HashSet<Uid>,
    messages_set: HashSet<Uid>,
    pt_object_set: HashSet<String>,
}

impl ImpactMaker {
    fn clear(&mut self) {
        self.application_periods_set.clear();
        self.application_pattern_set.clear();
        self.messages_set.clear();
        self.pt_object_set.clear();
    }

    fn make_impact(row: &ChaosRow) -> Result<chaos_proto::chaos::Impact, Error> {
        let mut impact = chaos_proto::chaos::Impact::new();
        impact.set_id(row.impact_id.to_string());
        impact.set_created_at(u64::try_from(row.impact_created_at.timestamp())?);
        if let Some(updated_at) = &row.impact_updated_at {
            impact.set_updated_at(u64::try_from(updated_at.timestamp())?);
        }
        // Fill severity
        let severity = impact.severity.mut_or_insert_default();
        severity.set_id(row.severity_id.to_string());
        severity.set_wording(row.severity_wording.clone());
        severity.set_priority(row.severity_priority);
        if let Some(color) = &row.severity_color {
            severity.set_color(color.clone());
        }
        let effect = row
            .severity_effect
            .as_ref()
            .unwrap_or(&SeverityEffect::UnknownEffect);
        severity.set_effect(DisruptionMaker::make_severity_effect(effect));
        Ok(impact)
    }

    fn update_application_period(
        application_periods_set: &mut HashSet<Uid>,
        row: &ChaosRow,
        impact: &mut chaos_proto::chaos::Impact,
    ) -> Result<(), Error> {
        if application_periods_set.insert(row.application_id) {
            let mut application_period = chaos_proto::gtfs_realtime::TimeRange::new();
            if let Some(start) = &row.application_start_date {
                application_period.set_start(u64::try_from(start.timestamp())?);
            }
            if let Some(end) = &row.application_end_date {
                application_period.set_end(u64::try_from(end.timestamp())?);
            }
            impact.application_periods.push(application_period);
        }
        Ok(())
    }

    fn update_messages(
        messages_set: &mut HashSet<Uid>,
        row: &ChaosRow,
        impact: &mut chaos_proto::chaos::Impact,
    ) {
        if let Some(message_id) = row.message_id {
            if messages_set.insert(message_id) {
                let mut message = chaos_proto::chaos::Message::new();
                if let Some(text) = &row.message_text {
                    message.set_text(text.clone());
                }
                let channel = message.channel.mut_or_insert_default();
                if let Some(name) = &row.channel_name {
                    channel.set_name(name.clone());
                }
                if let Some(channel_id) = &row.channel_id {
                    channel.set_id(channel_id.to_string());
                }
                if let Some(content_type) = &row.channel_content_type {
                    channel.set_content_type(content_type.clone());
                }
                for channel_type in row.channel_type.iter().flatten() {
                    channel
                        .types
                        .push(ImpactMaker::make_channel_type(channel_type).into());
                }
                impact.messages.push(message);
            }
        }
    }

    fn make_channel_type(channel_type: &ChannelTypeSQL) -> chaos_proto::chaos::channel::Type {
        use chaos_proto::chaos::channel::Type;
        match channel_type {
            ChannelTypeSQL::Title => Type::title,
            ChannelTypeSQL::Beacon => Type::beacon,
            ChannelTypeSQL::Twitter => Type::twitter,
            ChannelTypeSQL::Notification => Type::notification,
            ChannelTypeSQL::Sms => Type::sms,
            ChannelTypeSQL::Facebook => Type::facebook,
            ChannelTypeSQL::Email => Type::email,
            ChannelTypeSQL::Mobile => Type::mobile,
            ChannelTypeSQL::Web => Type::web,
        }
    }

    fn update_application_pattern(
        application_pattern_set: &mut HashSet<Uid>,
        row: &ChaosRow,
        impact: &mut chaos_proto::chaos::Impact,
    ) -> Result<(), Error> {
        if let Some(pattern_id) = row.pattern_id {
            if application_pattern_set.insert(pattern_id) {
                let mut pattern = chaos_proto::chaos::Pattern::new();
                if let Some(start_date) = row.pattern_start_date {
                    pattern.set_start_date(u32::try_from(start_date.and_hms(0, 0, 0).timestamp())?);
                }
                if let Some(end_date) = row.pattern_end_date {
                    pattern.set_end_date(u32::try_from(end_date.and_hms(0, 0, 0).timestamp())?);
                }
                // time_slot_begin && time_slot_end have always the same size
                // even after filter_map
                // thanks to the sql query
                let time_slots_iter = row
                    .time_slot_begin
                    .iter()
                    .flatten()
                    .zip(row.time_slot_end.iter().flatten());

                if let Some(week_pattern) = &row.pattern_weekly_pattern {
                    // Bit(7) is coded into a vec of size 5
                    // like [0, 0, 0, 7, 248]
                    // with last element corresponding to the bit pattern
                    // example : 248 -> "11111000"
                    if week_pattern.len() == 5 {
                        let proto_week = pattern.week_pattern.mut_or_insert_default();
                        // unwrap is safe thanks to len check before
                        let char_bit = format!("{:b}", week_pattern.last().unwrap());
                        let mut iter = char_bit.chars();
                        proto_week.set_monday(iter.next() == Some('1'));
                        proto_week.set_tuesday(iter.next() == Some('1'));
                        proto_week.set_wednesday(iter.next() == Some('1'));
                        proto_week.set_thursday(iter.next() == Some('1'));
                        proto_week.set_friday(iter.next() == Some('1'));
                        proto_week.set_saturday(iter.next() == Some('1'));
                        proto_week.set_sunday(iter.next() == Some('1'));
                    } else {
                        bail!("pattern_weekly_pattern must have a size of 7");
                    }
                }

                for (begin, end) in time_slots_iter {
                    let mut time_slot = chaos_proto::chaos::TimeSlot::new();
                    time_slot.set_begin(begin.num_seconds_from_midnight());
                    time_slot.set_end(end.num_seconds_from_midnight());
                    pattern.time_slots.push(time_slot);
                }
                impact.application_patterns.push(pattern);
            }
        }
        Ok(())
    }

    fn update_pt_objects(
        pt_object_set: &mut HashSet<String>,
        row: &ChaosRow,
        impact: &mut chaos_proto::chaos::Impact,
    ) -> Result<(), Error> {
        let id = if let Some(id) = &row.ptobject_uri {
            id.clone()
        } else {
            bail!("PtObject has no uri");
        };

        let pt_object_type = row.ptobject_type.clone();

        // Early exit if we already pushed a pt_object in impacts.informed_entities[]
        // except for Line/rail section (they can be updated)
        if pt_object_set.contains(&id)
            && pt_object_type != PtObjectType::RailSection
            && pt_object_type != PtObjectType::LineSection
        {
            return Ok(());
        }
        pt_object_set.insert(id.clone());

        use chaos_proto::chaos::pt_object;
        match pt_object_type {
            PtObjectType::LineSection => {
                // check if we need to create a new line section
                // or just update it ie. push a new route into it
                let found_line_section = impact
                    .informed_entities
                    .iter_mut()
                    .filter(|pt_object| match pt_object.pt_object_type {
                        Some(pt_object_type) => match pt_object_type.enum_value() {
                            Ok(object_type) => object_type == pt_object::Type::line_section,
                            _ => false,
                        },
                        None => false,
                    })
                    .find(|pt_object| matches!(&pt_object.uri, Some(uri) if uri == &id));

                match found_line_section {
                    Some(pt_object) => {
                        // we found line_section so we push a new route
                        // if not already in line_section.routes[]
                        if let Some(route_id) = &row.ls_route_uri {
                            let found_route =
                                pt_object.pt_line_section.routes.iter().find(
                                    |route| matches!(&route.uri, Some(uri) if uri == route_id),
                                );
                            if found_route.is_none() {
                                let mut route = chaos_proto::chaos::PtObject::new();
                                route.set_uri(route_id.clone());
                                route.set_pt_object_type(pt_object::Type::route);
                                pt_object
                                    .pt_line_section
                                    .mut_or_insert_default()
                                    .routes
                                    .push(route);
                            }
                        }
                    }
                    None => {
                        let mut pt_object = chaos_proto::chaos::PtObject::new();
                        pt_object.set_uri(id.clone());
                        pt_object.set_pt_object_type(pt_object::Type::line_section);
                        let line_section = pt_object.pt_line_section.mut_or_insert_default();
                        if let Some(line_id) = &row.ls_line_uri {
                            let line = line_section.line.mut_or_insert_default();
                            line.set_uri(line_id.clone());
                            line.set_pt_object_type(pt_object::Type::line);
                        }
                        if let Some(start) = &row.ls_start_uri {
                            let start_stop = line_section.start_point.mut_or_insert_default();
                            start_stop.set_uri(start.clone());
                            start_stop.set_pt_object_type(pt_object::Type::stop_area);
                        }
                        if let Some(end) = &row.ls_end_uri {
                            let end_stop = line_section.end_point.mut_or_insert_default();
                            end_stop.set_uri(end.clone());
                            end_stop.set_pt_object_type(pt_object::Type::stop_area);
                        }
                        if let Some(route_id) = &row.ls_route_uri {
                            let mut route = chaos_proto::chaos::PtObject::new();
                            route.set_uri(route_id.clone());
                            route.set_pt_object_type(pt_object::Type::route);
                            line_section.routes.push(route);
                        }

                        impact.informed_entities.push(pt_object);
                    }
                }
            }
            PtObjectType::RailSection => {
                // check if we need to create a new rail section or just push a new route into it
                let found_rail_section = impact
                    .informed_entities
                    .iter_mut()
                    .filter(|pt_object| match pt_object.pt_object_type {
                        Some(pt_object_type) => match pt_object_type.enum_value() {
                            Ok(object_type) => object_type == pt_object::Type::rail_section,
                            _ => false,
                        },
                        None => false,
                    })
                    .find(|pt_object| matches!(&pt_object.uri, Some(uri) if uri == &id));

                match found_rail_section {
                    Some(pt_object) => {
                        // we found rail_section so we push a new route
                        // if not already in rail_section.routes[]
                        if let Some(route_id) = &row.rs_route_uri {
                            let found_route =
                                pt_object.pt_rail_section.routes.iter().find(
                                    |route| matches!(&route.uri, Some(uri) if uri == route_id),
                                );
                            if found_route.is_none() {
                                let mut route = chaos_proto::chaos::PtObject::new();
                                route.set_uri(route_id.clone());
                                route.set_pt_object_type(pt_object::Type::route);
                                pt_object
                                    .pt_rail_section
                                    .mut_or_insert_default()
                                    .routes
                                    .push(route);
                            }
                        }
                    }
                    None => {
                        let mut pt_object = chaos_proto::chaos::PtObject::new();
                        pt_object.set_uri(id.clone());
                        pt_object.set_pt_object_type(pt_object::Type::rail_section);
                        let rail_section = pt_object.pt_rail_section.mut_or_insert_default();
                        if let Some(line_id) = &row.rs_line_uri {
                            let line = rail_section.line.mut_or_insert_default();
                            line.set_uri(line_id.clone());
                            line.set_pt_object_type(pt_object::Type::line);
                        }
                        if let Some(start) = &row.rs_start_uri {
                            let start_stop = rail_section.start_point.mut_or_insert_default();
                            start_stop.set_uri(start.clone());
                            start_stop.set_pt_object_type(pt_object::Type::stop_area);
                        }
                        if let Some(end) = &row.rs_end_uri {
                            let end_stop = rail_section.end_point.mut_or_insert_default();
                            end_stop.set_uri(end.clone());
                            end_stop.set_pt_object_type(pt_object::Type::stop_area);
                        }
                        if let Some(route_id) = &row.rs_route_uri {
                            let mut route = chaos_proto::chaos::PtObject::new();
                            route.set_uri(route_id.clone());
                            route.set_pt_object_type(pt_object::Type::route);
                            rail_section.routes.push(route);
                        }

                        if let Some(blocked_stop_area) = &row.rs_blocked_sa {
                            let blocked_stop_area = serde_json::from_str::<Vec<BlockedStopArea>>(
                                blocked_stop_area.as_str(),
                            )
                            .with_context(|| {
                                "Could not deserialize blocked_stop_area of rail_section"
                            })?;
                            for stop_area in blocked_stop_area {
                                let mut pt_object_ordered =
                                    chaos_proto::chaos::OrderedPtObject::new();
                                pt_object_ordered.set_uri(stop_area.id);
                                pt_object_ordered.set_order(stop_area.order);
                                rail_section.blocked_stop_areas.push(pt_object_ordered);
                            }
                        }

                        impact.informed_entities.push(pt_object);
                    }
                }
            }
            _ => {
                let mut pt_object = chaos_proto::chaos::PtObject::new();
                pt_object.set_uri(id);
                pt_object.set_pt_object_type(ImpactMaker::make_pt_object_type(&pt_object_type));
                impact.informed_entities.push(pt_object);
            }
        };

        Ok(())
    }

    fn make_pt_object_type(pt_object_type: &PtObjectType) -> chaos_proto::chaos::pt_object::Type {
        use chaos_proto::chaos::pt_object::Type;
        match pt_object_type {
            PtObjectType::StopArea => Type::stop_area,
            PtObjectType::StopPoint => Type::stop_point,
            PtObjectType::LineSection => Type::line_section,
            PtObjectType::RailSection => Type::rail_section,
            PtObjectType::Route => Type::route,
            PtObjectType::Line => Type::line,
            PtObjectType::Network => Type::network,
        }
    }
}

// Remove ChaosRow when PR https://github.com/diesel-rs/diesel/pull/2254 is merged
// and use model_v2
#[derive(Queryable, QueryableByName, Debug)]
pub struct ChaosRow {
    // Disruptions field
    #[sql_type = "Uuid"]
    pub disruption_id: Uid,
    #[sql_type = "Nullable<Text>"]
    pub disruption_reference: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub disruption_start_publication_date: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub disruption_end_publication_date: Option<NaiveDateTime>,
    #[sql_type = "Text"]
    pub contributor: String,

    // Cause fields
    #[sql_type = "Uuid"]
    pub cause_id: Uid,
    #[sql_type = "Text"]
    pub cause_wording: String,
    // Category fields
    #[sql_type = "Nullable<Text>"]
    pub category_name: Option<String>,

    // Tag fields
    #[sql_type = "Nullable<Uuid>"]
    pub tag_id: Option<Uid>,
    #[sql_type = "Nullable<Text>"]
    pub tag_name: Option<String>,

    // Impact fields
    #[sql_type = "Uuid"]
    pub impact_id: Uid,
    #[sql_type = "Timestamp"]
    pub impact_created_at: NaiveDateTime,
    #[sql_type = "Nullable<Timestamp>"]
    pub impact_updated_at: Option<NaiveDateTime>,

    // Application period fields
    #[sql_type = "Uuid"]
    pub application_id: Uid,
    #[sql_type = "Nullable<Timestamp>"]
    pub application_start_date: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub application_end_date: Option<NaiveDateTime>,

    // Severity fields
    #[sql_type = "Uuid"]
    pub severity_id: Uid,
    #[sql_type = "Text"]
    pub severity_wording: String,
    #[sql_type = "Nullable<Text>"]
    pub severity_color: Option<String>,
    #[sql_type = "Int4"]
    pub severity_priority: i32,
    #[sql_type = "Nullable<crate::chaos::sql_types::severity_effect>"]
    pub severity_effect: Option<SeverityEffect>,

    // Ptobject fields
    #[sql_type = "crate::chaos::sql_types::pt_object_type"]
    pub ptobject_type: PtObjectType,
    #[sql_type = "Nullable<Text>"]
    pub ptobject_uri: Option<String>,

    // Ptobject line_section fields
    #[sql_type = "Nullable<Text>"]
    pub ls_line_uri: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub ls_start_uri: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub ls_end_uri: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub ls_route_uri: Option<String>,

    // Ptobject rail_section fields
    #[sql_type = "Nullable<Text>"]
    pub rs_line_uri: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub rs_start_uri: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub rs_end_uri: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub rs_route_uri: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub rs_blocked_sa: Option<String>,

    // Message fields
    #[sql_type = "Nullable<Uuid>"]
    pub message_id: Option<Uid>,
    #[sql_type = "Nullable<Text>"]
    pub message_text: Option<String>,
    // Channel fields
    #[sql_type = "Nullable<Uuid>"]
    pub channel_id: Option<Uid>,
    #[sql_type = "Nullable<Text>"]
    pub channel_name: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub channel_content_type: Option<String>,
    #[sql_type = "Array<Nullable<crate::chaos::sql_types::channel_type_enum>>"]
    pub channel_type: Vec<Option<ChannelTypeSQL>>,

    //  Property & Associate property fields
    #[sql_type = "Nullable<Text>"]
    pub property_value: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub property_key: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub property_type: Option<String>,

    // Pattern & TimeSlot fields
    #[sql_type = "Nullable<Uuid>"]
    pub pattern_id: Option<Uid>,
    #[sql_type = "Nullable<Date>"]
    pub pattern_start_date: Option<NaiveDate>,
    #[sql_type = "Nullable<Date>"]
    pub pattern_end_date: Option<NaiveDate>,
    #[sql_type = "Nullable<Bit>"]
    pub pattern_weekly_pattern: Option<Vec<u8>>,
    #[sql_type = "Array<Nullable<Time>>"]
    pub time_slot_begin: Vec<Option<NaiveTime>>,
    #[sql_type = "Array<Nullable<Time>>"]
    pub time_slot_end: Vec<Option<NaiveTime>>,
}
