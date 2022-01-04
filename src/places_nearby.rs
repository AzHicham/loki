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

extern crate static_assertions;

use super::geometry::{bounding_box, distance_coord_to_coord};
use crate::models::{ModelRefs, StopPointIdx, base_model::BaseStopPoints, Coord};
use regex::Regex;
use std::fmt::{Display, Formatter};


pub fn places_nearby_impl<'model>(
    models: & ModelRefs<'model>,
    uri: & str,
    radius: f64,
) -> Result<PlacesNearbyIter<'model>, BadPlacesNearby> {
    let center = parse_entrypoint(models, uri)?;
    let bounding_box = bounding_box(&center, radius);

    Ok(PlacesNearbyIter::new(models.clone(), center, radius, bounding_box))
}

pub struct PlacesNearbyIter<'model> {
    center: Coord,
    radius: f64,
    bounding_box: (f64, f64, f64, f64),
    inner: BaseStopPoints<'model>,
    models : ModelRefs<'model>

}

impl<'model> PlacesNearbyIter<'model> {
    pub fn new(
        models: ModelRefs<'model>,
        center: Coord,
        radius: f64,
        bounding_box: (f64, f64, f64, f64),
    ) -> Self {
        Self {
            center,
            radius,
            bounding_box,
            inner: models.base.stop_points(),
            models

        }
    }
}

impl<'model> Iterator for PlacesNearbyIter<'model> {
    type Item = (StopPointIdx, f64);

    fn next(&mut self) -> Option<Self::Item> {
        for stop_point_idx in self.inner.by_ref() {
            let coord = self.models.base.coord(stop_point_idx);
            // in order to avoid the '"expensive" calculation of  distance_coord_to_coord()
            // we first make the "cheap" check that the stop_point is within a bounding box that contains
            // all points within the requested radius
            if within_box(&self.bounding_box, &coord) {
                let distance = distance_coord_to_coord(&self.center, &coord);
                if distance < self.radius {
                    return Some((StopPointIdx::Base(stop_point_idx), distance));
                }
            }
        }
        None
    }
}

#[derive(Debug)]
pub enum BadPlacesNearby {
    InvalidEntryPoint(String),
    InvalidPtObject(String),
    InvalidFormatCoord(String),
    InvalidRangeCoord(String),
}

impl Display for BadPlacesNearby {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BadPlacesNearby::InvalidEntryPoint(uri) => {
                write!(f, "Unable to parse entrypoint : {}", uri)
            }
            BadPlacesNearby::InvalidPtObject(uri) => {
                write!(f, "Invalid/Unknown pt_object : {}", uri)
            }
            BadPlacesNearby::InvalidFormatCoord(uri) => {
                write!(
                    f,
                    "Unable to parse {} as a coord. Expected format is (double:double)",
                    uri
                )
            }
            BadPlacesNearby::InvalidRangeCoord(uri) => {
                write!(
                    f,
                    "Invalid coord : {}. Coordinates must be between [-90;90] for latitude and [-180;180] for longitude",
                    uri
                )
            }
        }
    }
}

impl std::error::Error for BadPlacesNearby {}

fn parse_entrypoint(model: &ModelRefs, uri: &str) -> Result<Coord, BadPlacesNearby> {
    return if let Some(stop_point_id) = uri.strip_prefix("stop_point:") {
        if let Some(stop_point_idx) = model.base.stop_point_idx(stop_point_id) {
            let coord = model.base.coord(stop_point_idx);
            Ok(coord)
        } else {
            Err(BadPlacesNearby::InvalidPtObject(uri.to_string()))
        }
    } else if let Some(stop_area_id) = uri.strip_prefix("stop_area:") {
        if let Some(coord) = model.base.stop_area_coord(stop_area_id) {
            Ok(coord)
        } else {
            Err(BadPlacesNearby::InvalidPtObject(uri.to_string()))
        }
    } else if let Some(coord_str) = uri.strip_prefix("coord:") {
        lazy_static! {
            static ref COORD_REGEX: Regex =
                Regex::new(r"^([-+]?[0-9]*\.?[0-9]*):([-+]?[0-9]*\.?[0-9]*)$",).unwrap();
        }
        if let Some(cap) = COORD_REGEX.captures(coord_str) {
            let lon = cap[1].parse::<f64>();
            let lat = cap[2].parse::<f64>();
            match (lon, lat) {
                (Ok(lon), Ok(lat)) => {
                    if (-180.0..=180.0).contains(&lon) && (-90.0..=90.0).contains(&lat) {
                        Ok(Coord { lon, lat })
                    } else {
                        Err(BadPlacesNearby::InvalidRangeCoord(uri.to_string()))
                    }
                }
                _ => Err(BadPlacesNearby::InvalidFormatCoord(uri.to_string())),
            }
        } else {
            Err(BadPlacesNearby::InvalidFormatCoord(uri.to_string()))
        }
    } else {
        Err(BadPlacesNearby::InvalidEntryPoint(uri.to_string()))
    };
}

fn within_box(bbox: &(f64, f64, f64, f64), point: &Coord) -> bool {
    point.lat > bbox.0 && point.lat < bbox.1 && point.lon > bbox.2 && point.lon < bbox.3
}
