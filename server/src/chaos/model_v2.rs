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
    chaos::{
        schema::{
            application_periods, associate_disruption_property, associate_disruption_tag,
            associate_impact_pt_object as aipt, category, cause, channel, channel_type,
            contributor, disruption, impact, line_section, message, pattern, property, pt_object,
            rail_section, severity, tag, time_slot,
        },
        sql_types::{ChannelType, DisruptionStatus, ImpactStatus, PtObjectType},
    },
    server_config::ChaosParams,
};
use anyhow::Error;
use diesel::prelude::*;
use loki_launch::loki::{
    chrono::{NaiveDate, NaiveTime},
    NaiveDateTime,
};
use uuid::Uuid as Uid;

pub fn chaos_disruption_from_database(
    config: &ChaosParams,
    publication_period: (NaiveDateTime, NaiveDateTime),
) -> Result<(), Error> {
    let connection = PgConnection::establish(&config.chaos_database)?;

    let query_join = disruption::table
        .inner_join(contributor::table)
        .inner_join(cause::table)
        .left_join(category::table.on(category::id.nullable().eq(cause::category_id)))
        .left_join(associate_disruption_tag::table)
        .left_join(tag::table.on(tag::id.eq(associate_disruption_tag::tag_id)))
        .inner_join(impact::table)
        .inner_join(application_periods::table.on(application_periods::impact_id.eq(impact::id)))
        .inner_join(severity::table.on(severity::id.nullable().eq(impact::severity_id)))
        .inner_join(aipt::table.on(aipt::impact_id.eq(impact::id)))
        .inner_join(pt_object::table.on(aipt::pt_object_id.eq(pt_object::id)))
        .left_join(line_section::table.on(line_section::object_id.eq(pt_object::id.nullable())))
        .left_join(rail_section::table.on(rail_section::object_id.eq(pt_object::id.nullable())))
        .left_join(message::table.on(message::impact_id.eq(impact::id.nullable())))
        .left_join(channel::table.on(channel::id.nullable().eq(message::channel_id)))
        .left_join(channel_type::table.on(channel_type::channel_id.eq(channel::id.nullable())))
        .left_join(associate_disruption_property::table)
        .left_join(property::table.on(property::id.eq(associate_disruption_property::property_id)))
        .left_join(pattern::table.on(pattern::impact_id.eq(impact::id.nullable())))
        .left_join(time_slot::table.on(time_slot::pattern_id.eq(pattern::id.nullable())));

    let query_select = query_join.select((
        disruption::id,
        disruption::reference,
        disruption::status,
        disruption::start_publication_date,
        disruption::end_publication_date,
        disruption::created_at,
        disruption::updated_at,
        contributor::contributor_code,
        cause::id,
        cause::wording,
        cause::is_visible,
        cause::created_at,
        cause::updated_at,
        category::name.nullable(),
        category::id.nullable(),
        category::created_at.nullable(),
        category::updated_at.nullable(),
        tag::id.nullable(),
        tag::name.nullable(),
        tag::is_visible.nullable(),
        tag::created_at.nullable(),
        tag::updated_at.nullable(),
        impact::id,
        impact::status,
        impact::disruption_id,
        impact::created_at,
        impact::updated_at,
        application_periods::id,
        application_periods::start_date,
        application_periods::end_date,
        severity::id,
        severity::wording,
        severity::color,
        severity::is_visible,
        severity::priority,
        severity::created_at,
        severity::updated_at,
        pt_object::id,
        pt_object::type_.nullable(),
        pt_object::uri,
        pt_object::created_at,
        pt_object::updated_at,
        message::id.nullable(),
        message::text.nullable(),
        message::created_at.nullable(),
        message::updated_at.nullable(),
        channel::id.nullable(),
        channel::name.nullable(),
        channel::content_type.nullable(),
        channel::max_size.nullable(),
        channel::created_at.nullable(),
        channel::updated_at.nullable(),
        channel_type::id.nullable(),
        channel_type::name.nullable(),
        associate_disruption_property::value.nullable(),
        property::key.nullable(),
        property::type_.nullable(),
        pattern::start_date.nullable(),
        pattern::end_date.nullable(),
        pattern::weekly_pattern.nullable(),
        pattern::id.nullable(),
        time_slot::begin.nullable(),
        time_slot::end.nullable(),
        time_slot::id.nullable(),
    ));

    let query_filter = query_select
        .filter(
            (disruption::start_publication_date
                .le(publication_period.1)
                .and(disruption::end_publication_date.ge(publication_period.0)))
            .or(disruption::start_publication_date
                .le(publication_period.1)
                .and(disruption::end_publication_date.is_null())),
        )
        .filter(contributor::contributor_code.eq_any(&config.chaos_contributors))
        .filter(disruption::status.eq(DisruptionStatus::Published))
        .filter(impact::status.eq(ImpactStatus::Published));

    let query_final = query_filter
        .order((
            disruption::id,
            cause::id,
            tag::id,
            impact::id,
            message::id,
            channel::id,
            channel_type::id,
        ))
        .limit(config.chaos_batch_size as i64)
        .offset(config.chaos_batch_size as i64);

    let res = query_final.load::<ChaosDisruption>(&connection);

    println!("{:?}", res);

    Ok(())
}

#[derive(Queryable, Debug)]
pub struct ChaosDisruption {
    // Disruptions field
    pub disruption_id: Uid,
    pub disruption_reference: Option<String>,
    pub disruption_status: DisruptionStatus,
    pub disruption_start_publication_date: Option<NaiveDateTime>,
    pub disruption_end_publication_date: Option<NaiveDateTime>,
    pub disruption_created_at: NaiveDateTime,
    pub disruption_updated_at: Option<NaiveDateTime>,
    pub contributor: String,
    // Cause fields
    pub cause_id: Uid,
    pub cause_wording: String,
    pub cause_visible: bool,
    pub cause_created_at: NaiveDateTime,
    pub cause_updated_at: Option<NaiveDateTime>,
    // Category fields
    pub category_name: Option<String>,
    pub category_id: Option<Uid>,
    pub category_created_at: Option<NaiveDateTime>,
    pub category_updated_at: Option<NaiveDateTime>,
    // Tag fields
    pub tag_id: Option<Uid>,
    pub tag_name: Option<String>,
    pub tag_is_visible: Option<bool>,
    pub tag_created_at: Option<NaiveDateTime>,
    pub tag_updated_at: Option<NaiveDateTime>,
    // Impact fields
    pub impact_id: Uid,
    pub impact_status: ImpactStatus,
    pub impact_disruption_id: Option<Uid>,
    pub impact_created_at: NaiveDateTime,
    pub impact_updated_at: Option<NaiveDateTime>,
    // Application period fields
    pub application_id: Uid,
    pub application_start_date: Option<NaiveDateTime>,
    pub application_end_date: Option<NaiveDateTime>,
    // Severity fields
    pub severity_id: Uid,
    pub severity_wording: String,
    pub severity_color: Option<String>,
    pub severity_is_visible: bool,
    pub severity_priority: i32,
    pub severity_created_at: NaiveDateTime,
    pub severity_updated_at: Option<NaiveDateTime>,
    // Ptobject fields
    pub pt_object_id: Uid,
    pub pt_object_type: Option<PtObjectType>,
    pub pt_object_uri: Option<String>,
    pub pt_object_created_at: NaiveDateTime,
    pub pt_object_updated_at: Option<NaiveDateTime>,
    // Ptobject line_section fields
    // pub ls_line_uri: Option<String>,
    // pub ls_line_created_at: Option<NaiveDateTime>,
    // pub ls_line_updated_at: Option<NaiveDateTime>,
    // pub ls_start_uri: Option<String>,
    // pub ls_start_created_at: Option<NaiveDateTime>,
    // pub ls_start_updated_at: Option<NaiveDateTime>,
    // pub ls_end_uri: Option<String>,
    // pub ls_end_created_at: Option<NaiveDateTime>,
    // pub ls_end_updated_at: Option<NaiveDateTime>,
    // pub ls_route_id: Option<Uid>,
    // pub ls_route_uri: Option<String>,
    // pub ls_route_created_at: Option<NaiveDateTime>,
    // pub ls_route_updated_at: Option<NaiveDateTime>,
    // Ptobject rail_section fields
    // pub rs_line_uri: Option<String>,
    // pub rs_line_created_at: Option<NaiveDateTime>,
    // pub rs_line_updated_at: Option<NaiveDateTime>,
    // pub rs_start_uri: Option<String>,
    // pub rs_start_created_at: Option<NaiveDateTime>,
    // pub rs_start_updated_at: Option<NaiveDateTime>,
    // pub rs_end_uri: Option<String>,
    // pub rs_end_created_at: Option<NaiveDateTime>,
    // pub rs_end_updated_at: Option<NaiveDateTime>,
    // pub rs_route_id: Option<Uid>,
    // pub rs_route_uri: Option<String>,
    // pub rs_route_created_at: Option<NaiveDateTime>,
    // pub rs_route_updated_at: Option<NaiveDateTime>,
    // Message fields
    pub message_id: Option<Uid>,
    pub message_text: Option<String>,
    pub message_created_at: Option<NaiveDateTime>,
    pub message_updated_at: Option<NaiveDateTime>,
    // Channel fields
    pub channel_id: Option<Uid>,
    pub channel_name: Option<String>,
    pub channel_content_type: Option<String>,
    pub channel_max_size: Option<i32>,
    pub channel_created_at: Option<NaiveDateTime>,
    pub channel_updated_at: Option<NaiveDateTime>,
    pub channel_type_id: Option<Uid>,
    pub channel_type: Option<ChannelType>,
    //  Property & Associate property fields
    pub property_value: Option<String>,
    pub property_key: Option<String>,
    pub property_type: Option<String>,
    // Pattern & TimeSlot fields
    pub pattern_start_date: Option<NaiveDate>,
    pub pattern_end_date: Option<NaiveDate>,
    pub pattern_weekly_pattern: Option<Vec<u8>>,
    pub pattern_id: Option<Uid>,
    pub time_slot_begin: Option<NaiveTime>,
    pub time_slot_end: Option<NaiveTime>,
    pub time_slot_id: Option<Uid>,
}
