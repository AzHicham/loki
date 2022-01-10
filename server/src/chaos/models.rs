use crate::{
    chaos::schema::{
        application_periods, associate_disruption_property, associate_disruption_tag,
        associate_impact_pt_object as aipt, category, cause, channel, channel_type, contributor,
        disruption, impact, line_section, message, pattern, property, pt_object, rail_section,
        severity, tag, time_slot,
    },
    server_config::ChaosParams,
};

use crate::chaos::sql_types::DisruptionStatus;
use anyhow::Error;
use diesel::sql_types::Text;
use diesel::sql_types::Timestamp;
use diesel::{debug_query, prelude::*, Queryable};
use launch::loki::NaiveDateTime;
use uuid::Uuid;

fn establish_connection(database_url: &str) -> ConnectionResult<PgConnection> {
    PgConnection::establish(&database_url)
}

pub fn chaos_disruption_from_database(
    config: &ChaosParams,
    /* publication_period: (NaiveDateTime, NaiveDateTime),
    contributors: &[&str],*/
) -> Result<(), Error> {
    let connection = establish_connection(&config.chaos_database)?;

    let query = disruption::table
        .inner_join(contributor::table)
        .inner_join(cause::table)
        .left_join(category::table.on(category::id.nullable().eq(cause::category_id)))
        .left_join(associate_disruption_tag::table)
        .left_join(tag::table.on(tag::id.eq(associate_disruption_tag::tag_id)))
        .inner_join(impact::table)
        .inner_join(application_periods::table.on(application_periods::impact_id.eq(impact::id)))
        .inner_join(severity::table.on(severity::id.nullable().eq(impact::severity_id)))
        .inner_join(aipt::table.on(aipt::impact_id.eq(impact::id)))
        .inner_join(pt_object::table.on(aipt::impact_id.eq(pt_object::id)))
        .left_join(line_section::table.on(line_section::object_id.eq(pt_object::id.nullable())))
        .left_join(rail_section::table.on(rail_section::object_id.eq(pt_object::id.nullable())))
        .left_join(message::table.on(message::impact_id.eq(impact::id.nullable())))
        .left_join(channel::table.on(channel::id.nullable().eq(message::channel_id)))
        .left_join(channel_type::table.on(channel_type::channel_id.eq(channel::id.nullable())))
        .left_join(associate_disruption_property::table)
        .left_join(property::table.on(property::id.eq(associate_disruption_property::property_id)))
        .left_join(pattern::table.on(pattern::impact_id.eq(impact::id.nullable())))
        .left_join(time_slot::table.on(time_slot::pattern_id.eq(pattern::id.nullable())))
        .select((disruption::created_at, disruption::updated_at))
        .limit(5);

    let debug = debug_query::<diesel::pg::Pg, _>(&query);
    println!("Displaying {} posts", debug);
    let res = query
        .load::<(NaiveDateTime, Option<NaiveDateTime>)>(&connection)
        .expect("Error loading posts");
    for r in res {
        println!("{:?}", r);
    }

    let resuls: Result<Vec<ChaosDisruption>, diesel::result::Error> =
        diesel::sql_query(QUERY).load(&connection);
    println!("{:?}", resuls);

    Ok(())
}

#[derive(Queryable, QueryableByName, Debug)]
pub struct ChaosDisruption {
    // Disruptions field
    #[sql_type = "diesel::sql_types::Uuid"]
    pub disruption_id: Uuid,
    #[sql_type = "diesel::sql_types::Nullable<Text>"]
    pub disruption_reference: Option<String>,
    #[sql_type = "crate::chaos::sql_types::disruption_status"]
    pub disruption_status: DisruptionStatus,
    #[sql_type = "diesel::sql_types::Nullable<Timestamp>"]
    pub disruption_start_publication_date: Option<NaiveDateTime>,
    #[sql_type = "diesel::sql_types::Nullable<Timestamp>"]
    pub disruption_end_publication_date: Option<NaiveDateTime>,
    #[sql_type = "diesel::sql_types::Timestamp"]
    pub disruption_created_at: NaiveDateTime,
    #[sql_type = "diesel::sql_types::Nullable<Timestamp>"]
    pub disruption_updated_at: Option<NaiveDateTime>,
    #[sql_type = "diesel::sql_types::Text"]
    pub contributor: String,
    // Cause fields
    // pub cause_id: Option<NaiveDateTime>,
    // pub cause_wording: Uuid,
    // pub cause_visible: Uuid,
    // pub cause_created_at: NaiveDateTime,
    // pub cause_updated_at: Option<NaiveDateTime>,
    // // Category fields
    // pub category_name: Option<NaiveDateTime>,
    // pub category_id: Uuid,
    // pub category_created_at: NaiveDateTime,
    // pub category_updated_at: Option<NaiveDateTime>,
    // // Tag fields
    // pub tag_id: Option<NaiveDateTime>,
    // pub tag_name: Uuid,
    // pub tag_is_visible: Uuid,
    // pub tag_created_at: NaiveDateTime,
    // pub tag_updated_at: Option<NaiveDateTime>,
    // // Application period fields
    // pub application_id: Option<NaiveDateTime>,
    // pub application_created_at: NaiveDateTime,
    // pub application_updated_at: Option<NaiveDateTime>,
    // // Severity fields
    // pub severity_id: Uuid,
    // pub severity_wording: Uuid,
    // pub severity_color: Uuid,
    // pub severity_is_visible: Uuid,
    // pub severity_priority: Uuid,
    // pub severity_created_at: NaiveDateTime,
    // pub severity_updated_at: Option<NaiveDateTime>,
    // // Ptobject fields
    // pub ptobject_id: Uuid,
    // pub ptobject_type: Uuid,
    // pub ptobject_uri: Uuid,
    // pub ptobject_created_at: NaiveDateTime,
    // pub ptobject_updated_at: Option<NaiveDateTime>,
    // // Ptobject line_section optional fields
    // // Ptobject rail_section optional fields
    // // Message fields
    // // Ptobject fields
    // pub message_id: Uuid,
    // pub message_text: Uuid,
    // pub message_created_at: NaiveDateTime,
    // pub message_updated_at: Option<NaiveDateTime>,
    // // Channel fields
    // pub channel_id: Uuid,
    // pub channel_name: Uuid,
    // pub channel_content_type: Uuid,
    // pub channel_max_size: Uuid,
    // pub channel_created_at: NaiveDateTime,
    // pub channel_updated_at: Option<NaiveDateTime>,
    // pub channel_type_id: Uuid,
    // pub channel_type: Uuid,
    // //  Property & Associate property fields
    // pub property_value: Uuid,
    // pub property_key: Uuid,
    // pub property_type: NaiveDateTime,
    // // Channel fields
    // pub pattern_start_date: Uuid,
    // pub pattern_end_date: Uuid,
    // pub pattern_weekly_pattern: Uuid,
    // pub pattern_id: Uuid,
    // pub time_slot_begin: NaiveDateTime,
    // pub time_slot_end: Option<NaiveDateTime>,
    // pub time_slot_id: Uuid,
}

const QUERY: &str = r#"
SELECT
d.id as disruption_id, d.reference as disruption_reference, d.note as disruption_note,
d.status as disruption_status,
extract(epoch from d.start_publication_date  AT TIME ZONE 'UTC') :: bigint as
disruption_start_publication_date,
extract(epoch from d.end_publication_date  AT TIME ZONE 'UTC') :: bigint as
disruption_end_publication_date,
extract(epoch from d.created_at  AT TIME ZONE 'UTC') :: bigint as disruption_created_at,
extract(epoch from d.updated_at  AT TIME ZONE 'UTC') :: bigint as disruption_updated_at,
co.contributor_code as contributor,

c.id as cause_id, c.wording as cause_wording,
c.is_visible as cause_visible,
extract(epoch from c.created_at  AT TIME ZONE 'UTC') :: bigint as cause_created_at,
extract(epoch from c.updated_at  AT TIME ZONE 'UTC') :: bigint as cause_updated_at,

cat.name as category_name, cat.id as category_id,
extract(epoch from c.created_at  AT TIME ZONE 'UTC') :: bigint as category_created_at,
extract(epoch from c.updated_at  AT TIME ZONE 'UTC') :: bigint as category_updated_at,

t.id as tag_id, t.name as tag_name, t.is_visible as tag_is_visible,
extract(epoch from t.created_at  AT TIME ZONE 'UTC') :: bigint as tag_created_at,
extract(epoch from t.updated_at  AT TIME ZONE 'UTC') :: bigint as tag_updated_at,

i.id as impact_id, i.status as impact_status, i.disruption_id as impact_disruption_id,
extract(epoch from i.created_at  AT TIME ZONE 'UTC') :: bigint as impact_created_at,
extract(epoch from i.updated_at  AT TIME ZONE 'UTC') :: bigint as impact_updated_at,

a.id as application_id,
extract(epoch from a.start_date  AT TIME ZONE 'UTC') :: bigint as application_start_date,
extract(epoch from a.end_date  AT TIME ZONE 'UTC') :: bigint as application_end_date,

s.id as severity_id, s.wording as severity_wording, s.color as severity_color,
s.is_visible as severity_is_visible, s.priority as severity_priority,
s.effect as severity_effect,
extract(epoch from s.created_at  AT TIME ZONE 'UTC') :: bigint as severity_created_at,
extract(epoch from s.updated_at  AT TIME ZONE 'UTC') :: bigint as severity_updated_at,

p.id as ptobject_id, p.type as ptobject_type, p.uri as ptobject_uri,
extract(epoch from p.created_at  AT TIME ZONE 'UTC') :: bigint as ptobject_created_at,
extract(epoch from p.updated_at  AT TIME ZONE 'UTC') :: bigint as ptobject_updated_at,

ls_line.uri as ls_line_uri,
extract(epoch from ls_line.created_at  AT TIME ZONE 'UTC') :: bigint as ls_line_created_at,
extract(epoch from ls_line.updated_at  AT TIME ZONE 'UTC') :: bigint as ls_line_updated_at,
ls_start.uri as ls_start_uri,
extract(epoch from ls_start.created_at  AT TIME ZONE 'UTC') :: bigint as ls_start_created_at,
extract(epoch from ls_start.updated_at  AT TIME ZONE 'UTC') :: bigint as ls_start_updated_at,
ls_end.uri as ls_end_uri,
extract(epoch from ls_end.created_at  AT TIME ZONE 'UTC') :: bigint as ls_end_created_at,
extract(epoch from ls_end.updated_at  AT TIME ZONE 'UTC') :: bigint as ls_end_updated_at,
ls_route.id AS ls_route_id,
ls_route.uri AS ls_route_uri,
extract(epoch from ls_route.created_at  AT TIME ZONE 'UTC') :: bigint as ls_route_created_at,
extract(epoch from ls_route.updated_at  AT TIME ZONE 'UTC') :: bigint as ls_route_updated_at,

rs_line.id as rs_line_id,
rs_line.uri as rs_line_uri,
extract(epoch from rs_line.created_at  AT TIME ZONE 'UTC') :: bigint as rs_line_created_at,
extract(epoch from rs_line.updated_at  AT TIME ZONE 'UTC') :: bigint as rs_line_updated_at,
rs_start.uri as rs_start_uri,
extract(epoch from rs_start.created_at  AT TIME ZONE 'UTC') :: bigint as rs_start_created_at,
extract(epoch from rs_start.updated_at  AT TIME ZONE 'UTC') :: bigint as rs_start_updated_at,
rs_end.uri as rs_end_uri,
extract(epoch from rs_end.created_at  AT TIME ZONE 'UTC') :: bigint as rs_end_created_at,
extract(epoch from rs_end.updated_at  AT TIME ZONE 'UTC') :: bigint as rs_end_updated_at,
rs_route.id AS rs_route_id,
rs_route.uri AS rs_route_uri,
extract(epoch from rs_route.created_at  AT TIME ZONE 'UTC') :: bigint as rs_route_created_at,
extract(epoch from rs_route.updated_at  AT TIME ZONE 'UTC') :: bigint as rs_route_updated_at,
rail_section.blocked_stop_areas as rs_blocked_sa,

m.id as message_id, m.text as message_text,
extract(epoch from m.created_at  AT TIME ZONE 'UTC') :: bigint as message_created_at,
extract(epoch from m.updated_at  AT TIME ZONE 'UTC') :: bigint as message_updated_at,

ch.id as channel_id, ch.name as channel_name,
ch.content_type as channel_content_type, ch.max_size as channel_max_size,
extract(epoch from ch.created_at  AT TIME ZONE 'UTC') :: bigint as channel_created_at,
extract(epoch from ch.updated_at  AT TIME ZONE 'UTC') :: bigint as channel_updated_at,
cht.id as channel_type_id, cht.name as channel_type,

adp.value as property_value, pr.key as property_key, pr.type as property_type,

pt.start_date as pattern_start_date,
pt.end_date as pattern_end_date,
pt.weekly_pattern as pattern_weekly_pattern,
pt.id as pattern_id,
extract(epoch from ts.begin ) ::int as time_slot_begin,
extract(epoch from ts.end ) as time_slot_end,
ts.id as time_slot_id

FROM disruption AS d
JOIN contributor AS co ON d.contributor_id = co.id
JOIN cause AS c ON (c.id = d.cause_id)
LEFT JOIN category AS cat ON cat.id=c.category_id
LEFT JOIN associate_disruption_tag ON associate_disruption_tag.disruption_id = d.id
LEFT JOIN tag AS t ON associate_disruption_tag.tag_id = t.id
JOIN impact AS i ON i.disruption_id = d.id
JOIN application_periods AS a ON a.impact_id = i.id
JOIN severity AS s ON s.id = i.severity_id
JOIN associate_impact_pt_object ON associate_impact_pt_object.impact_id = i.id
JOIN pt_object AS p ON associate_impact_pt_object.pt_object_id = p.id
LEFT JOIN line_section ON p.id = line_section.object_id
LEFT JOIN pt_object AS ls_line ON line_section.line_object_id = ls_line.id
LEFT JOIN pt_object AS ls_start ON line_section.start_object_id = ls_start.id
LEFT JOIN pt_object AS ls_end ON line_section.end_object_id = ls_end.id
LEFT JOIN associate_line_section_route_object
    ON associate_line_section_route_object.line_section_id = line_section.id
LEFT JOIN pt_object AS ls_route
    ON associate_line_section_route_object.route_object_id = ls_route.id
LEFT JOIN rail_section ON p.id = rail_section.object_id
LEFT JOIN pt_object AS rs_line ON rail_section.line_object_id = rs_line.id
LEFT JOIN pt_object AS rs_start ON rail_section.start_object_id = rs_start.id
LEFT JOIN pt_object AS rs_end ON rail_section.end_object_id = rs_end.id
LEFT JOIN associate_rail_section_route_object
    ON associate_rail_section_route_object.rail_section_id = rail_section.id
LEFT JOIN pt_object AS rs_route
    ON associate_rail_section_route_object.route_object_id = rs_route.id
LEFT JOIN message AS m ON m.impact_id = i.id
LEFT JOIN channel AS ch ON m.channel_id = ch.id
LEFT JOIN channel_type as cht on ch.id = cht.channel_id
LEFT JOIN associate_disruption_property adp ON adp.disruption_id = d.id
LEFT JOIN property pr ON pr.id = adp.property_id
LEFT JOIN pattern AS pt ON pt.impact_id = i.id
LEFT JOIN time_slot AS ts ON ts.pattern_id = pt.id
"#;
