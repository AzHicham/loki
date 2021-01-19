extern crate static_assertions;

mod engine;
mod request;
// pub mod calendar_data;
// pub mod daily_data;

pub mod crowding_data;


pub use chrono::NaiveDateTime;
pub use log;
pub use time::PositiveDuration;
pub use transit_model;

pub mod time;

pub mod traits;

mod timetables;

mod transit_data;

pub type PeriodicData = transit_data::TransitData<timetables::PeriodicTimetables>;
pub type DailyData = transit_data::TransitData<timetables::DailyTimetables>;

pub type DailyRequest<'data, 'model> = request::depart_after::Request<'data,  DailyData>;

pub type PeriodicRequest<'data, 'model> =
    request::depart_after::Request<'data,  PeriodicData>;

// pub use laxatips_data::{
//     LaxatipsData,
//     time::{PositiveDuration},
//     transit_data::{TransitData, Idx, StopPoint, VehicleJourney, TransitModelTransfer}
// };

// pub use laxatips_daily_data::{
//     LaxatipsDailyData,
//     time::{PositiveDuration},
//     transit_data::{TransitData, Idx, StopPoint, VehicleJourney, TransitModelTransfer}
// };

pub use engine::multicriteria_raptor::MultiCriteriaRaptor;

pub use request::depart_after::Request as DepartAfterRequest;

pub mod response;
