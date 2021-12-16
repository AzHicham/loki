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

mod utility;

use crate::models::{ModelRefs, StopPointIdx};
use transit_model::objects::StopPoint;
use typed_index_collection::Iter;

use utility::BadPlacesNearby;
use utility::{bounding_box, compute_distance, parse_entrypoint, project_coord, within_box};

pub fn places_nearby_impl<'model, 'uri>(
    model: &'model ModelRefs,
    uri: &'uri str,
    radius: f64,
) -> Result<PlacesNearbyResult<'model>, BadPlacesNearby> {
    // ep: entrypoint
    let ep_coord = parse_entrypoint(model, uri)?;
    let ep_coord_xyz = project_coord(ep_coord);
    let bounding_box = bounding_box(ep_coord, radius);

    Ok(PlacesNearbyResult::new(
        model,
        ep_coord_xyz,
        radius,
        bounding_box,
    ))
}

pub struct PlacesNearbyResult<'model> {
    ep_coord_xyz: (f64, f64, f64),
    radius: f64,
    bounding_box: (f64, f64, f64, f64),
    inner: Iter<'model, StopPoint>,
}

impl<'model> PlacesNearbyResult<'model> {
    pub fn new(
        model: &'model ModelRefs,
        ep_coord_xyz: (f64, f64, f64),
        radius: f64,
        bounding_box: (f64, f64, f64, f64),
    ) -> Self {
        Self {
            ep_coord_xyz,
            radius,
            bounding_box,
            inner: model.base.stop_points.iter(),
        }
    }
}

impl<'model> Iterator for PlacesNearbyResult<'model> {
    type Item = (StopPointIdx, f64);

    fn next(&mut self) -> Option<Self::Item> {
        for sp in self.inner.by_ref() {
            if within_box(&self.bounding_box, &sp.1.coord) {
                let sp_coord_xyz = project_coord(sp.1.coord);
                let distance = compute_distance(&self.ep_coord_xyz, &sp_coord_xyz);
                if distance < self.radius {
                    return Some((StopPointIdx::Base(sp.0), distance));
                }
            }
        }
        None
    }
}
