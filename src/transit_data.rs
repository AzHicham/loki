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

use chrono::NaiveDate;
use iters::MissionsOfStop;
pub use transit_model::objects::{
    StopPoint, Time as TransitModelTime, Transfer as TransitModelTransfer, VehicleJourney,
};
pub use typed_index_collection::Idx;

use crate::{loads_data::{Load, LoadsData}, model::{ModelRefs, StopPointIdx, TransferIdx, VehicleJourneyIdx}, time::{
        Calendar, PositiveDuration, SecondsSinceDatasetUTCStart, SecondsSinceTimezonedDayStart,
    }, timetables::{generic_timetables::VehicleTimesError, FlowDirection, InsertionError}};

use std::{collections::HashMap, fmt::Debug};

use crate::timetables::{RemovalError, Timetables as TimetablesTrait, TimetablesIter};

use crate::transit_model::Model;

use crate::tracing::error;

pub struct TransitData<Timetables: TimetablesTrait> {
    pub(super) stop_point_idx_to_stop: HashMap<StopPointIdx, Stop>,

    pub(super) stops_data: Vec<StopData<Timetables>>,
    pub(super) timetables: Timetables,

    pub(super) transfers_data: Vec<TransferData>,
}

pub struct StopData<Timetables: TimetablesTrait> {
    pub(super) stop_point_idx: StopPointIdx,
    pub(super) position_in_timetables: Vec<(Timetables::Mission, Timetables::Position)>,
    pub(super) outgoing_transfers: Vec<(Stop, TransferDurations, Transfer)>,
    pub(super) incoming_transfers: Vec<(Stop, TransferDurations, Transfer)>,
}

#[derive(Debug, Clone)]
pub struct TransferDurations {
    pub walking_duration: PositiveDuration,
    pub total_duration: PositiveDuration, // = walking_duration + some waiting time
}

pub struct TransferData {
    pub from_stop: Stop,
    pub to_stop: Stop,
    pub durations: TransferDurations,
    pub transit_model_transfer_idx: TransferIdx,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Ord, PartialOrd)]
pub struct Stop {
    pub(super) idx: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Transfer {
    pub(super) idx: usize,
}

impl<Timetables: TimetablesTrait> TransitData<Timetables> {
    pub fn stop_data(&self, stop: &Stop) -> &StopData<Timetables> {
        &self.stops_data[stop.idx]
    }

    pub fn stop_point_idx_to_stop(&self, stop_point_idx: &StopPointIdx) -> Option<&Stop> {
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

    fn transfer_from_to_stop(&self, transfer: &Self::Transfer) -> (Self::Stop, Self::Stop) {
        let transfer_data = &self.transfers_data[transfer.idx];
        (transfer_data.from_stop, transfer_data.to_stop)
    }

    fn transfer_duration(&self, transfer: &Self::Transfer) -> PositiveDuration {
        let transfer_data = &self.transfers_data[transfer.idx];
        transfer_data.durations.total_duration
    }

    fn transfer_transit_model_idx(&self, transfer: &Self::Transfer) -> TransferIdx {
        let transfer_data = &self.transfers_data[transfer.idx];
        transfer_data.transit_model_transfer_idx.clone()
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

    fn to_naive_datetime(
        &self,
        seconds: &crate::time::SecondsSinceDatasetUTCStart,
    ) -> chrono::NaiveDateTime {
        self.timetables.calendar().to_naive_datetime(seconds)
    }

    fn vehicle_journey_idx(&self, trip: &Self::Trip) -> VehicleJourneyIdx {
        self.timetables.vehicle_journey_idx(trip)
    }

    fn stop_point_idx(&self, stop: &Stop) -> StopPointIdx {
        self.stops_data[stop.idx].stop_point_idx.clone()
    }

    fn stoptime_idx(&self, position: &Self::Position, trip: &Self::Trip) -> usize {
        self.timetables.stoptime_idx(position, trip)
    }

    fn day_of(&self, trip: &Self::Trip) -> chrono::NaiveDate {
        self.timetables.day_of(trip)
    }

    fn is_same_stop(&self, stop_a: &Self::Stop, stop_b: &Self::Stop) -> bool {
        stop_a.idx == stop_b.idx
    }

    fn calendar(&self) -> &Calendar {
        self.timetables.calendar()
    }

    fn stop_point_idx_to_stop(&self, stop_point_idx: &StopPointIdx) -> Option<Self::Stop> {
        self.stop_point_idx_to_stop.get(stop_point_idx).copied()
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
}

impl<Timetables> data_interface::DataUpdate for TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
{
    fn remove_vehicle(
        &mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &chrono::NaiveDate,
    ) -> Result<(), RemovalError> {
        self.timetables.remove(date, vehicle_journey_idx)
    }

    fn add_vehicle<'date, Stops, Flows, Dates, BoardTimes, DebarkTimes>(
        &mut self,
        stop_points: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads_data: &LoadsData,
        valid_dates: Dates,
        timezone: &chrono_tz::Tz,
        vehicle_journey_idx: VehicleJourneyIdx,
    ) -> Vec<InsertionError>
    where
        Stops: Iterator<Item = StopPointIdx> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        Dates: Iterator<Item = &'date chrono::NaiveDate>,
        BoardTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
    {
        let stops = self.create_stops(stop_points.clone()).into_iter();
        let (missions, insertion_errors) = self.timetables.insert(
            stops,
            flows,
            board_times,
            debark_times,
            loads_data,
            valid_dates,
            timezone,
            &vehicle_journey_idx,
        );

        for mission in missions.iter() {
            for position in self.timetables.positions(mission) {
                let stop = self.timetables.stop_at(&position, mission);
                let stop_data = &mut self.stops_data[stop.idx];
                stop_data
                    .position_in_timetables
                    .push((mission.clone(), position));
            }
        }

        insertion_errors

        // for error in insertion_errors {
        //     let vehicle_journey_name = real_time_model.vehicle_journey_name(&vehicle_journey_idx, model);
        //     use crate::timetables::InsertionError::*;
        //     match error {
        //         Times(error, dates) => {
        //             handle_vehicletimes_error(vehicle_journey_name, &dates, stop_points.clone(), real_time_model, model, &error);
        //         }
        //         VehicleJourneyAlreadyExistsOnDate(date) => {
        //             error!(
        //                 "Trying to insert the vehicle journey {} more than once on day {}",
        //                 vehicle_journey_name, date
        //             );
        //         }
        //         DateOutOfCalendar(date) => {
        //             use crate::transit_data::data_interface::Data;
        //             error!(
        //                 "Trying to insert the vehicle journey {} on day {},  \
        //                     but this day is not allowed in the calendar.  \
        //                     Allowed dates are between {} and {}",
        //                 vehicle_journey_name,
        //                 date,
        //                 self.calendar().first_date(),
        //                 self.calendar().last_date(),
        //             );
        //         }
        //     }
        // }

        // Ok(())
    }

    fn modify_vehicle<'date, Stops, Flows, Dates, BoardTimes, DebarkTimes>(
        &mut self,
        stops: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads_data: &LoadsData,
        valid_dates: Dates,
        timezone: &chrono_tz::Tz,
        vehicle_journey_idx: VehicleJourneyIdx,
    ) -> (Vec<RemovalError>, Vec<InsertionError>)
    where
        Stops: Iterator<Item = StopPointIdx> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        Dates: Iterator<Item = &'date chrono::NaiveDate> + Clone,
        BoardTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
    {
        let mut removal_errors = Vec::new();
        let mut insertion_errors = Vec::new();
        for date in valid_dates.clone() {
            let removal_result = self.remove_vehicle(&vehicle_journey_idx, date);
            match removal_result {
                Ok(()) => {
                    let errors = self.add_vehicle(
                        stops.clone(),
                        flows.clone(),
                        board_times.clone(),
                        debark_times.clone(),
                        loads_data,
                        valid_dates.clone(),
                        timezone,
                        vehicle_journey_idx.clone(),
                    );
                    insertion_errors.extend_from_slice(errors.as_slice());
                }
                Err(removal_error) => {
                    removal_errors.push(removal_error);
                }
            }
        }
        (removal_errors, insertion_errors)
    }
}

impl<Timetables: TimetablesTrait> data_interface::DataIO for TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
{
    fn new(
        model: &Model,
        loads_data: &LoadsData,
        default_transfer_duration: PositiveDuration,
    ) -> Self {
        Self::_new(model, loads_data, default_transfer_duration)
    }
}

impl<'a, Timetables> data_interface::DataIters<'a> for TransitData<Timetables>
where
    Timetables: TimetablesTrait + TimetablesIter<'a>,
    Timetables::Mission: 'a,
    Timetables::Position: 'a,
{
    type MissionsAtStop = MissionsOfStop<'a, Timetables>;

    fn missions_at(&'a self, stop: &Self::Stop) -> Self::MissionsAtStop {
        self.missions_of(stop)
    }

    type OutgoingTransfersAtStop = iters::OutgoingTransfersAtStop<'a>;
    fn outgoing_transfers_at(&'a self, from_stop: &Self::Stop) -> Self::OutgoingTransfersAtStop {
        self.outgoing_transfers_at(from_stop)
    }

    type IncomingTransfersAtStop = iters::IncomingTransfersAtStop<'a>;
    fn incoming_transfers_at(&'a self, stop: &Self::Stop) -> Self::IncomingTransfersAtStop {
        self.incoming_transfers_at(stop)
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

fn handle_vehicletimes_error(
    vehicle_journey_name: &str,
    dates: &[NaiveDate],
    stop_points: impl Iterator<Item = StopPointIdx> + Clone,
    model : & ModelRefs<'_>,
    error: &VehicleTimesError,
) {
    let days_strings: Vec<String> = dates
        .iter()
        .map(|date| date.format("%H:%M:%S %d-%b-%y").to_string())
        .collect();

    match error {
        VehicleTimesError::DebarkBeforeUpstreamBoard(position_pair) => {
            let upstream_stop_name = stop_points
                .clone()
                .nth(position_pair.upstream)
                .map(|stop_point_idx| model.stop_point_name(&stop_point_idx))
                .unwrap_or_else(|| "unknown_stopp_time");
            let downstream_stop_name = stop_points
                .clone()
                .nth(position_pair.downstream)
                .map(|stop_point_idx| model.stop_point_name(&stop_point_idx))
                .unwrap_or_else(|| "unknown_stopp_time");
            error!(
                "Skipping vehicle journey {} on days {:?} because its \
                    debark time at {}-th stop_time ({}) \
                    is earlier than its \
                    board time upstream {}-th stop_time ({}). ",
                vehicle_journey_name,
                days_strings,
                position_pair.downstream,
                downstream_stop_name,
                position_pair.upstream,
                upstream_stop_name
            );
        }
        VehicleTimesError::DecreasingBoardTime(position_pair) => {
            let upstream_stop_name = stop_points
                .clone()
                .nth(position_pair.upstream)
                .map(|stop_point_idx| model.stop_point_name(&stop_point_idx))
                .unwrap_or_else(|| "unknown_stopp_time");
            let downstream_stop_name = stop_points
                .clone()
                .nth(position_pair.downstream)
                .map(|stop_point_idx| model.stop_point_name(&stop_point_idx))
                .unwrap_or_else(|| "unknown_stopp_time");
            error!(
                "Skipping vehicle journey {} on days {:?} because its \
                    board time at {}-th stop_time ({}) \
                    is earlier than its \
                    board time upstream at {}-th stop_time ({}). ",
                vehicle_journey_name,
                days_strings,
                position_pair.downstream,
                downstream_stop_name,
                position_pair.upstream,
                upstream_stop_name
            );
        }
        VehicleTimesError::DecreasingDebarkTime(position_pair) => {
            let upstream_stop_name = stop_points
                .clone()
                .nth(position_pair.upstream)
                .map(|stop_point_idx| model.stop_point_name(&stop_point_idx))
                .unwrap_or_else(|| "unknown_stopp_time");
            let downstream_stop_name = stop_points
                .clone()
                .nth(position_pair.downstream)
                .map(|stop_point_idx| model.stop_point_name(&stop_point_idx))
                .unwrap_or_else(|| "unknown_stopp_time");
            error!(
                "Skipping vehicle journey {} on days {:?} because its \
                    debark time at {}-th stop_time ({}) \
                    is earlier than its \
                    debark time upstream at {}-th stop_time ({}). ",
                vehicle_journey_name,
                days_strings,
                position_pair.downstream,
                downstream_stop_name,
                position_pair.upstream,
                upstream_stop_name
            );
        }
    }
}
