extern crate static_assertions;

mod engine;
mod request;

pub mod loads_data;

pub use chrono::NaiveDateTime;
pub use log;
pub use time::PositiveDuration;
pub use transit_model;

pub mod time;

pub mod traits;

mod timetables;

mod transit_data;

pub type DailyData = transit_data::TransitData<timetables::DailyTimetables>;
pub type PeriodicData = transit_data::TransitData<timetables::PeriodicTimetables>;

pub type LoadsDailyData = transit_data::TransitData<timetables::LoadsDailyTimetables>;
pub type LoadsPeriodicData = transit_data::TransitData<timetables::LoadsPeriodicTimetables>;

pub use loads_data::LoadsData;

pub use transit_data::{Idx, StopPoint, TransitData, TransitModelTransfer, VehicleJourney};

pub use engine::multicriteria_raptor::MultiCriteriaRaptor;

pub use request::{depart_after::DepartAfter, loads_depart_after::LoadsDepartAfter};

pub mod response;

