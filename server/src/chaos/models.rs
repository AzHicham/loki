use crate::chaos::schema::*;
use diesel::prelude::*;
use diesel::Queryable;

#[derive(Queryable)]
pub struct ApplicationPeriod {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub published: bool,
}
