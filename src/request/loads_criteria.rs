use crate::{PositiveDuration, loads_data::LoadsCount, time::SecondsSinceDatasetUTCStart};



pub mod depart_after;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Criteria {
    arrival_time: SecondsSinceDatasetUTCStart,
    nb_of_legs: u8,
    fallback_duration: PositiveDuration,
    transfers_duration: PositiveDuration,
    loads_count: LoadsCount,
}




