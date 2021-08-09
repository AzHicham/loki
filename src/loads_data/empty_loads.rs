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

use chrono::NaiveDate;
use log::{debug, trace};
use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;
use transit_model::objects::VehicleJourney;
use transit_model::Model;
use typed_index_collection::Idx;

type StopSequence = u32;
type Occupancy = u8;
type VehicleJourneyIdx = Idx<VehicleJourney>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Load {}

impl Default for Load {
    fn default() -> Self {
        ()
    }
}

use std::cmp::Ordering;



impl Ord for Load {
    fn cmp(&self, other: &Self) -> Ordering {
        true
    }
}

impl PartialOrd for Load {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LoadsCount();

impl LoadsCount {
    pub fn zero() -> Self {
        Self { }
    }

    pub fn add(&self, load: Load) -> Self {
        Self {  }
    }

    pub fn max(&self) -> Load {
        Load::default()
    }

    pub fn is_lower(&self, other: &Self) -> bool {
        true
    }
}

impl Default for LoadsCount {
    fn default() -> Self {
        Self::zero()
    }
}

fn occupancy_to_load(occupancy: Occupancy) -> Load {
    debug_assert!(occupancy <= 100);
    Load::default()
}

pub struct LoadsData();


impl LoadsData {
    pub fn loads(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &NaiveDate,
    ) -> Option<&[Load]> {
        None
    }

    pub fn empty() -> Self {
        LoadsData { }
    }

    pub fn new<P: AsRef<Path>>(
        csv_occupancys_filepath: P,
        model: &Model,
    ) -> Result<Self, Box<dyn Error>> {
        
        Ok(LoadsData::empty())
    }

}