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
pub mod data_update;
pub mod init;
pub mod iters;

use chrono::NaiveDate;
use iters::MissionsOfStop;

use crate::{
    loads_data::Load,
    models::{
        base_model::BaseModel, ModelRefs, StopPointIdx, StopTimeIdx, TransferIdx, VehicleJourneyIdx,
    },
    time::{days_patterns::DaysPatterns, Calendar, PositiveDuration, SecondsSinceDatasetUTCStart},
    timetables::{
        day_to_timetable::VehicleJourneyToTimetable,
        generic_timetables::{PositionPair, VehicleTimesError},
        InsertionError, ModifyError,
    },
    RealTimeLevel,
};

use std::{collections::HashMap, fmt::Debug};

use crate::timetables::{RemovalError, Timetables as TimetablesTrait, TimetablesIter};

use crate::tracing::error;

pub struct TransitData<Timetables: TimetablesTrait> {
    pub(super) stop_point_idx_to_stop: HashMap<StopPointIdx, Stop>,

    pub(super) stops_data: Vec<StopData<Timetables>>,
    pub(super) timetables: Timetables,

    pub(super) transfers_data: Vec<TransferData>,

    pub(super) vehicle_journey_to_timetable: VehicleJourneyToTimetable<Timetables::Mission>,

    pub(super) calendar: Calendar,
    pub(super) days_patterns: DaysPatterns,
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
    pub bike_accessible: bool,
    pub wheelchair_accessible: bool,
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
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a>,
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
        self.timetables
            .board_time_of(trip, position, &self.calendar)
    }

    fn debark_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)> {
        self.timetables
            .debark_time_of(trip, position, &self.calendar)
    }

    fn arrival_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load) {
        self.timetables
            .arrival_time_of(trip, position, &self.calendar)
    }

    fn departure_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load) {
        self.timetables
            .departure_time_of(trip, position, &self.calendar)
    }

    fn transfer_from_to_stop(&self, transfer: &Self::Transfer) -> (Self::Stop, Self::Stop) {
        let transfer_data = &self.transfers_data[transfer.idx];
        (transfer_data.from_stop, transfer_data.to_stop)
    }

    fn transfer_duration(&self, transfer: &Self::Transfer) -> PositiveDuration {
        let transfer_data = &self.transfers_data[transfer.idx];
        transfer_data.durations.total_duration
    }

    fn transfer_idx(&self, transfer: &Self::Transfer) -> TransferIdx {
        let transfer_data = &self.transfers_data[transfer.idx];
        transfer_data.transit_model_transfer_idx.clone()
    }

    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &crate::time::SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        real_time_level: &RealTimeLevel,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        self.timetables.earliest_trip_to_board_at(
            waiting_time,
            mission,
            position,
            real_time_level,
            &self.calendar,
            &self.days_patterns,
        )
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
        self.timetables.earliest_filtered_trip_to_board_at(
            waiting_time,
            mission,
            position,
            real_time_level,
            filter,
            &self.calendar,
            &self.days_patterns,
        )
    }

    fn latest_trip_that_debark_at(
        &self,
        waiting_time: &crate::time::SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        real_time_level: &RealTimeLevel,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        self.timetables.latest_trip_that_debark_at(
            waiting_time,
            mission,
            position,
            real_time_level,
            &self.calendar,
            &self.days_patterns,
        )
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
        self.timetables.latest_filtered_trip_that_debark_at(
            waiting_time,
            mission,
            position,
            real_time_level,
            filter,
            &self.calendar,
            &self.days_patterns,
        )
    }

    fn to_naive_datetime(
        &self,
        seconds: &crate::time::SecondsSinceDatasetUTCStart,
    ) -> chrono::NaiveDateTime {
        self.calendar.to_naive_datetime(seconds)
    }

    fn vehicle_journey_idx(&self, trip: &Self::Trip) -> VehicleJourneyIdx {
        self.timetables.vehicle_journey_idx(trip)
    }

    fn stop_point_idx(&self, stop: &Stop) -> StopPointIdx {
        self.stops_data[stop.idx].stop_point_idx.clone()
    }

    fn stoptime_idx(&self, position: &Self::Position, trip: &Self::Trip) -> StopTimeIdx {
        let idx = self.timetables.stoptime_idx(position, trip);
        StopTimeIdx { idx }
    }

    fn day_of(&self, trip: &Self::Trip) -> chrono::NaiveDate {
        let day = self.timetables.day_of(trip);
        self.calendar.to_naive_date(&day)
    }

    fn is_same_stop(&self, stop_a: &Self::Stop, stop_b: &Self::Stop) -> bool {
        stop_a.idx == stop_b.idx
    }

    fn calendar(&self) -> &Calendar {
        &self.calendar
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

impl<Timetables: TimetablesTrait> data_interface::DataIO for TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a>,
{
    fn new(base_model: &BaseModel) -> Self {
        Self::_new(base_model)
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
        self.inner_outgoing_transfers_at(from_stop, false, false)
    }
    fn inner_outgoing_transfers_at(
        &'a self,
        from_stop: &Self::Stop,
        must_be_bike_accessible: bool,
        must_be_wheelchair_accessible: bool,
    ) -> Self::OutgoingTransfersAtStop {
        self.outgoing_transfers_at(
            from_stop,
            must_be_bike_accessible,
            must_be_wheelchair_accessible,
        )
    }

    type IncomingTransfersAtStop = iters::IncomingTransfersAtStop<'a>;
    fn incoming_transfers_at(&'a self, stop: &Self::Stop) -> Self::IncomingTransfersAtStop {
        self.inner_incoming_transfers_at(stop, false, false)
    }

    fn inner_incoming_transfers_at(
        &'a self,
        stop: &Self::Stop,
        must_be_bike_accessible: bool,
        must_be_wheelchair_accessible: bool,
    ) -> Self::IncomingTransfersAtStop {
        self.incoming_transfers_at(stop, must_be_bike_accessible, must_be_wheelchair_accessible)
    }

    type TripsOfMission = <Timetables as TimetablesIter<'a>>::Trips;

    fn trips_of(
        &'a self,
        mission: &Self::Mission,
        real_time_level: &RealTimeLevel,
    ) -> Self::TripsOfMission {
        self.timetables
            .trips_of(mission, real_time_level, &self.days_patterns)
    }
}

impl<Timetables> data_interface::DataWithIters for TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a>,
    Timetables::Mission: 'static,
    Timetables::Position: 'static,
{
}

pub fn handle_insertion_error(
    model: &ModelRefs,
    start_date: &NaiveDate,
    end_date: &NaiveDate,
    insertion_error: &InsertionError,
) {
    use crate::timetables::InsertionError::*;
    match insertion_error {
        Times(vehicle_journey_idx, real_time_level, error, dates) => {
            let _ = handle_vehicletimes_error(
                vehicle_journey_idx,
                dates,
                model,
                error,
                real_time_level,
            );
        }
        NoValidDates(vehicle_journey_idx) => {
            let vehicle_journey_name = model.vehicle_journey_name(vehicle_journey_idx);
            error!(
                "Trying to insert the vehicle journey {} with no valid dates.",
                vehicle_journey_name,
            );
        }
        RealTimeVehicleJourneyAlreadyExistsOnDate(date, vehicle_journey_idx) => {
            let vehicle_journey_name = model.vehicle_journey_name(vehicle_journey_idx);
            error!(
                "Trying to insert the real time vehicle journey {} more than once on day {}",
                vehicle_journey_name, date
            );
        }
        InvalidDate(date, vehicle_journey_idx) => {
            let vehicle_journey_name = model.vehicle_journey_name(vehicle_journey_idx);
            error!(
                "Trying to insert the vehicle journey {} on day {},  \
                        but this day is not allowed in the calendar.  \
                        Allowed dates are between {} and {}",
                vehicle_journey_name, date, start_date, end_date,
            );
        }
        BaseVehicleJourneyAlreadyExists(vehicle_journey_idx) => {
            let vehicle_journey_name = model.vehicle_journey_name(vehicle_journey_idx);
            error!(
                "Trying to insert the base vehicle journey {} more than once.",
                vehicle_journey_name
            );
        }
    }
}

fn handle_vehicletimes_error(
    vehicle_journey_idx: &VehicleJourneyIdx,
    dates: &[NaiveDate],
    model: &ModelRefs<'_>,
    error: &VehicleTimesError,
    real_time_level: &RealTimeLevel,
) -> Result<(), ()> {
    if dates.is_empty() {
        error!("Received a vehicle times error with no date. {:?}", error);
        return Err(());
    }

    let date = dates.first().unwrap();

    let days_strings = if dates.len() == 1 {
        date.format("%Y-%m-%d").to_string()
    } else {
        format!(
            "{} and ({} others)",
            date.format("%Y-%m-%d"),
            dates.len() - 1
        )
    };

    let vehicle_journey_name = model.vehicle_journey_name(vehicle_journey_idx);

    match error {
        VehicleTimesError::LessThanTwoStops => {
            error!(
                "Skipping vehicle journey {} because it has less than 2 stops",
                vehicle_journey_name,
            );
        }
        VehicleTimesError::DebarkBeforeUpstreamBoard(position_pair) => {
            let (upstream_stop_name, downstream_stop_name) = upstream_downstream_stop_uris(
                model,
                vehicle_journey_idx,
                date,
                position_pair,
                real_time_level,
            )?;
            error!(
                "Skipping vehicle journey {} on day {:?} because its \
                    debark time at {:?}-th stop_time ({}) \
                    is earlier than its \
                    board time upstream {:?}-th stop_time ({}). ",
                vehicle_journey_name,
                days_strings,
                position_pair.downstream.idx,
                downstream_stop_name,
                position_pair.upstream.idx,
                upstream_stop_name
            );
        }
        VehicleTimesError::DecreasingBoardTime(position_pair) => {
            let (upstream_stop_name, downstream_stop_name) = upstream_downstream_stop_uris(
                model,
                vehicle_journey_idx,
                date,
                position_pair,
                real_time_level,
            )?;
            error!(
                "Skipping vehicle journey {} on day {:?} because its \
                    board time at {:?}-th stop_time ({}) \
                    is earlier than its \
                    board time upstream at {:?}-th stop_time ({}). ",
                vehicle_journey_name,
                days_strings,
                position_pair.downstream,
                downstream_stop_name,
                position_pair.upstream,
                upstream_stop_name
            );
        }
        VehicleTimesError::DecreasingDebarkTime(position_pair) => {
            let (upstream_stop_name, downstream_stop_name) = upstream_downstream_stop_uris(
                model,
                vehicle_journey_idx,
                date,
                position_pair,
                real_time_level,
            )?;
            error!(
                "Skipping vehicle journey {} on day {:?} because its \
                    debark time at {:?}-th stop_time ({}) \
                    is earlier than its \
                    debark time upstream at {:?}-th stop_time ({}). ",
                vehicle_journey_name,
                days_strings,
                position_pair.downstream,
                downstream_stop_name,
                position_pair.upstream,
                upstream_stop_name
            );
        }
    }

    Ok(())
}

fn upstream_downstream_stop_uris<'model>(
    model: &'model ModelRefs<'model>,
    vehicle_journey_idx: &VehicleJourneyIdx,
    date: &NaiveDate,
    position_pair: &PositionPair,
    real_time_level: &RealTimeLevel,
) -> Result<(String, String), ()> {
    let upstream_stop = model
        .stop_point_at(
            vehicle_journey_idx,
            position_pair.upstream,
            date,
            real_time_level,
        )
        .ok_or_else(|| {
            error!(
                "Received a position pair with invalid upstream stop. \
                    Vehicle journey {} on {} upstream {:?}.",
                model.vehicle_journey_name(vehicle_journey_idx),
                date,
                position_pair.upstream
            )
        })?;
    let upstream_stop_name = model.stop_point_uri(&upstream_stop);

    let dowstream_stop = model
        .stop_point_at(
            vehicle_journey_idx,
            position_pair.downstream,
            date,
            real_time_level,
        )
        .ok_or_else(|| {
            error!(
                "Received a position pair with invalid downstream stop. \
                    Vehicle journey {} on {} downstream {:?}.",
                model.vehicle_journey_name(vehicle_journey_idx),
                date,
                position_pair.downstream
            )
        })?;

    let downstream_stop_name = model.stop_point_uri(&dowstream_stop);

    Ok((upstream_stop_name, downstream_stop_name))
}

pub fn handle_removal_error(
    model: &ModelRefs,
    start_date: &NaiveDate,
    end_date: &NaiveDate,
    error: &RemovalError,
) {
    match error {
        RemovalError::UnknownDate(date, vehicle_journey_idx) => {
            let vehicle_journey_name = model.vehicle_journey_name(vehicle_journey_idx);
            error!(
                "Trying to remove the vehicle journey {} on day {},  \
                    but this day is not allowed in the data.  \
                    Allowed dates are between {} and {}",
                vehicle_journey_name, date, start_date, end_date,
            );
        }
        RemovalError::UnknownVehicleJourney(vehicle_journey_idx) => {
            let vehicle_journey_name = model.vehicle_journey_name(vehicle_journey_idx);
            error!(
                "Trying to remove the vehicle journey {} \
                    but this vehicle journey is unknown",
                vehicle_journey_name
            );
        }
        RemovalError::DateInvalidForVehicleJourney(date, vehicle_journey_idx) => {
            let vehicle_journey_name = model.vehicle_journey_name(vehicle_journey_idx);
            error!(
                "Trying to remove the vehicle journey {} on day {},  \
                    but this vehicle journeys does not exists on this day. ",
                vehicle_journey_name, date,
            );
        }
    }
}

pub fn handle_modify_error(
    model: &ModelRefs,
    start_date: &NaiveDate,
    end_date: &NaiveDate,
    modify_error: &ModifyError,
) {
    match modify_error {
        ModifyError::UnknownDate(date, vehicle_journey_idx) => {
            let vehicle_journey_name = model.vehicle_journey_name(vehicle_journey_idx);
            error!(
                "Trying to modify the vehicle journey {} on day {},  \
                    but this day is not allowed in the data.  \
                    Allowed dates are between {} and {}",
                vehicle_journey_name, date, start_date, end_date,
            );
        }
        ModifyError::UnknownVehicleJourney(vehicle_journey_idx) => {
            let vehicle_journey_name = model.vehicle_journey_name(vehicle_journey_idx);
            error!(
                "Trying to modify the vehicle journey {} \
                    but this vehicle journey is unknown",
                vehicle_journey_name
            );
        }
        ModifyError::DateInvalidForVehicleJourney(date, vehicle_journey_idx) => {
            let vehicle_journey_name = model.vehicle_journey_name(vehicle_journey_idx);
            error!(
                "Trying to modify the vehicle journey {} on day {},  \
                    but this vehicle journeys does not exists on this day. ",
                vehicle_journey_name, date,
            );
        }
        ModifyError::Times(vehicle_journey_idx, times_err, dates) => {
            let _ = handle_vehicletimes_error(
                vehicle_journey_idx,
                dates,
                model,
                times_err,
                &RealTimeLevel::RealTime,
            );
        }
    }
}
