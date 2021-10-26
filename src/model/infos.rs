use super::{ModelRefs, StopPointIdx};

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

pub struct Coord {
    pub lat : f64,
    pub lon : f64,
}



impl<'model> ModelRefs<'model> {

    pub fn stop_point_uri(&self,  stop_point_idx : & StopPointIdx) -> String {
        format!("stop_point:{}", self.stop_point_name(stop_point_idx))
    }

    pub fn house_numer(&self, stop_point_idx : & StopPointIdx) -> Option<&'model str> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => {
                let stop_point = &self.base.stop_points[*idx];
                let has_address_id = &stop_point.address_id;
                if let Some(address_id) = has_address_id {
                    let address = &self.base.addresses.get(&address_id)?;
                    Some(address.street_name.as_str())
                }
                else {
                    None
                }

            },
            StopPointIdx::New(_) => None,
        }
    }

    pub fn street_name(&self, stop_point_idx : & StopPointIdx) -> Option<&'model str> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => {
                let stop_point = &self.base.stop_points[*idx];
                let has_address_id = &stop_point.address_id;
                if let Some(address_id) = has_address_id {
                    let address = &self.base.addresses.get(&address_id)?;
                    Some(address.street_name.as_str())
                }
                else {
                    None
                }
            },
            StopPointIdx::New(_) => None,
        }
    }


    pub fn coord(&self, stop_point_idx : & StopPointIdx) -> Option<Coord> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => {
                let stop_point = &self.base.stop_points[*idx];
                let coord = Coord {
                    lat : stop_point.coord.lat,
                    lon : stop_point.coord.lon,
                };
                Some(coord)


            },
            StopPointIdx::New(_) => None,
        }
    }

    pub fn platform_code(&self, stop_point_idx : & StopPointIdx) -> Option<&str> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => {
                let stop_point = &self.base.stop_points[*idx];
                let has_platform_code = &stop_point.platform_code;
                has_platform_code.as_ref()
                    .map(|s| s.as_str())


            },
            StopPointIdx::New(_) => None,
        }
    }

    pub fn fare_zone_id(&self, stop_point_idx : & StopPointIdx) -> Option<&str> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => {
                let stop_point = &self.base.stop_points[*idx];
                stop_point.fare_zone_id.as_ref()
                    .map(|s| s.as_str())
            },
            StopPointIdx::New(_) => None,
        }
    } 

    pub fn stop_area(&self, stop_area_id : &str) -> Option<&transit_model::objects::StopArea> {
        self.base.stop_areas.get(stop_area_id)
    }

    pub fn codes(&self, stop_point_idx : & StopPointIdx) -> Option<impl Iterator<Item = & (String, String)> + '_> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => {
                let stop_point = &self.base.stop_points[*idx];
                Some(stop_point.codes.iter())
            },
            StopPointIdx::New(_) => {
                None
            },
        }
    }


}