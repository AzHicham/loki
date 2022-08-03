// Copyright  (C) 2022, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Hove (www.kisio.com).
// Help us simplify mobility and open public transport:
// a non ending quest to the responsive locomotion way of traveling!
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

use std::fmt::Display;

use tracing::debug;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Regularity {
    Rare,
    Intermittent,
    Frequent,
}

impl Regularity {
    pub fn new(physical_mode_name: &str) -> Self {
        match physical_mode_name {
            "Bus" | "Funicular" | "Coach" | "Train" | "LongDistanceTrain" | "Air" | "Boat"
            | "Ferry" | "SuspendedCableCar" => Regularity::Rare,
            "LocalTrain" | "RapidTransit" | "Tramway" | "RailShuttle" | "BusRapidTransit"
            | "Shuttle" => Regularity::Intermittent,
            "Metro" => Regularity::Frequent,
            // unknown physical mode, let's default to Rare
            _ => {
                debug!("unknown physical mode {}", physical_mode_name);
                Regularity::Rare
            }
        }
    }
}

impl Display for Regularity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Regularity::Frequent => write!(f, "frequent"),
            Regularity::Intermittent => write!(f, "intermittent"),
            Regularity::Rare => write!(f, "rare"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Uncertainty {
    level: u8,
    last_vehicle_regularity: Option<Regularity>,
}

impl Uncertainty {
    pub fn zero() -> Self {
        Self {
            level: 0,
            last_vehicle_regularity: None,
        }
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    pub fn extend(&self, next_vehicle_regularity: Regularity) -> Self {
        use Regularity::{Frequent, Intermittent, Rare};
        let delta = match (self.last_vehicle_regularity, next_vehicle_regularity) {
            (_, Frequent) => 1,

            (None | Some(Frequent), Intermittent) => 2,

            (None | Some(Frequent), Rare) => 3,

            (Some(Rare | Intermittent), Intermittent) => 5,

            (Some(Rare | Intermittent), Rare) => 10,
        };
        let level = self.level.saturating_add(delta);
        Self {
            level,
            last_vehicle_regularity: Some(next_vehicle_regularity),
        }
    }
}

impl Ord for Uncertainty {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.level.cmp(&other.level)
    }
}

impl PartialOrd for Uncertainty {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for Uncertainty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.level)
    }
}
