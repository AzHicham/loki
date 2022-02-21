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

use crate::{
    filters::Filters,
    loads_data::Load,
    models::{ModelRefs, StopPointIdx, StopTimeIdx, TransferIdx, VehicleJourneyIdx},
    time::{Calendar, PositiveDuration, SecondsSinceDatasetUTCStart},
    DataWithIters, RealTimeLevel,
};
pub use transit_model::objects::{
    StopPoint, Time as TransitModelTime, Transfer as TransitModelTransfer, VehicleJourney,
};
pub use typed_index_collection::Idx;

use crate::transit_data::{data_interface, data_interface::Data};

pub struct TransitDataFiltered<'data, 'filter, Data> {
    pub(super) transit_data: &'data Data,
    memory: &'filter FilterMemory,
}

#[derive(Debug)]
pub struct FilterMemory {
    allowed_base_stop_points: Vec<bool>,
    allowed_new_stop_points: Vec<bool>,
    allowed_base_vehicle_journeys: Vec<bool>,
    allowed_new_vehicle_journeys: Vec<bool>,
}

impl Default for FilterMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl FilterMemory {
    pub fn new() -> Self {
        Self {
            allowed_base_stop_points: Vec::new(),
            allowed_new_stop_points: Vec::new(),
            allowed_base_vehicle_journeys: Vec::new(),
            allowed_new_vehicle_journeys: Vec::new(),
        }
    }

    pub fn fill_allowed_stops_and_vehicles(&mut self, filters: &Filters, model: &ModelRefs<'_>) {
        self.allowed_base_vehicle_journeys
            .resize(model.nb_of_base_vehicle_journeys(), true);
        for idx in model.base_vehicle_journeys() {
            let vj_idx = VehicleJourneyIdx::Base(idx);
            self.allowed_base_vehicle_journeys[idx.get()] =
                filters.is_vehicle_journey_valid(&vj_idx, model);
        }
        self.allowed_base_stop_points
            .resize(model.nb_of_base_stops(), true);
        for idx in model.base_stop_points() {
            let stop_idx = StopPointIdx::Base(idx);
            self.allowed_base_stop_points[idx.get()] =
                filters.is_stop_point_valid(&stop_idx, model);
        }

        self.allowed_new_vehicle_journeys
            .resize(model.nb_of_new_vehicle_journeys(), true);
        for idx in model.new_vehicle_journeys() {
            let vj_idx = VehicleJourneyIdx::New(idx);
            self.allowed_new_vehicle_journeys[idx.idx] =
                filters.is_vehicle_journey_valid(&vj_idx, model)
        }

        self.allowed_new_stop_points
            .resize(model.nb_of_new_stops(), true);
        for idx in model.new_stops() {
            let stop_idx = StopPointIdx::New(idx.clone());
            self.allowed_new_stop_points[idx.idx] = filters.is_stop_point_valid(&stop_idx, model);
        }
    }
}

impl<'data, 'filter, Data> TransitDataFiltered<'data, 'filter, Data>
where
    Data: data_interface::Data,
{
    pub fn is_stop_allowed(&self, stop: &Data::Stop) -> bool {
        let stop_idx = self.stop_point_idx(stop);
        match stop_idx {
            StopPointIdx::Base(idx) => self.memory.allowed_base_stop_points[idx.get()],
            StopPointIdx::New(idx) => self.memory.allowed_new_stop_points[idx.idx],
        }
    }

    pub fn is_vehicle_journey_allowed(&self, vehicle_journey_idx: &VehicleJourneyIdx) -> bool {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => self.memory.allowed_base_vehicle_journeys[idx.get()],
            VehicleJourneyIdx::New(idx) => self.memory.allowed_new_vehicle_journeys[idx.idx],
        }
    }

    pub fn new(data: &'data Data, memory: &'filter FilterMemory) -> Self {
        Self {
            transit_data: data,
            memory,
        }
    }
}

impl<Data: data_interface::Data> data_interface::TransitTypes
    for TransitDataFiltered<'_, '_, Data>
{
    type Stop = Data::Stop;
    type Mission = Data::Mission;
    type Position = Data::Position;
    type Trip = Data::Trip;
    type Transfer = Data::Transfer;
}

impl<Data> data_interface::Data for TransitDataFiltered<'_, '_, Data>
where
    Data: data_interface::Data,
{
    fn is_upstream(
        &self,
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool {
        self.transit_data.is_upstream(upstream, downstream, mission)
    }

    fn next_on_mission(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.transit_data.next_on_mission(position, mission)
    }

    fn previous_on_mission(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.transit_data.previous_on_mission(position, mission)
    }

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission {
        self.transit_data.mission_of(trip)
    }

    fn stop_of(&self, position: &Self::Position, mission: &Self::Mission) -> Self::Stop {
        self.transit_data.stop_of(position, mission)
    }

    fn board_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)> {
        let mission = self.mission_of(trip);
        let stop = self.stop_of(position, &mission);

        if self.is_stop_allowed(&stop) {
            self.transit_data.board_time_of(trip, position)
        } else {
            None
        }
    }

    fn debark_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)> {
        let mission = self.mission_of(trip);
        let stop = self.stop_of(position, &mission);

        if self.is_stop_allowed(&stop) {
            self.transit_data.debark_time_of(trip, position)
        } else {
            None
        }
    }

    fn arrival_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load) {
        self.transit_data.arrival_time_of(trip, position)
    }

    fn departure_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load) {
        self.transit_data.departure_time_of(trip, position)
    }

    fn transfer_from_to_stop(&self, transfer: &Self::Transfer) -> (Self::Stop, Self::Stop) {
        self.transit_data.transfer_from_to_stop(transfer)
    }

    fn transfer_duration(&self, transfer: &Self::Transfer) -> PositiveDuration {
        self.transit_data.transfer_duration(transfer)
    }

    fn transfer_idx(&self, transfer: &Self::Transfer) -> TransferIdx {
        self.transit_data.transfer_idx(transfer)
    }

    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &crate::time::SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        real_time_level: &RealTimeLevel,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        let stop = self.stop_of(position, mission);

        if self.is_stop_allowed(&stop) {
            self.transit_data.earliest_filtered_trip_to_board_at(
                waiting_time,
                mission,
                position,
                real_time_level,
                |vehicle_journey_idx: &VehicleJourneyIdx| {
                    self.is_vehicle_journey_allowed(vehicle_journey_idx)
                },
            )
        } else {
            None
        }
    }

    fn earliest_filtered_trip_to_board_at<Filter>(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        real_time_level: &RealTimeLevel,
        filter: Filter,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)>
    where
        Filter: Fn(&VehicleJourneyIdx) -> bool,
    {
        let stop = self.stop_of(position, mission);
        if self.is_stop_allowed(&stop) {
            self.transit_data.earliest_filtered_trip_to_board_at(
                waiting_time,
                mission,
                position,
                real_time_level,
                |vehicle_journey_idx: &VehicleJourneyIdx| {
                    self.is_vehicle_journey_allowed(vehicle_journey_idx)
                        && filter(vehicle_journey_idx)
                },
            )
        } else {
            None
        }
    }

    fn earliest_trip_that_debark_at(
        &self,
        waiting_time: &crate::time::SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        real_time_level: &RealTimeLevel,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        let stop = self.stop_of(position, mission);

        if self.is_stop_allowed(&stop) {
            self.transit_data.earliest_filtered_trip_that_debark_at(
                waiting_time,
                mission,
                position,
                real_time_level,
                |vehicle_journey_idx: &VehicleJourneyIdx| {
                    self.is_vehicle_journey_allowed(vehicle_journey_idx)
                },
            )
        } else {
            None
        }
    }

    fn earliest_filtered_trip_that_debark_at<Filter>(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        real_time_level: &RealTimeLevel,
        filter: Filter,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)>
    where
        Filter: Fn(&VehicleJourneyIdx) -> bool,
    {
        let stop = self.stop_of(position, mission);
        if self.is_stop_allowed(&stop) {
            self.transit_data.earliest_filtered_trip_that_debark_at(
                waiting_time,
                mission,
                position,
                real_time_level,
                |vehicle_journey_idx: &VehicleJourneyIdx| {
                    self.is_vehicle_journey_allowed(vehicle_journey_idx)
                        && filter(vehicle_journey_idx)
                },
            )
        } else {
            None
        }
    }

    fn latest_trip_that_debark_at(
        &self,
        waiting_time: &crate::time::SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        real_time_level: &RealTimeLevel,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        let stop = self.stop_of(position, mission);

        if self.is_stop_allowed(&stop) {
            self.transit_data.latest_filtered_trip_that_debark_at(
                waiting_time,
                mission,
                position,
                real_time_level,
                |vehicle_journey_idx: &VehicleJourneyIdx| {
                    self.is_vehicle_journey_allowed(vehicle_journey_idx)
                },
            )
        } else {
            None
        }
    }

    fn latest_filtered_trip_that_debark_at<Filter>(
        &self,
        waiting_time: &crate::time::SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        real_time_level: &RealTimeLevel,
        filter: Filter,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)>
    where
        Filter: Fn(&VehicleJourneyIdx) -> bool,
    {
        let stop = self.stop_of(position, mission);

        if self.is_stop_allowed(&stop) {
            self.transit_data.latest_filtered_trip_that_debark_at(
                waiting_time,
                mission,
                position,
                real_time_level,
                |vehicle_journey_idx: &VehicleJourneyIdx| {
                    self.is_vehicle_journey_allowed(vehicle_journey_idx)
                        && filter(vehicle_journey_idx)
                },
            )
        } else {
            None
        }
    }

    fn to_naive_datetime(
        &self,
        seconds: &crate::time::SecondsSinceDatasetUTCStart,
    ) -> chrono::NaiveDateTime {
        self.transit_data.calendar().to_naive_datetime(seconds)
    }

    fn vehicle_journey_idx(&self, trip: &Self::Trip) -> VehicleJourneyIdx {
        self.transit_data.vehicle_journey_idx(trip)
    }

    fn stop_point_idx(&self, stop: &Self::Stop) -> StopPointIdx {
        self.transit_data.stop_point_idx(stop)
    }

    fn stoptime_idx(&self, position: &Self::Position, trip: &Self::Trip) -> StopTimeIdx {
        self.transit_data.stoptime_idx(position, trip)
    }

    fn day_of(&self, trip: &Self::Trip) -> chrono::NaiveDate {
        self.transit_data.day_of(trip)
    }

    fn is_same_stop(&self, stop_a: &Self::Stop, stop_b: &Self::Stop) -> bool {
        self.transit_data.is_same_stop(stop_a, stop_b)
    }

    fn calendar(&self) -> &Calendar {
        self.transit_data.calendar()
    }

    fn stop_point_idx_to_stop(&self, stop_point_idx: &StopPointIdx) -> Option<Self::Stop> {
        self.transit_data.stop_point_idx_to_stop(stop_point_idx)
    }

    fn nb_of_trips(&self) -> usize {
        self.transit_data.nb_of_trips()
    }

    fn nb_of_stops(&self) -> usize {
        self.transit_data.nb_of_stops()
    }

    fn stop_id(&self, stop: &Self::Stop) -> usize {
        self.transit_data.stop_id(stop)
    }

    fn nb_of_missions(&self) -> usize {
        self.transit_data.nb_of_missions()
    }

    fn mission_id(&self, mission: &Self::Mission) -> usize {
        self.transit_data.mission_id(mission)
    }
}

impl<'data, Data> data_interface::DataIters<'data> for TransitDataFiltered<'_, '_, Data>
where
    Data: data_interface::Data + data_interface::DataIters<'data>,
    Data::Mission: 'data,
    Data::Position: 'data,
{
    type MissionsAtStop = Data::MissionsAtStop;

    fn missions_at(&'data self, stop: &Self::Stop) -> Self::MissionsAtStop {
        self.transit_data.missions_at(stop)
    }

    type OutgoingTransfersAtStop = Data::OutgoingTransfersAtStop;
    fn outgoing_transfers_at(&'data self, from_stop: &Self::Stop) -> Self::OutgoingTransfersAtStop {
        self.transit_data.outgoing_transfers_at(from_stop)
    }

    type IncomingTransfersAtStop = Data::IncomingTransfersAtStop;
    fn incoming_transfers_at(&'data self, stop: &Self::Stop) -> Self::IncomingTransfersAtStop {
        self.transit_data.incoming_transfers_at(stop)
    }

    type TripsOfMission = Data::TripsOfMission;

    fn trips_of(
        &'data self,
        mission: &Self::Mission,
        real_time_level: &RealTimeLevel,
    ) -> Self::TripsOfMission {
        self.transit_data.trips_of(mission, real_time_level)
    }
}

impl<Data> data_interface::DataWithIters for TransitDataFiltered<'_, '_, Data>
where
    Data: DataWithIters,
    Data::Mission: 'static,
    Data::Position: 'static,
{
}
