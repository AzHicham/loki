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

mod day_to_timetable;
pub(crate) mod generic_timetables;
mod iters;
// mod daily;
// mod periodic;
mod periodic_split_vj_by_tz;

// pub use daily::DailyTimetables;
// pub use periodic::PeriodicTimetables;
pub use periodic_split_vj_by_tz::PeriodicSplitVjByTzTimetables;

use std::hash::Hash;

pub use crate::transit_data::Stop;

use crate::{
    loads_data::{Load, LoadsData},
    models::VehicleJourneyIdx,
    time::{Calendar, SecondsSinceDatasetUTCStart, SecondsSinceTimezonedDayStart},
    transit_data::data_interface::RealTimeLevel,
};

use chrono::NaiveDate;
use std::fmt::Debug;

use self::generic_timetables::VehicleTimesError;

#[derive(Debug, Clone)]
pub enum RealTimeValidity {
    BaseOnly,
    RealTimeOnly,
    BaseAndRealTime,
}

impl RealTimeValidity {
    pub fn is_valid_for(&self, real_time_level: RealTimeLevel) -> bool {
        match (self, real_time_level) {
            (RealTimeValidity::BaseAndRealTime, _) => true,
            (RealTimeValidity::BaseOnly, RealTimeLevel::Base) => true,
            (RealTimeValidity::RealTimeOnly, RealTimeLevel::RealTime) => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub enum FlowDirection {
    BoardOnly,
    DebarkOnly,
    BoardAndDebark,
    NoBoardDebark,
}
pub type StopFlows = Vec<(Stop, FlowDirection)>;

pub trait Types {
    type Mission: Debug + Clone + Hash + Eq;
    type Position: Debug + Clone;
    type Trip: Debug + Clone;
}

pub trait Timetables: Types {
    fn new(first_date: NaiveDate, last_date: NaiveDate) -> Self;

    fn calendar(&self) -> &Calendar;

    fn nb_of_missions(&self) -> usize;
    fn mission_id(&self, mission: &Self::Mission) -> usize;

    fn vehicle_journey_idx(&self, trip: &Self::Trip) -> VehicleJourneyIdx;
    fn stoptime_idx(&self, position: &Self::Position, trip: &Self::Trip) -> usize;
    fn day_of(&self, trip: &Self::Trip) -> NaiveDate;

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission;
    fn stop_at(&self, position: &Self::Position, mission: &Self::Mission) -> Stop;

    fn nb_of_trips(&self) -> usize;

    fn is_upstream_in_mission(
        &self,
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool;

    fn next_position(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position>;

    fn previous_position(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position>;

    fn arrival_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load);

    fn departure_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load);

    fn debark_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)>;

    fn board_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)>;

    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)>;

    fn earliest_filtered_trip_to_board_at<Filter>(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        filter: Filter,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)>
    where
        Filter: Fn(&VehicleJourneyIdx) -> bool;

    fn latest_trip_that_debark_at(
        &self,
        time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)>;

    fn latest_filtered_trip_that_debark_at<Filter>(
        &self,
        time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        filter: Filter,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)>
    where
        Filter: Fn(&VehicleJourneyIdx) -> bool;

    fn insert<'date, Stops, Flows, Dates, BoardTimes, DebarkTimes>(
        &mut self,
        stops: Stops,
        flows: Flows,
        board_times: BoardTimes,
        debark_times: DebarkTimes,
        loads_data: &LoadsData,
        valid_dates: Dates,
        timezone: &chrono_tz::Tz,
        vehicle_journey_idx: &VehicleJourneyIdx,
        real_time_validity: &RealTimeValidity,
    ) -> (Vec<Self::Mission>, Vec<InsertionError>)
    where
        Stops: Iterator<Item = Stop> + ExactSizeIterator + Clone,
        Flows: Iterator<Item = FlowDirection> + ExactSizeIterator + Clone,
        Dates: Iterator<Item = &'date chrono::NaiveDate>,
        BoardTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone,
        DebarkTimes: Iterator<Item = SecondsSinceTimezonedDayStart> + ExactSizeIterator + Clone;

    fn remove(
        &mut self,
        date: &chrono::NaiveDate,
        vehicle_journey_idx: &VehicleJourneyIdx,
        real_time_level: &RealTimeLevel,
    ) -> Result<(), RemovalError>;
}

#[derive(Clone, Debug)]
pub enum InsertionError {
    Times(VehicleJourneyIdx, VehicleTimesError, Vec<NaiveDate>),
    VehicleJourneyAlreadyExistsOnDate(NaiveDate, VehicleJourneyIdx),
    InvalidDate(NaiveDate, VehicleJourneyIdx),
}

#[derive(Clone, Debug)]
pub enum RemovalError {
    UnknownDate(NaiveDate, VehicleJourneyIdx),
    UnknownVehicleJourney(VehicleJourneyIdx),
    DateInvalidForVehicleJourney(NaiveDate, VehicleJourneyIdx),
}

pub trait TimetablesIter<'a>: Types {
    type Positions: Iterator<Item = Self::Position>;
    fn positions(&'a self, mission: &Self::Mission) -> Self::Positions;

    type Trips: Iterator<Item = Self::Trip>;
    fn trips_of(&'a self, mission: &Self::Mission) -> Self::Trips;

    type Missions: Iterator<Item = Self::Mission>;
    fn missions(&'a self) -> Self::Missions;
}
