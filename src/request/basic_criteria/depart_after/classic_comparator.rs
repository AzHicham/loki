use crate::traits;
use crate::{
    loads_data::LoadsCount,
    time::{PositiveDuration},
};

use chrono::NaiveDateTime;
use traits::{BadRequest, RequestIO};

use super::{Criteria, Arrivals, Departures, GenericBasicDepartAfter, Arrival, Departure};


pub struct Request<'data, Data: traits::Data> {
    generic: GenericBasicDepartAfter<'data, Data>,
}



impl<'data, Data: traits::Data> traits::TransitTypes for Request<'data, Data> {
    type Stop = Data::Stop;
    type Mission = Data::Mission;
    type Trip = Data::Trip;
    type Transfer = Data::Transfer;
    type Position = Data::Position;
}

impl<'data, Data: traits::Data> traits::RequestTypes for Request<'data, Data> {
    type Departure = Departure;
    type Arrival = Arrival;
    type Criteria = Criteria;
}

impl<'data, 'model, Data: traits::Data> traits::Request for Request<'data, Data> {
    fn is_lower(&self, lower: &Self::Criteria, upper: &Self::Criteria) -> bool {
        let arrival_penalty = self.generic.leg_arrival_penalty();
        let walking_penalty = self.generic.leg_walking_penalty();

        lower.arrival_time + arrival_penalty * (lower.nb_of_legs as u32)
            <= upper.arrival_time + arrival_penalty * (upper.nb_of_legs as u32)
        // && lower.nb_of_transfers <= upper.nb_of_transfers
        &&
        lower.fallback_duration + lower.transfers_duration  + walking_penalty * (lower.nb_of_legs as u32)
            <=  upper.fallback_duration + upper.transfers_duration + walking_penalty * (upper.nb_of_legs as u32)

 
    }



    fn is_valid(&self, criteria: &Self::Criteria) -> bool {
        self.generic.is_valid(criteria)
    }

    fn board_and_ride(
        &self,
        position: &Self::Position,
        trip: &Self::Trip,
        waiting_criteria: &Self::Criteria,
    ) -> Option<Self::Criteria> {
        self.generic.board_and_ride(position, trip, waiting_criteria)
    }

    fn best_trip_to_board(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
        waiting_criteria: &Self::Criteria,
    ) -> Option<(Self::Trip, Self::Criteria)> {
        self.generic.best_trip_to_board(position, mission, waiting_criteria)
    }

    fn debark(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
        onboard_criteria: &Self::Criteria,
    ) -> Option<Self::Criteria> {
        self.generic.debark(trip, position, onboard_criteria)
    }

    fn ride(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
        criteria: &Self::Criteria,
    ) -> Self::Criteria {
        self.generic.ride(trip, position, criteria)
    }

    fn transfer(
        &self,
        from_stop: &Self::Stop,
        transfer: &Self::Transfer,
        criteria: &Self::Criteria,
    ) -> (Self::Stop, Self::Criteria) {
        self.generic.transfer(from_stop, transfer, criteria)
    }

    fn depart(&self, departure: &Self::Departure) -> (Self::Stop, Self::Criteria) {
        self.generic.depart(departure)
    }

    fn arrival_stop(&self, arrival: &Self::Arrival) -> Self::Stop {
        self.generic.arrival_stop(arrival)
    }

    fn arrive(&self, arrival: &Self::Arrival, criteria: &Self::Criteria) -> Self::Criteria {
        self.generic.arrive(arrival, criteria)
    }

    fn is_upstream(
        &self,
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool {
        self.generic.is_upstream(upstream, downstream, mission)
    }

    fn next_on_mission(
        &self,
        stop: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.generic.next_on_mission(stop, mission)
    }

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission {
        self.generic.mission_of(trip)
    }

    fn stop_of(&self, position: &Self::Position, mission: &Self::Mission) -> Self::Stop {
        self.generic.stop_of(position, mission)
    }

    fn nb_of_stops(&self) -> usize {
        self.generic.nb_of_stops()
    }

    fn stop_id(&self, stop: &Self::Stop) -> usize {
        self.generic.stop_id(stop)
    }

    fn nb_of_missions(&self) -> usize {
        self.generic.nb_of_missions()
    }

    fn mission_id(&self, mission: &Self::Mission) -> usize {
        self.generic.mission_id(mission)
    }
}

impl<'data, 'outer, Data> traits::RequestIters<'outer> for Request<'data, Data>
where
    Data: traits::Data + traits::DataIters<'outer>,
{
    type Departures = Departures;
    fn departures(&'outer self) -> Self::Departures {
        self.generic.departures()
    }

    type Arrivals = Arrivals;
    fn arrivals(&'outer self) -> Self::Arrivals {
        self.generic.arrivals()
    }
}

impl<'data, 'outer, Data> traits::DataIters<'outer> for Request<'data, Data>
where
    Data: traits::Data + traits::DataIters<'outer>,
{
    type MissionsAtStop = Data::MissionsAtStop;

    fn boardable_missions_at(&'outer self, stop: &Self::Stop) -> Self::MissionsAtStop {
        self.generic.boardable_missions_at(stop)
    }
    type TransfersAtStop = Data::TransfersAtStop;
    fn transfers_at(&'outer self, from_stop: &Self::Stop) -> Self::TransfersAtStop {
        self.generic.transfers_at(from_stop)
    }

    type TripsOfMission = Data::TripsOfMission;
    fn trips_of(&'outer self, mission: &Self::Mission) -> Self::TripsOfMission {
        self.generic.trips_of(mission)
    }
}

impl<'data, Data> traits::RequestWithIters for Request<'data, Data> where
    Data: traits::DataWithIters
{
}

use crate::response;
use crate::traits::Journey as PTJourney;



impl<'data, Data> RequestIO<'data, Data> for Request<'data, Data>
where
    Data: traits::Data,
{
    fn new<S: AsRef<str>, T: AsRef<str>>(
        model: &transit_model::Model,
        transit_data: &'data Data,
        departure_datetime: NaiveDateTime,
        departures_stop_point_and_fallback_duration: impl Iterator<Item = (S, PositiveDuration)>,
        arrivals_stop_point_and_fallback_duration: impl Iterator<Item = (T, PositiveDuration)>,
        leg_arrival_penalty: PositiveDuration,
        leg_walking_penalty: PositiveDuration,
        max_duration_to_arrival: PositiveDuration,
        max_nb_legs: u8,
    ) -> Result<Self, BadRequest> {
        let generic_result = GenericBasicDepartAfter::new(
            model,
            transit_data,
            departure_datetime,
            departures_stop_point_and_fallback_duration,
            arrivals_stop_point_and_fallback_duration,
            leg_arrival_penalty,
            leg_walking_penalty,
            max_duration_to_arrival,
            max_nb_legs,
        );
        generic_result.map(|generic| Self { generic })
    }
    fn create_response(
        &self,
        data: &Data,
        pt_journey: &PTJourney<Self>,
    ) -> Result<response::Journey<Data>, response::BadJourney<Data>> {
        self.generic
            .create_response(data, pt_journey, LoadsCount::default())
    }
}
