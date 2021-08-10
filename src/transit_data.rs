// Copyright  (C) 2020, Kisio Digital and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Kisio Digital (www.kisio.com).
// Help us simplify mobility and open public transport:
// a non ending quest to the responsive locomotion way of traveling!
//
// This contribution is a part of the research and development work of the
// IVA Project which aims to enhance traveler information and is carried out
// under the leadership of the Technological Research Institute SystemX,
// with the partnership and support of the transport organization authority
// Ile-De-France Mobilités (IDFM), SNCF, and public funds
// under the scope of the French Program "Investissements d’Avenir".
//
// LICENCE: This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// Stay tuned using
// twitter @navitia
// channel `#navitia` on riot https://riot.im/app/#/room/#navitia:matrix.org
// https://groups.google.com/d/forum/navitia
// www.navitia.io

pub mod data_interface;
pub mod init;
pub mod iters;

use iters::MissionsOfStop;
pub use transit_model::objects::Time as TransitModelTime;
pub use transit_model::objects::{StopPoint, Transfer as TransitModelTransfer, VehicleJourney};
pub use typed_index_collection::Idx;

use crate::{
    loads_data::{Load, LoadsData},
    time::{Calendar, PositiveDuration, SecondsSinceDatasetUTCStart},
};

use std::{collections::HashMap, fmt::Debug};

use crate::timetables::{Timetables as TimetablesTrait, TimetablesIter};

pub struct TransitData<Timetables: TimetablesTrait> {
    pub(super) stop_point_idx_to_stop: HashMap<Idx<StopPoint>, Stop>,

    pub(super) stops_data: Vec<StopData<Timetables>>,
    pub(super) timetables: Timetables,
}
pub struct StopData<Timetables: TimetablesTrait> {
    pub(super) stop_point_idx: Idx<StopPoint>,
    pub(super) position_in_timetables: Vec<(Timetables::Mission, Timetables::Position)>,
    pub(super) transfers_to: Vec<(Stop, PositiveDuration, Idx<TransitModelTransfer>)>,
    pub(super) transfers_from: Vec<(Stop, PositiveDuration, Idx<TransitModelTransfer>)>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Ord, PartialOrd)]
pub struct Stop {
    pub(super) idx: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TransferType {
    Forward,
    Backward,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Transfer {
    pub(super) transfer_type: TransferType,
    pub(super) stop: Stop,
    pub(super) idx_in_stop_transfers: usize,
}

impl<Timetables: TimetablesTrait> TransitData<Timetables> {
    pub fn stop_data<'a>(&'a self, stop: &Stop) -> &'a StopData<Timetables> {
        &self.stops_data[stop.idx]
    }

    pub fn stop_point_idx_to_stop(&self, stop_point_idx: &Idx<StopPoint>) -> Option<&Stop> {
        self.stop_point_idx_to_stop.get(stop_point_idx)
    }
}

impl<Timetables: TimetablesTrait> data_interface::TransitTypes for TransitData<Timetables> {
    type Stop = Stop;

    type Mission = Timetables::Mission;

    type Position = Timetables::Position;

    type Trip = Timetables::Trip;

    type Transfer = Transfer;
}

impl<Timetables: TimetablesTrait> data_interface::Data for TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
{
    fn stop_point_idx_to_stop(&self, stop_point_idx: &Idx<StopPoint>) -> Option<Self::Stop> {
        self.stop_point_idx_to_stop.get(stop_point_idx).copied()
    }

    fn new(
        model: &transit_model::Model,
        loads_data: &LoadsData,
        default_transfer_duration: PositiveDuration,
    ) -> Self {
        Self::_new(model, loads_data, default_transfer_duration)
    }

    fn calendar(&self) -> &Calendar {
        self.timetables.calendar()
    }

    fn is_upstream(
        &self,
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool {
        self.timetables
            .is_upstream_in_mission(upstream, downstream, mission)
    }

    fn next_on_mission(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.timetables.next_position(position, mission)
    }

    fn previous_on_mission(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.timetables.previous_position(position, mission)
    }

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission {
        self.timetables.mission_of(trip)
    }

    fn stop_of(&self, position: &Self::Position, mission: &Self::Mission) -> Self::Stop {
        self.timetables.stop_at(position, mission)
    }

    fn board_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)> {
        self.timetables.board_time_of(trip, position)
    }

    fn debark_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)> {
        self.timetables.debark_time_of(trip, position)
    }

    fn arrival_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load) {
        self.timetables.arrival_time_of(trip, position)
    }

    fn departure_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load) {
        self.timetables.departure_time_of(trip, position)
    }

    fn transfer(&self, transfer: &Self::Transfer) -> (Self::Stop, PositiveDuration) {
        let stop_data = self.stop_data(&transfer.stop);
        let result = match transfer.transfer_type {
            TransferType::Forward => stop_data.transfers_to[transfer.idx_in_stop_transfers],
            TransferType::Backward => stop_data.transfers_from[transfer.idx_in_stop_transfers],
        };
        (result.0, result.1)
    }

    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &crate::time::SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        self.timetables
            .earliest_trip_to_board_at(waiting_time, mission, position)
    }

    fn latest_trip_that_debark_at(
        &self,
        waiting_time: &crate::time::SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        self.timetables
            .latest_trip_that_debark_at(waiting_time, mission, position)
    }

    fn nb_of_trips(&self) -> usize {
        self.timetables.nb_of_trips()
    }

    fn nb_of_stops(&self) -> usize {
        self.stops_data.len()
    }

    fn stop_id(&self, stop: &Stop) -> usize {
        stop.idx
    }

    fn nb_of_missions(&self) -> usize {
        self.timetables.nb_of_missions()
    }

    fn mission_id(&self, mission: &Self::Mission) -> usize {
        self.timetables.mission_id(mission)
    }

    fn to_naive_datetime(
        &self,
        seconds: &crate::time::SecondsSinceDatasetUTCStart,
    ) -> chrono::NaiveDateTime {
        self.timetables.calendar().to_naive_datetime(seconds)
    }

    fn vehicle_journey_idx(&self, trip: &Self::Trip) -> Idx<VehicleJourney> {
        self.timetables.vehicle_journey_idx(trip)
    }

    fn stop_point_idx(&self, stop: &Stop) -> Idx<StopPoint> {
        self.stops_data[stop.idx].stop_point_idx
    }

    fn stoptime_idx(&self, position: &Self::Position, trip: &Self::Trip) -> usize {
        self.timetables.stoptime_idx(position, trip)
    }

    fn transfer_idx(&self, transfer: &Self::Transfer) -> Idx<TransitModelTransfer> {
        let stop_data = self.stop_data(&transfer.stop);
        let result = match transfer.transfer_type {
            TransferType::Forward => stop_data.transfers_to[transfer.idx_in_stop_transfers],
            TransferType::Backward => stop_data.transfers_from[transfer.idx_in_stop_transfers],
        };
        result.2
    }

    fn day_of(&self, trip: &Self::Trip) -> chrono::NaiveDate {
        self.timetables.day_of(trip)
    }

    fn transfer_start_end_stop(
        &self,
        transfer: &Self::Transfer,
    ) -> (Self::Stop, Self::Stop, PositiveDuration) {
        match transfer.transfer_type {
            TransferType::Forward => {
                let start_stop_data = self.stop_data(&transfer.stop);
                let end_stop_data = start_stop_data.transfers_to[transfer.idx_in_stop_transfers];
                (transfer.stop, end_stop_data.0, end_stop_data.1)
            }
            TransferType::Backward => {
                let start_stop_data = self.stop_data(&transfer.stop);
                let end_stop_data = start_stop_data.transfers_from[transfer.idx_in_stop_transfers];
                (end_stop_data.0, transfer.stop, end_stop_data.1)
            }
        }
    }

    fn is_same_stop(&self, stop_a: &Self::Stop, stop_b: &Self::Stop) -> bool {
        stop_a.idx == stop_b.idx
    }
}

impl<'a, Timetables> data_interface::DataIters<'a> for TransitData<Timetables>
where
    Timetables: TimetablesTrait + TimetablesIter<'a>,
    Timetables::Mission: 'a,
    Timetables::Position: 'a,
{
    type MissionsAtStop = MissionsOfStop<'a, Timetables>;

    fn boardable_missions_at(&'a self, stop: &Self::Stop) -> Self::MissionsAtStop {
        self.missions_of(stop)
    }

    type OutgoingTransfersAtStop = iters::ForwardTransfersOfStop;
    fn outgoing_transfers_at(&'a self, from_stop: &Self::Stop) -> Self::OutgoingTransfersAtStop {
        self.transfers_forward_of(from_stop)
    }

    type IncomingTransfersAtStop = iters::BackwardTransfersOfStop;
    fn incoming_transfers_at(&'a self, stop: &Self::Stop) -> Self::IncomingTransfersAtStop {
        self.transfers_backward_of(stop)
    }

    type TripsOfMission = <Timetables as TimetablesIter<'a>>::Trips;

    fn trips_of(&'a self, mission: &Self::Mission) -> Self::TripsOfMission {
        self.timetables.trips_of(mission)
    }
}

impl<Timetables> data_interface::DataWithIters for TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
    Timetables::Mission: 'static,
    Timetables::Position: 'static,
{
}
