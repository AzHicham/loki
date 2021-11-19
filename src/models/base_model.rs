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

use std::ops::Deref;

use typed_index_collection::Idx;

pub struct BaseModel {
    collections: transit_model::model::Collections,
}

pub type BaseVehicleJourneyIdx = Idx<transit_model::objects::VehicleJourney>;
pub type BaseStopPointIdx = Idx<transit_model::objects::StopPoint>;
pub type BaseTransferIdx = Idx<transit_model::objects::Transfer>;

pub type BaseStopTime = transit_model::objects::StopTime;

impl Deref for BaseModel {
    type Target = transit_model::model::Collections;

    fn deref(&self) -> &Self::Target {
        &self.collections
    }
}

impl BaseModel {
    pub fn from_transit_model(model: transit_model::Model) -> Self {
        Self {
            collections: model.into_collections(),
        }
    }

    pub fn new(collections: transit_model::model::Collections) -> Self {
        Self { collections }
    }

    pub fn stop_point_idx(&self, stop_id: &str) -> Option<BaseStopPointIdx> {
        self.collections.stop_points.get_idx(stop_id)
    }
}