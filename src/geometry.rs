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

use transit_model::objects::Coord;

const N_DEG_TO_RAD: f64 = 0.017_453_292_38;
const EARTH_RADIUS_IN_METERS: f64 = 6_372_797.560856;

pub fn distance_coord_to_coord(from: &Coord, to: &Coord) -> f64 {
    let longitude_arc = (from.lon - to.lon) * N_DEG_TO_RAD;
    let latitude_arc = (from.lat - to.lat) * N_DEG_TO_RAD;
    let latitude_h = (latitude_arc * 0.5).sin();
    let latitude_h = latitude_h * latitude_h;
    let longitude_h = (longitude_arc * 0.5).sin();
    let longitude_h = longitude_h * longitude_h;
    let tmp = (from.lat * N_DEG_TO_RAD).cos() * (to.lat * N_DEG_TO_RAD).cos();
    EARTH_RADIUS_IN_METERS * 2.0 * (latitude_h + tmp * longitude_h).sqrt().asin()
}

pub fn bounding_box(coord: Coord, radius: f64) -> (f64, f64, f64, f64) {
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
