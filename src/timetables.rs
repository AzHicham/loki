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

pub mod day_to_timetable;
pub(crate) mod generic_timetables;
mod iters;

pub mod periodic_split_vj_by_tz;

pub use periodic_split_vj_by_tz::PeriodicSplitVjByTzTimetables;

use std::hash::Hash;

pub use crate::transit_data::Stop;

use crate::{
    models::VehicleJourneyIdx, time::days_patterns::DaysPatterns,
    transit_data::data_interface::RealTimeLevel,
};

use chrono::NaiveDate;
use std::fmt::Debug;

use self::generic_timetables::VehicleTimesError;

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
    type Position: Debug + Clone + PartialEq + Eq;
    type Trip: Debug + Clone;
}

#[derive(Clone, Debug)]
pub enum InsertionError {
    Times(
        VehicleJourneyIdx,
        RealTimeLevel,
        VehicleTimesError,
        Vec<NaiveDate>,
    ),
    BaseVehicleJourneyAlreadyExists(VehicleJourneyIdx),
    RealTimeVehicleJourneyAlreadyExistsOnDate(NaiveDate, VehicleJourneyIdx),
    InvalidDate(NaiveDate, VehicleJourneyIdx),
    NoValidDates(VehicleJourneyIdx),
}

#[derive(Clone, Debug)]
pub enum RemovalError {
    UnknownDate(NaiveDate, VehicleJourneyIdx),
    UnknownVehicleJourney(VehicleJourneyIdx),
    DateInvalidForVehicleJourney(NaiveDate, VehicleJourneyIdx),
}

#[derive(Clone, Debug)]
pub enum ModifyError {
    UnknownDate(NaiveDate, VehicleJourneyIdx),
    UnknownVehicleJourney(VehicleJourneyIdx),
    DateInvalidForVehicleJourney(NaiveDate, VehicleJourneyIdx),
    Times(VehicleJourneyIdx, VehicleTimesError, Vec<NaiveDate>),
}

pub trait TimetablesIter<'a>: Types {
    type Positions: Iterator<Item = Self::Position>;
    fn positions(&'a self, mission: &Self::Mission) -> Self::Positions;

    type Trips: Iterator<Item = Self::Trip>;
    fn trips_of(
        &'a self,
        mission: &Self::Mission,
        real_time_level: &RealTimeLevel,
        days_patterns: &'a DaysPatterns,
    ) -> Self::Trips;

    type Missions: Iterator<Item = Self::Mission>;
    fn missions(&'a self) -> Self::Missions;
}
