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

use transit_model::objects::PhysicalMode;

pub enum Regularity {
    Rare,
    Intermittent,
    Frequent,
}

impl Regularity {
    pub fn new(physical_mode: &PhysicalMode) -> Option<Self> {
        match physical_mode.id.as_str() {
            "physical_mode:Bus" | "physical_mode:Funicular" => Some(Regularity::Rare),

            "physical_mode:LocalTrain" | "physical_mode:RapidTransit" | "physical_mode:Tramway" => {
                Some(Regularity::Intermittent)
            }
            "physical_mode:Metro" => Some(Regularity::Frequent),
            _ => None,
        }
    }
}
