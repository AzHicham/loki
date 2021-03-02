use crate::traits;
use crate::{
    loads_data::LoadsCount,
    time::{PositiveDuration, SecondsSinceDatasetUTCStart},
};

use log::warn;
use traits::{BadRequest, RequestTypes};

pub struct GenericRequest<'data, Data: traits::Data> {
    pub(super) transit_data: &'data Data,
    pub(super) departure_datetime: SecondsSinceDatasetUTCStart,
    pub(super) departures_stop_point_and_fallback_duration: Vec<(Data::Stop, PositiveDuration)>,
    pub(super) arrivals_stop_point_and_fallbrack_duration: Vec<(Data::Stop, PositiveDuration)>,
    pub(super) leg_arrival_penalty: PositiveDuration,
    pub(super) leg_walking_penalty: PositiveDuration,
    pub(super) max_arrival_time: SecondsSinceDatasetUTCStart,
    pub(super) max_nb_legs: u8,
}

impl<'data, Data> GenericRequest<'data, Data>
where
    Data: traits::Data,
{
    pub fn new<Departures, Arrivals, D, A>
    (
        model: &transit_model::Model,
        transit_data: & 'data Data,
        request_input : traits::RequestInput<Departures, Arrivals, D, A>
    ) -> Result<Self, BadRequest>
    where
        Arrivals : Iterator<Item = (A, PositiveDuration)>,
        Departures : Iterator<Item = (D, PositiveDuration)>,
        A : AsRef<str>,
        D : AsRef<str>,
        Self: Sized
    {
        let departure_datetime = transit_data
            .calendar()
            .from_naive_datetime(&request_input.departure_datetime)
            .ok_or_else(|| {
                warn!(
                    "The departure datetime {:?} is out of bound of the allowed dates. \
                    Allowed dates are between {:?} and {:?}.",
                    request_input.departure_datetime,
                    transit_data.calendar().first_datetime(),
                    transit_data.calendar().last_datetime(),
                );
                BadRequest::DepartureDatetime
            })?;

        let departures : Vec<_> = request_input.departures_stop_point_and_fallback_duration
            .enumerate()
            .filter_map(|(idx, (stop_point_uri, fallback_duration))| {
                let stop_point_uri = stop_point_uri.as_ref();
                let stop_idx = model.stop_points.get_idx(stop_point_uri).or_else(|| {
                    warn!(
                        "The {}th departure stop point {} is not found in model. \
                                I ignore it.",
                        idx, stop_point_uri
                    );
                    None
                })?;
                let stop = transit_data.stop_point_idx_to_stop(&stop_idx).or_else(|| {
                    warn!(
                        "The {}th departure stop point {} with idx {:?} is not found in transit_data. \
                            I ignore it",
                        idx, stop_point_uri, stop_idx
                    );
                    None
                })?;
                Some((stop, fallback_duration))
            })
            .collect();
        if departures.is_empty() {
            return Err(BadRequest::NoValidDepartureStop);
        }

        let arrivals : Vec<_> = request_input.arrivals_stop_point_and_fallback_duration
            .enumerate()
            .filter_map(|(idx, (stop_point_uri, fallback_duration))| {
                let stop_point_uri = stop_point_uri.as_ref();
                let stop_idx = model.stop_points.get_idx(stop_point_uri).or_else(|| {
                    warn!(
                        "The {}th arrival stop point {} is not found in model. \
                                I ignore it.",
                        idx, stop_point_uri
                    );
                    None
                })?;
                let stop = transit_data.stop_point_idx_to_stop(&stop_idx).or_else(|| {
                    warn!(
                        "The {}th arrival stop point {} with idx {:?} is not found in transit_data. \
                            I ignore it",
                        idx, stop_point_uri, stop_idx
                    );
                    None
                })?;
                Some((stop, fallback_duration))
            })
            .collect();

        if arrivals.is_empty() {
            return Err(BadRequest::NoValidArrivalStop);
        }

        let result = Self {
            transit_data,
            departure_datetime,
            departures_stop_point_and_fallback_duration: departures,
            arrivals_stop_point_and_fallbrack_duration: arrivals,
            leg_arrival_penalty : request_input.params.leg_arrival_penalty,
            leg_walking_penalty : request_input.params.leg_walking_penalty,
            max_arrival_time: departure_datetime + request_input.params.max_journey_duration,
            max_nb_legs : request_input.params.max_nb_of_legs,
        };

        Ok(result)
    }
}

use crate::response;
use crate::traits::Journey as PTJourney;
impl<'data, Data> GenericRequest<'data, Data>
where
    Data: traits::Data,
{
    pub fn create_response<R>(
        &self,
        pt_journey: &PTJourney<R>,
        loads_count: LoadsCount,
    ) -> Result<response::Journey<Data>, response::BadJourney<Data>>
    where
        R: RequestTypes<
            Departure = Departure,
            Arrival = Arrival,
            Trip = Data::Trip,
            Position = Data::Position,
            Transfer = Data::Transfer,
        >,
    {
        let departure_datetime = self.departure_datetime;
        let departure_idx = pt_journey.departure_leg.departure.idx;
        let departure_fallback_duration =
            &self.departures_stop_point_and_fallback_duration[departure_idx].1;

        let first_vehicle = response::VehicleLeg {
            trip: pt_journey.departure_leg.trip.clone(),
            board_position: pt_journey.departure_leg.board_position.clone(),
            debark_position: pt_journey.departure_leg.debark_position.clone(),
        };

        let arrival_fallback_duration =
            &self.arrivals_stop_point_and_fallbrack_duration[pt_journey.arrival.idx].1;

        let connections = pt_journey.connection_legs.iter().map(|connection_leg| {
            let transfer = connection_leg.transfer.clone();
            let vehicle_leg = response::VehicleLeg {
                trip: connection_leg.trip.clone(),
                board_position: connection_leg.board_position.clone(),
                debark_position: connection_leg.debark_position.clone(),
            };
            (transfer, vehicle_leg)
        });

        response::Journey::new(
            departure_datetime,
            *departure_fallback_duration,
            first_vehicle,
            connections,
            *arrival_fallback_duration,
            loads_count,
            &self.transit_data,
        )
    }
}

use crate::traits::Data as DataTrait;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Departure {
    pub(super) idx: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Arrival {
    pub(super) idx: usize,
}

impl<'data, Data> GenericRequest<'data, Data>
where
    Data: DataTrait,
{
    pub(super) fn departures(&self) -> Departures {
        let nb_of_departures = self.departures_stop_point_and_fallback_duration.len();
        Departures {
            inner: 0..nb_of_departures,
        }
    }

    pub(super) fn arrivals(&self) -> Arrivals {
        let nb_of_arrivals = self.arrivals_stop_point_and_fallbrack_duration.len();
        Arrivals {
            inner: 0..nb_of_arrivals,
        }
    }
}

pub struct Departures {
    pub(super) inner: std::ops::Range<usize>,
}

impl Iterator for Departures {
    type Item = Departure;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|idx| Departure { idx })
    }
}

pub struct Arrivals {
    pub(super) inner: std::ops::Range<usize>,
}

impl Iterator for Arrivals {
    type Item = Arrival;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|idx| Arrival { idx })
    }
}
