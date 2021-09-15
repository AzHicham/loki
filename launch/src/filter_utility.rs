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

use crate::loki::Idx;
use loki::transit_model::{
    model::GetCorresponding,
    objects::{Line, Network, Route, StopArea, StopPoint, VehicleJourney},
    Model,
};
use relational_types::IdxSet;
use std::collections::HashSet;

#[derive(Debug)]
pub enum FilterType<'a> {
    StopPoint(&'a str),
    StopArea(&'a str),
    Line(&'a str),
    Route(&'a str),
    Network(&'a str),
    PhysicalMode(&'a str),
    CommercialMode(&'a str),
}

pub struct Filters {
    pub forbidden_sp_idx: HashSet<Idx<StopPoint>>,
    pub allowed_sp_idx: HashSet<Idx<StopPoint>>,
    pub forbidden_vj_idx: HashSet<Idx<VehicleJourney>>,
    pub allowed_vj_idx: HashSet<Idx<VehicleJourney>>,
}

pub fn parse_filter(input: &str) -> Option<FilterType> {
    if let Some(stop_point) = input.strip_prefix("stop_point:") {
        return Some(FilterType::StopPoint(stop_point));
    }
    if let Some(stop_area) = input.strip_prefix("stop_area:") {
        return Some(FilterType::StopArea(stop_area));
    }
    if let Some(line) = input.strip_prefix("line:") {
        return Some(FilterType::Line(line));
    }
    if let Some(route) = input.strip_prefix("route:") {
        return Some(FilterType::Route(route));
    }
    if let Some(network) = input.strip_prefix("network:") {
        return Some(FilterType::Network(network));
    }
    if let Some(physical_mode) = input.strip_prefix("physical_mode:") {
        return Some(FilterType::PhysicalMode(physical_mode));
    }
    if let Some(commercial_mode) = input.strip_prefix("commercial_mode:") {
        return Some(FilterType::CommercialMode(commercial_mode));
    }
    None
}

pub fn create_filter_idx(
    model: &Model,
    forbidden_uri: &[String],
    allowed_uri: &[String],
) -> Filters {
    let mut forbidden_sp_idx = HashSet::new();
    let mut allowed_sp_idx = HashSet::new();

    let mut forbidden_sa_idx: IdxSet<StopArea> = IdxSet::new();
    let mut allowed_sa_idx: IdxSet<StopArea> = IdxSet::new();

    // Handle Stop points and stop areas
    for s in forbidden_uri {
        let out = parse_filter(s.as_str());
        match out {
            Some(FilterType::StopPoint(sp)) => {
                let sp_idx = model.stop_points.get_idx(sp);
                if let Some(idx) = sp_idx {
                    forbidden_sp_idx.insert(idx);
                }
            }
            Some(FilterType::StopArea(sa_uri)) => {
                let opt_idx = model.stop_areas.get_idx(sa_uri);
                if let Some(idx) = opt_idx {
                    forbidden_sa_idx.insert(idx);
                }
            }
            _ => (),
        }
    }
    let sp_into_forbidden_sa: IdxSet<StopPoint> = forbidden_sa_idx.get_corresponding(model);
    forbidden_sp_idx.extend(sp_into_forbidden_sa);

    for s in allowed_uri {
        let out = parse_filter(s.as_str());
        match out {
            Some(FilterType::StopPoint(sp)) => {
                let sp_idx = model.stop_points.get_idx(sp);
                if let Some(idx) = sp_idx {
                    allowed_sp_idx.insert(idx);
                }
            }
            Some(FilterType::StopArea(sa_uri)) => {
                let opt_idx = model.stop_areas.get_idx(sa_uri);
                if let Some(idx) = opt_idx {
                    allowed_sa_idx.insert(idx);
                }
            }
            _ => (),
        }
    }

    let sp_into_allowed_sa: IdxSet<StopPoint> = allowed_sa_idx.get_corresponding(model);
    allowed_sp_idx.extend(sp_into_allowed_sa);

    //Handle VJ
    let mut forbidden_vj_idx = HashSet::new();
    let mut allowed_vj_idx = HashSet::new();

    let mut forbidden_line_idx: IdxSet<Line> = IdxSet::new();
    let mut allowed_line_idx: IdxSet<Line> = IdxSet::new();
    let mut forbidden_route_idx: IdxSet<Route> = IdxSet::new();
    let mut allowed_route_idx: IdxSet<Route> = IdxSet::new();
    let mut forbidden_network_idx: IdxSet<Network> = IdxSet::new();
    let mut allowed_network_idx: IdxSet<Network> = IdxSet::new();

    for s in forbidden_uri {
        let out = parse_filter(s.as_str());
        match out {
            Some(FilterType::Line(line)) => {
                let line_idx = model.lines.get_idx(line);
                if let Some(idx) = line_idx {
                    forbidden_line_idx.insert(idx);
                }
            }
            Some(FilterType::Route(route)) => {
                let route_idx = model.routes.get_idx(route);
                if let Some(idx) = route_idx {
                    forbidden_route_idx.insert(idx);
                }
            }
            Some(FilterType::Network(network)) => {
                let network_idx = model.networks.get_idx(network);
                if let Some(idx) = network_idx {
                    forbidden_network_idx.insert(idx);
                }
            }
            _ => (),
        }
    }

    for s in allowed_uri {
        let out = parse_filter(s.as_str());
        match out {
            Some(FilterType::Line(line)) => {
                let line_idx = model.lines.get_idx(line);
                if let Some(idx) = line_idx {
                    allowed_line_idx.insert(idx);
                }
            }
            Some(FilterType::Route(route)) => {
                let route_idx = model.routes.get_idx(route);
                if let Some(idx) = route_idx {
                    allowed_route_idx.insert(idx);
                }
            }
            Some(FilterType::Network(network)) => {
                let network_idx = model.networks.get_idx(network);
                if let Some(idx) = network_idx {
                    allowed_network_idx.insert(idx);
                }
            }
            _ => (),
        }
    }

    let vj_into_forbidden_line: IdxSet<VehicleJourney> =
        forbidden_line_idx.get_corresponding(model);
    forbidden_vj_idx.extend(vj_into_forbidden_line);
    let vj_into_allowed_line: IdxSet<VehicleJourney> = allowed_line_idx.get_corresponding(model);
    allowed_vj_idx.extend(vj_into_allowed_line);

    let vj_into_forbidden_route: IdxSet<VehicleJourney> =
        forbidden_route_idx.get_corresponding(model);
    forbidden_vj_idx.extend(vj_into_forbidden_route);
    let vj_into_allowed_route: IdxSet<VehicleJourney> = allowed_route_idx.get_corresponding(model);
    allowed_vj_idx.extend(vj_into_allowed_route);

    let vj_into_forbidden_network: IdxSet<VehicleJourney> =
        forbidden_network_idx.get_corresponding(model);
    forbidden_vj_idx.extend(vj_into_forbidden_network);
    let vj_into_allowed_network: IdxSet<VehicleJourney> =
        allowed_network_idx.get_corresponding(model);
    allowed_vj_idx.extend(vj_into_allowed_network);

    Filters {
        forbidden_sp_idx,
        allowed_sp_idx,
        forbidden_vj_idx,
        allowed_vj_idx,
    }
}
