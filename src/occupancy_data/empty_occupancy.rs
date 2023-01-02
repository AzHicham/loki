// Copyright  (C) 2020, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Hove (www.kisio.com).
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

use chrono::NaiveDate;
use std::{error::Error, fmt::Display, io};
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Occupancy {
    Unknown,
}

impl Display for Occupancy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Occupancy ()")
    }
}

impl Default for Occupancy {
    fn default() -> Self {
        Occupancy::Unknown
    }
}

use std::cmp::Ordering;

use crate::models::{base_model, StopTimeIdx, VehicleJourneyIdx};

impl Ord for Occupancy {
    fn cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl PartialOrd for Occupancy {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct OccupanciesCount();

impl Display for OccupanciesCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OccupanciesCount ()")
    }
}

impl OccupanciesCount {
    pub fn zero() -> Self {
        Self {}
    }

    pub fn add(&self, _occupancy: Occupancy) -> Self {
        Self {}
    }

    pub fn max(&self) -> Occupancy {
        Occupancy::default()
    }

    pub fn is_lower(&self, _other: &Self) -> bool {
        true
    }
}

impl Default for OccupanciesCount {
    fn default() -> Self {
        Self::zero()
    }
}

pub struct OccupancyData();

impl OccupancyData {
    pub fn occupancies(
        &self,
        _vehicle_journey_idx: &VehicleJourneyIdx,
        _date: &NaiveDate,
    ) -> Option<&[Occupancy]> {
        None
    }

    pub fn occupancy(
        &self,
        _vehicle_journey_idx: &VehicleJourneyIdx,
        _stop_time_idx: StopTimeIdx,
        _date: &NaiveDate,
    ) -> Option<Occupancy> {
        None
    }

    pub fn empty() -> Self {
        OccupancyData {}
    }

    pub fn try_from_reader<R: io::Read>(
        _csv_occupancy_reader: R,
        _model: &base_model::Model,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(OccupancyData::empty())
    }
}
