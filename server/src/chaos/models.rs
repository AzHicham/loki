use crate::chaos::schema::*;
use diesel::prelude::*;
use diesel::sql_types::Timestamp;
use diesel::Queryable;
use launch::loki::chrono::{DateTime, Utc};
use launch::loki::NaiveDateTime;
use std::env;
use uuid::Uuid;

#[derive(Queryable, Debug)]
pub struct ApplicationPeriod {
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
    pub id: Uuid,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub impact_id: Uuid,
}

pub fn establish_connection() -> PgConnection {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}

fn rtmain() {
    use crate::chaos::schema::application_periods::created_at;
    use crate::chaos::schema::application_periods::dsl::application_periods;
    use crate::chaos::schema::application_periods::end_date;
    let connection = establish_connection();
    let results = application_periods
        .select((created_at, end_date))
        .limit(5)
        .load::<(NaiveDateTime, Option<NaiveDateTime>)>(&connection)
        .expect("Error loading posts");

    println!("Displaying {} posts", results.len());
    for post in results {
        println!("{:?}", post);
    }
}
