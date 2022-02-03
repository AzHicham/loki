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

pub mod apply_disruption;
pub mod base_model;
pub mod cancel_disruption;
pub mod model_refs;
pub mod real_time_disruption;
pub mod real_time_model;

pub use model_refs::ModelRefs;
pub use real_time_model::RealTimeModel;

use crate::{time::SecondsSinceTimezonedDayStart, timetables::FlowDirection};

use self::{
    base_model::{BaseStopPointIdx, BaseStopTimes, BaseTransferIdx, BaseVehicleJourneyIdx},
    real_time_model::{NewStopPointIdx, NewVehicleJourneyIdx, RealTimeStopTimes},
};

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum VehicleJourneyIdx {
    Base(BaseVehicleJourneyIdx),
    New(NewVehicleJourneyIdx),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum StopPointIdx {
    Base(BaseStopPointIdx),
    New(NewStopPointIdx),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum TransferIdx {
    Base(BaseTransferIdx),
    New(usize),
}

#[derive(Debug, Clone, Copy)]
pub struct StopTimeIdx {
    pub idx: usize,
}

#[derive(Debug, Clone)]
pub struct StopTime {
    pub stop: StopPointIdx,
    pub board_time: SecondsSinceTimezonedDayStart,
    pub debark_time: SecondsSinceTimezonedDayStart,
    pub flow_direction: FlowDirection,
}

#[derive(Clone)]
pub enum StopTimes<'a> {
    Base(BaseStopTimes<'a>),
    New(RealTimeStopTimes<'a>),
}

#[derive(Debug, Clone)]
pub struct Coord {
    pub lat: f64,
    pub lon: f64,
}

pub type Rgb = transit_model::objects::Rgb;

pub type Timezone = chrono_tz::Tz;

pub struct Contributor {
    pub id: String,
    pub name: String,
    pub license: Option<String>,
    pub url: Option<String>,
}

impl<'a> Iterator for StopTimes<'a> {
    type Item = StopTime;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            StopTimes::Base(inner) => inner.next(),
            StopTimes::New(inner) => inner.next(),
        }
    }
}
