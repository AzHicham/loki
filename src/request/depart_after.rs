use crate::laxatips_data::{
    LaxatipsData,
    transit_data::{ Stop,  },
    time::{PositiveDuration, SecondsSinceDatasetStart},
};





pub mod init;
pub mod public_transit;
pub mod response;

pub struct Request<'data> {
    laxatips_data: &'data LaxatipsData,
    departure_datetime: SecondsSinceDatasetStart,
    departures_stop_point_and_fallback_duration: Vec<(Stop, PositiveDuration)>,
    arrivals_stop_point_and_fallbrack_duration: Vec<(Stop, PositiveDuration)>,
    leg_arrival_penalty: PositiveDuration,
    leg_walking_penalty: PositiveDuration,
    max_arrival_time: SecondsSinceDatasetStart,
    max_nb_legs: u8,
}

