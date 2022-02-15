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
pub mod chaos_disruption;
pub mod kirin_disruption;
pub mod time_periods;

#[derive(Debug, Clone)]
pub struct VehicleJourneyId {
    pub id: String,
}

#[derive(Ord, Eq, PartialEq, Debug, Clone, Copy)]
pub enum Effect {
    // DO NOT change the order of effects !!
    // Effects are ordered from the least to the worst impact
    StopMoved,
    UnknownEffect,
    OtherEffect,
    ModifiedService,
    AdditionalService,
    Detour,
    SignificantDelays,
    ReducedService,
    NoService,
}

impl Effect {
    fn level(&self) -> u8 {
        match self {
            Effect::StopMoved => 0,
            Effect::UnknownEffect => 1,
            Effect::OtherEffect => 2,
            Effect::ModifiedService => 3,
            Effect::AdditionalService => 4,
            Effect::Detour => 5,
            Effect::SignificantDelays => 6,
            Effect::ReducedService => 7,
            Effect::NoService => 8,
        }
    }
}

impl PartialOrd for Effect {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.level().partial_cmp(&other.level())
    }
}
