use crate::time::{PositiveDuration, SecondsSinceDatasetUTCStart};
use crate::transit_data::{TransitData, Stop};
use crate::timetables::Timetables as TimetablesTrait;


pub mod init;
pub mod public_transit;
pub mod response;
// pub mod traits_impl;

pub struct Request<'data, 'model, Timetables : TimetablesTrait> {
    model : & 'model transit_model::Model,
    transit_data: &'data TransitData<Timetables>,
    departure_datetime: SecondsSinceDatasetUTCStart,
    departures_stop_point_and_fallback_duration: Vec<(Stop, PositiveDuration)>,
    arrivals_stop_point_and_fallbrack_duration: Vec<(Stop, PositiveDuration)>,
    leg_arrival_penalty: PositiveDuration,
    leg_walking_penalty: PositiveDuration,
    max_arrival_time: SecondsSinceDatasetUTCStart,
    max_nb_legs: u8,
}

