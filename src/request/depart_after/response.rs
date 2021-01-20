use crate::traits;
use crate::traits::Journey as PTJourney;

use crate::response;

use super::Request;

impl<'data, 'model, Data> Request<'data, Data>
where
    Data: traits::Data,
{
    pub fn create_response(
        &self,
        data: &Data,
        pt_journey: &PTJourney<Self>,
    ) -> Result<response::Journey<Data>, response::BadJourney<Data>> {
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
            data,
        )
    }
}
