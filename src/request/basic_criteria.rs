use std::marker::PhantomData;

use crate::{
    time::SecondsSinceDatasetUTCStart,
    traits::{self},
    PositiveDuration,
};

pub mod depart_after;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Criteria {
    arrival_time: SecondsSinceDatasetUTCStart,
    nb_of_legs: u8,
    fallback_duration: PositiveDuration,
    transfers_duration: PositiveDuration,
}

pub struct Types<'data, Data> {
    _phantom: PhantomData<&'data Data>,
}

impl<'data, Data: traits::Data> traits::TransitTypes for Types<'data, Data> {
    type Stop = Data::Stop;

    type Mission = Data::Mission;

    type Position = Data::Position;

    type Trip = Data::Trip;

    type Transfer = Data::Transfer;
}

impl<'data, Data: traits::Data> traits::RequestTypes for Types<'data, Data> {
    type Departure = super::generic_request::Departure;

    type Arrival = super::generic_request::Arrival;

    type Criteria = Criteria;
}
