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

use crate::models::{ModelRefs, StopPointIdx};
use regex::Regex;
use std::fmt::{Display, Formatter};
use transit_model::objects::Coord;

const N_DEG_TO_RAD: f64 = 0.017_453_292_38;
const EARTH_RADIUS_IN_METERS: f64 = 6_372_797.560856;

#[derive(Debug)]
pub enum BadPlacesNearby {
    InvalidEntryPoint(String),
    BadFormatCoord(String),
    UnavailableCoord(String),
}

impl Display for BadPlacesNearby {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Unable to parse {} as a coord. Expected format is (double:double)",
            self
        )
    }
}

impl std::error::Error for BadPlacesNearby {}

pub fn places_nearby_impl(model: &ModelRefs, uri: &str, radius: f64) -> Vec<(StopPointIdx, f64)> {
    let mut res: Vec<(StopPointIdx, f64)> = vec![];

    let entrypoint = parse_entrypoint(model, uri);
    if let Ok(coord) = entrypoint {
        let ep_coord = project_coord(coord);
        let bounding_box = bounding_box(coord, radius);
        // New stop_points do not have geo coord
        for sp in &model.base.stop_points {
            if within_box(&bounding_box, &sp.1.coord) {
                let sp_coord = project_coord(sp.1.coord);
                let distance = compute_distance(ep_coord, sp_coord);
                if distance < radius {
                    res.push((StopPointIdx::Base(sp.0), distance));
                }
            }
        }
    };
    res
}

fn parse_entrypoint(model: &ModelRefs, uri: &str) -> Result<Coord, BadPlacesNearby> {
    if let Some(coord_str) = uri.strip_prefix("coord:") {
        lazy_static! {
            static ref COORD_REGEX: Regex = Regex::new(
                r"^([-+]?[0-9]*\.?[0-9]*[eE]?[-+]?[0-9]*):([-+]?[0-9]*\.?[0-9]*[eE]?[-+]?[0-9]*)$",
            )
            .unwrap();
        }
        let cap = COORD_REGEX.captures(coord_str).unwrap();
        let lon = cap[1].parse::<f64>();
        let lat = cap[2].parse::<f64>();
        return match (lon, lat) {
            (Ok(lon), Ok(lat)) => Ok(Coord { lon, lat }),
            _ => Err(BadPlacesNearby::BadFormatCoord(uri.to_string())),
        };
    } else if let Some(stop_point_id) = uri.strip_prefix("stop_point:") {
        if let Some(stop_point) = model.base.stop_areas.get(stop_point_id) {
            return Ok(stop_point.coord);
        }
    } else if let Some(stop_area_id) = uri.strip_prefix("stop_area:") {
        if let Some(stop_area) = model.base.stop_areas.get(stop_area_id) {
            return Ok(stop_area.coord);
        }
    }

    Err(BadPlacesNearby::InvalidEntryPoint(uri.to_string()))
}

fn within_box(bbox: &(f64, f64, f64, f64), point: &Coord) -> bool {
    point.lat > bbox.0 && point.lat < bbox.1 && point.lon > bbox.2 && point.lon < bbox.3
}

fn compute_distance(p1: (f64, f64, f64), p2: (f64, f64, f64)) -> f64 {
    let (x1, y1, z1) = p1;
    let (x2, y2, z2) = p2;
    ((x1 - x2).powf(2.0) + (y1 - y2).powf(2.0) + (z1 - z2).powf(2.0)).sqrt()
}

fn project_coord(coord: Coord) -> (f64, f64, f64) {
    let lat_rad = coord.lat * N_DEG_TO_RAD;
    let lon_rad = coord.lon * N_DEG_TO_RAD;
    let x = EARTH_RADIUS_IN_METERS * lat_rad.cos() * lon_rad.sin();
    let y = EARTH_RADIUS_IN_METERS * lat_rad.cos() * lon_rad.cos();
    let z = EARTH_RADIUS_IN_METERS * lat_rad.sin();
    (x, y, z)
}

fn bounding_box(coord: Coord, radius: f64) -> (f64, f64, f64, f64) {
    let lat_rad = coord.lat * N_DEG_TO_RAD;
    let lon_rad = coord.lon * N_DEG_TO_RAD;
    // Radius of Earth at given latitude
    let earth_radius = wgs84earth_radius(lat_rad);
    // Radius of the parallel at given latitude
    let pearth_radius = earth_radius * lat_rad.cos();
    let lat_min = (lat_rad - radius / earth_radius) / N_DEG_TO_RAD;
    let lat_max = (lat_rad + radius / earth_radius) / N_DEG_TO_RAD;
    let lon_min = (lon_rad - radius / pearth_radius) / N_DEG_TO_RAD;
    let lon_max = (lon_rad + radius / pearth_radius) / N_DEG_TO_RAD;
    (lat_min, lat_max, lon_min, lon_max)
}

fn wgs84earth_radius(lat: f64) -> f64 {
    let an = EARTH_RADIUS_IN_METERS * EARTH_RADIUS_IN_METERS * lat.cos();
    let bn = EARTH_RADIUS_IN_METERS * EARTH_RADIUS_IN_METERS * lat.sin();
    let ad = EARTH_RADIUS_IN_METERS * lat.cos();
    let bd = EARTH_RADIUS_IN_METERS * lat.sin();
    ((an * an + bn * bn) / (ad * ad + bd * bd)).sqrt()
}
