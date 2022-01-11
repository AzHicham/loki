use crate::{
    chaos::sql_types::{DisruptionStatus, ImpactStatus, PtObjectType},
    server_config::ChaosParams,
};
use anyhow::Error;
use diesel::{
    pg::types::sql_types::Array,
    prelude::*,
    sql_types::{Bit, Bool, Date, Int4, Nullable, Text, Time, Timestamp, Uuid},
};
use launch::loki::{
    chrono::{NaiveDate, NaiveTime},
    NaiveDateTime,
};
use uuid::Uuid as Uid;

pub fn chaos_disruption_from_database(
    config: &ChaosParams,
    publication_period: (NaiveDateTime, NaiveDateTime),
) -> Result<(), Error> {
    let connection = PgConnection::establish(&config.chaos_database)?;

    let res = diesel::sql_query(include_str!("query.sql"))
        .bind::<Timestamp, _>(
            NaiveDateTime::parse_from_str("20300101T000000", "%Y%m%dT%H%M%S").unwrap(),
        )
        .bind::<Timestamp, _>(
            NaiveDateTime::parse_from_str("20100101T000000", "%Y%m%dT%H%M%S").unwrap(),
        )
        .bind::<Timestamp, _>(
            NaiveDateTime::parse_from_str("20300101T000000", "%Y%m%dT%H%M%S").unwrap(),
        )
        .bind::<Array<Text>, _>(&config.chaos_contributors)
        .bind::<Int4, _>(config.chaos_batch_size as i32)
        .bind::<Int4, _>(config.chaos_batch_size as i32)
        .load::<ChaosDisruption>(&connection);

    if let Err(ref err) = res {
        println!("{}", err.to_string());
    }
    println!("{:?}", res);

    Ok(())
}

// Remove ChaosDisruption when PR https://github.com/diesel-rs/diesel/pull/2254 is merged
// and use model_v2
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
    #[sql_type = "Nullable<Text>"]
    pub ls_line_uri: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub ls_line_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub ls_line_updated_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Text>"]
    pub ls_start_uri: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub ls_start_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub ls_start_updated_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Text>"]
    pub ls_end_uri: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub ls_end_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub ls_end_updated_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Uuid>"]
    pub ls_route_id: Option<Uid>,
    #[sql_type = "Nullable<Text>"]
    pub ls_route_uri: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub ls_route_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub ls_route_updated_at: Option<NaiveDateTime>,
    // Ptobject rail_section fields
    #[sql_type = "Nullable<Text>"]
    pub rs_line_uri: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub rs_line_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub rs_line_updated_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Text>"]
    pub rs_start_uri: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub rs_start_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub rs_start_updated_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Text>"]
    pub rs_end_uri: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub rs_end_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub rs_end_updated_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Uuid>"]
    pub rs_route_id: Option<Uid>,
    #[sql_type = "Nullable<Text>"]
    pub rs_route_uri: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub rs_route_created_at: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub rs_route_updated_at: Option<NaiveDateTime>,
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
    #[sql_type = "Nullable<Date>"]
    pub pattern_start_date: Option<NaiveDate>,
    #[sql_type = "Nullable<Date>"]
    pub pattern_end_date: Option<NaiveDate>,
    #[sql_type = "Nullable<Bit>"]
    pub pattern_weekly_pattern: Option<Vec<u8>>,
    #[sql_type = "Nullable<Uuid>"]
    pub pattern_id: Option<Uid>,
    #[sql_type = "Nullable<Time>"]
    pub time_slot_begin: Option<NaiveTime>,
    #[sql_type = "Nullable<Time>"]
    pub time_slot_end: Option<NaiveTime>,
    #[sql_type = "Nullable<Uuid>"]
    pub time_slot_id: Option<Uid>,
}
