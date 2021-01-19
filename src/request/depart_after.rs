use crate::time::{PositiveDuration, SecondsSinceDatasetUTCStart};
use crate::traits::TransitTypes;

pub mod init;
// pub mod public_transit;
pub mod response;
pub mod traits_impl;

pub struct Request<'data, 'model, Data : TransitTypes> {
    model : & 'model transit_model::Model,
    transit_data: &'data Data,
    departure_datetime: SecondsSinceDatasetUTCStart,
    departures_stop_point_and_fallback_duration: Vec<(Data::Stop, PositiveDuration)>,
    arrivals_stop_point_and_fallbrack_duration: Vec<(Data::Stop, PositiveDuration)>,
    leg_arrival_penalty: PositiveDuration,
    leg_walking_penalty: PositiveDuration,
    max_arrival_time: SecondsSinceDatasetUTCStart,
    max_nb_legs: u8,
}

