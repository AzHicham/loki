use crate::chaos::sql_types::{DisruptionStatus, ImpactStatus, PtObjectType};
use crate::{
    chaos::schema::{
        application_periods, associate_disruption_property, associate_disruption_tag,
        associate_impact_pt_object as aipt, category, cause, channel, channel_type, contributor,
        disruption, impact, line_section, message, pattern, property, pt_object, rail_section,
        severity, tag, time_slot,
    },
    server_config::ChaosParams,
};
use anyhow::Error;
use diesel::prelude::*;
use diesel::sql_types::{Bit, Bool, Int4, Nullable, Text, Timestamp, Uuid};
use launch::loki::NaiveDateTime;
use uuid::Uuid as Uid;

fn establish_connection(database_url: &str) -> ConnectionResult<PgConnection> {
    PgConnection::establish(&database_url)
}

pub fn chaos_disruption_from_database(
    config: &ChaosParams,
    /* publication_period: (NaiveDateTime, NaiveDateTime),
    contributors: &[&str],*/
) -> Result<(), Error> {
    let connection = establish_connection(&config.chaos_database)?;

    let res = disruption::table
        .inner_join(contributor::table)
        // .inner_join(cause::table)
        // .left_join(category::table.on(category::id.nullable().eq(cause::category_id)))
        // .left_join(associate_disruption_tag::table)
        // .left_join(tag::table.on(tag::id.eq(associate_disruption_tag::tag_id)))
        // .inner_join(impact::table)
        // .inner_join(application_periods::table.on(application_periods::impact_id.eq(impact::id)))
        // .inner_join(severity::table.on(severity::id.nullable().eq(impact::severity_id)))
        // .inner_join(aipt::table.on(aipt::impact_id.eq(impact::id)))
        // .inner_join(pt_object::table.on(aipt::impact_id.eq(pt_object::id)))
        // .left_join(line_section::table.on(line_section::object_id.eq(pt_object::id.nullable())))
        // .left_join(rail_section::table.on(rail_section::object_id.eq(pt_object::id.nullable())))
        // .left_join(message::table.on(message::impact_id.eq(impact::id.nullable())))
        // .left_join(channel::table.on(channel::id.nullable().eq(message::channel_id)))
        // .left_join(channel_type::table.on(channel_type::channel_id.eq(channel::id.nullable())))
        // .left_join(associate_disruption_property::table)
        // .left_join(property::table.on(property::id.eq(associate_disruption_property::property_id)))
        // .left_join(pattern::table.on(pattern::impact_id.eq(impact::id.nullable())))
        // .left_join(time_slot::table.on(time_slot::pattern_id.eq(pattern::id.nullable())))
        .select((
            disruption::id,
            disruption::reference,
            disruption::status,
            disruption::start_publication_date,
            disruption::end_publication_date,
            disruption::created_at,
            disruption::updated_at,
            contributor::contributor_code,
        ))
        .limit(5)
        .load::<Disruption>(&connection);

    println!("Displaying {:?} posts", res);
    for r in res {
        println!("{:?}", r);
    }

    // let resuls: Result<Vec<ChaosDisruption>, diesel::result::Error> =
    //     diesel::sql_query(include_str!("query.sql")).load(&connection);
    // if let Err(ref err) = resuls {
    //     println!("{}", err.to_string());
    // }
    // println!("{:?}", resuls);

    Ok(())
}

// Remove ChaosDisruption when PR https://github.com/diesel-rs/diesel/pull/2254
// is merged
#[derive(Queryable, QueryableByName, Debug)]
pub struct ChaosDisruption {
    // Disruptions field
    #[sql_type = "Uuid"]
    pub disruption_id: Uid,
    #[sql_type = "Nullable<Text>"]
    pub disruption_reference: Option<String>,
    #[sql_type = "crate::chaos::sql_types::disruption_status"]
    pub disruption_status: DisruptionStatus,
    #[sql_type = "Nullable<Timestamp>"]
    pub disruption_start_publication_date: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub disruption_end_publication_date: Option<NaiveDateTime>,
    #[sql_type = "Timestamp"]
    pub disruption_created_at: NaiveDateTime,
    #[sql_type = "Nullable<Timestamp>"]
    pub disruption_updated_at: Option<NaiveDateTime>,
    #[sql_type = "Text"]
    pub contributor: String,
    // Cause fields
    #[sql_type = "Uuid"]
    pub cause_id: Uid,
    #[sql_type = "Text"]
    pub cause_wording: String,
    #[sql_type = "diesel::sql_types::Bool"]
    pub cause_visible: bool,
    #[sql_type = "Timestamp"]
    pub cause_created_at: NaiveDateTime,
    #[sql_type = "Nullable<Timestamp>"]
    pub cause_updated_at: Option<NaiveDateTime>,
    // Category fields
    #[sql_type = "Nullable<Text>"]
    pub category_name: Option<String>,
    #[sql_type = "Nullable<Uuid>"]
    pub category_id: Option<Uid>,
    #[sql_type = "Nullable<Timestamp>"]
    pub category_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub category_updated_at: Option<NaiveDateTime>,
    // Tag fields
    #[sql_type = "Nullable<Uuid>"]
    pub tag_id: Option<Uid>,
    #[sql_type = "Nullable<Text>"]
    pub tag_name: Option<String>,
    #[sql_type = "Nullable<Bool>"]
    pub tag_is_visible: Option<bool>,
    #[sql_type = "Nullable<Timestamp>"]
    pub tag_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub tag_updated_at: Option<NaiveDateTime>,
    // Impact fields
    #[sql_type = "Uuid"]
    pub impact_id: Uid,
    #[sql_type = "crate::chaos::sql_types::impact_status"]
    pub impact_status: ImpactStatus,
    #[sql_type = "Nullable<Uuid>"]
    pub impact_disruption_id: Option<Uid>,
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
    #[sql_type = "Bool"]
    pub severity_is_visible: bool,
    #[sql_type = "Int4"]
    pub severity_priority: i32,
    #[sql_type = "Timestamp"]
    pub severity_created_at: NaiveDateTime,
    #[sql_type = "Nullable<Timestamp>"]
    pub severity_updated_at: Option<NaiveDateTime>,
    // Ptobject fields
    #[sql_type = "Uuid"]
    pub ptobject_id: Uid,
    #[sql_type = "crate::chaos::sql_types::pt_object_type"]
    pub ptobject_type: PtObjectType,
    #[sql_type = "Nullable<Text>"]
    pub ptobject_uri: Option<String>,
    #[sql_type = "Timestamp"]
    pub ptobject_created_at: NaiveDateTime,
    #[sql_type = "Nullable<Timestamp>"]
    pub ptobject_updated_at: Option<NaiveDateTime>,
    // Ptobject line_section fields
    // Ptobject rail_section fields
    // Message fields
    #[sql_type = "Nullable<Uuid>"]
    pub message_id: Option<Uid>,
    #[sql_type = "Nullable<Text>"]
    pub message_text: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub message_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub message_updated_at: Option<NaiveDateTime>,
    // Channel fields
    #[sql_type = "Nullable<Uuid>"]
    pub channel_id: Option<Uid>,
    #[sql_type = "Nullable<Text>"]
    pub channel_name: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub channel_content_type: Option<String>,
    #[sql_type = "Nullable<Int4>"]
    pub channel_max_size: Option<i32>,
    #[sql_type = "Nullable<Timestamp>"]
    pub channel_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub channel_updated_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Uuid>"]
    pub channel_type_id: Option<Uid>,
    #[sql_type = "Nullable<Text>"]
    pub channel_type: Option<String>,
    //  Property & Associate property fields
    #[sql_type = "Nullable<Text>"]
    pub property_value: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub property_key: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub property_type: Option<String>,
    // Pattern & TimeSlot fields
    #[sql_type = "Nullable<Timestamp>"]
    pub pattern_start_date: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub pattern_end_date: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Bit>"]
    pub pattern_weekly_pattern: Option<Vec<u8>>,
    #[sql_type = "Nullable<Uuid>"]
    pub pattern_id: Option<Uid>,
    #[sql_type = "Nullable<Timestamp>"]
    pub time_slot_begin: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub time_slot_end: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Uuid>"]
    pub time_slot_id: Option<Uid>,
}

#[derive(Queryable, Debug)]
pub struct Disruption {
    pub id: Uid,
    pub reference: Option<String>,
    pub status: DisruptionStatus,
    pub start_publication_date: Option<NaiveDateTime>,
    pub end_publication_date: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
    pub contributor: String,
}

#[derive(Queryable, Debug)]
pub struct Cause {
    pub id: Uid,
    pub wording: String,
    pub visible: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Queryable, Debug)]
pub struct Category {
    pub name: Option<String>,
    pub id: Option<Uid>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Queryable, Debug)]
pub struct Tag {
    pub id: Option<Uuid>,
    pub name: Option<String>,
    pub is_visible: Option<bool>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Queryable, Debug)]
pub struct Impact {
    pub id: Uid,
    pub status: ImpactStatus,
    pub disruption_id: Option<Uid>,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Queryable, Debug)]
pub struct ApplicationPeriod {
    pub id: Uid,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
}

#[derive(Queryable, Debug)]
pub struct Severity {
    pub id: Uid,
    pub wording: String,
    pub color: Option<String>,
    pub is_visible: bool,
    pub priority: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Queryable, Debug)]
pub struct PtObject {
    pub id: Uid,
    pub type_: PtObjectType,
    pub uri: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Queryable, Debug)]
pub struct Message {
    pub id: Option<Uid>,
    pub text: Option<String>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Queryable, Debug)]
pub struct Channel {
    pub id: Option<Uid>,
    pub name: Option<String>,
    pub content_type: Option<String>,
    pub max_size: Option<i32>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub type_id: Option<Uid>,
    pub type_: Option<String>,
}

#[derive(Queryable, Debug)]
pub struct Property {
    pub value: Option<String>,
    pub key: Option<String>,
    pub type_: Option<String>,
}

#[derive(Queryable, Debug)]
pub struct Pattern {
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub weekly_pattern: Option<Vec<u8>>,
    pub id: Option<Uid>,
}

#[derive(Queryable, Debug)]
pub struct TimeSlot {
    pub begin: Option<NaiveDateTime>,
    pub end: Option<NaiveDateTime>,
    pub id: Option<Uid>,
}
