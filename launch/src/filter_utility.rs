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

use crate::loki::transit_data_filtered::DataFilter;
use itertools::Itertools;
use loki::transit_model::{
    objects::{StopPoint, VehicleJourney},
    Model,
};

#[derive(PartialEq, Eq)]
pub enum FilterPtType<'a> {
    StopPoint(&'a str),
    StopArea(&'a str),
    Line(&'a str),
    Route(&'a str),
    Network(&'a str),
    PhysicalMode(&'a str),
    CommercialMode(&'a str),
}

pub fn create_filter<T>(model: &Model, forbidden_uri: &[T], allowed_uri: &[T]) -> DataFilter
where
    T: AsRef<str> + std::cmp::Eq + std::hash::Hash,
{
    // Pre processing
    let allowed_vj_filter: Vec<_> = allowed_uri
        .iter()
        .unique()
        .filter_map(|uri| parse_filter_vj(model, uri))
        .collect();
    let forbidden_vj_filter: Vec<_> = forbidden_uri
        .iter()
        .unique()
        .filter_map(|uri| parse_filter_vj(model, uri))
        .collect();

    let allowed_sp_filter: Vec<_> = allowed_uri
        .iter()
        .unique()
        .filter_map(|uri| parse_filter_sp(model, uri))
        .collect();
    let forbidden_sp_filter: Vec<_> = forbidden_uri
        .iter()
        .unique()
        .filter_map(|uri| parse_filter_sp(model, uri))
        .collect();

    let empty_filters = allowed_vj_filter.is_empty()
        && forbidden_vj_filter.is_empty()
        && allowed_sp_filter.is_empty()
        && forbidden_sp_filter.is_empty();

    if !empty_filters {
        let mut filter_vj = vec![allowed_vj_filter.is_empty(); model.vehicle_journeys.len()];
        let mut filter_sp = vec![allowed_sp_filter.is_empty(); model.stop_points.len()];

        if !(allowed_vj_filter.is_empty() && forbidden_vj_filter.is_empty()) {
            model
                .vehicle_journeys
                .iter()
                .enumerate()
                .for_each(|(idx, (_, vj))| {
                    filter_vj[idx] = (filter_vj[idx]
                        | parse_filter_list_vj(model, vj, &allowed_vj_filter))
                        & !parse_filter_list_vj(model, vj, &forbidden_vj_filter);
                });
        }

        if !(allowed_sp_filter.is_empty() && forbidden_sp_filter.is_empty()) {
            model
                .stop_points
                .iter()
                .enumerate()
                .for_each(|(idx, (_, sp))| {
                    filter_sp[idx] = (filter_sp[idx]
                        | parse_filter_list_sp(model, sp, &allowed_sp_filter))
                        & !parse_filter_list_sp(model, sp, &forbidden_sp_filter);
                });
        }

        DataFilter::new(filter_sp, filter_vj, false)
    } else {
        DataFilter::default()
    }
}

fn parse_filter_vj<'filter, T: AsRef<str>>(
    model: &Model,
    input: &'filter T,
) -> Option<FilterPtType<'filter>> {
    if let Some(line) = input.as_ref().strip_prefix("line:") {
        return if let Some(_) = model.lines.get(line) {
            Some(FilterPtType::Line(line))
        } else {
            None
        };
    }
    if let Some(route) = input.as_ref().strip_prefix("route:") {
        return if let Some(_) = model.routes.get(route) {
            Some(FilterPtType::Route(route))
        } else {
            None
        };
    }
    if let Some(network) = input.as_ref().strip_prefix("network:") {
        return if let Some(_) = model.networks.get(network) {
            Some(FilterPtType::Network(network))
        } else {
            None
        };
    }
    if let Some(physical_mode) = input.as_ref().strip_prefix("physical_mode:") {
        return if let Some(_) = model.physical_modes.get(physical_mode) {
            Some(FilterPtType::PhysicalMode(physical_mode))
        } else {
            None
        };
    }
    if let Some(commercial_mode) = input.as_ref().strip_prefix("commercial_mode:") {
        return if let Some(_) = model.commercial_modes.get(commercial_mode) {
            Some(FilterPtType::CommercialMode(commercial_mode))
        } else {
            None
        };
    }
    None
}

fn parse_filter_list_vj(model: &Model, vj: &VehicleJourney, filters: &[FilterPtType]) -> bool {
    for filter in filters.iter() {
        match filter {
            FilterPtType::CommercialMode(str) => {
                if let Some(route) = model.routes.get(&vj.route_id) {
                    if let Some(line) = model.lines.get(&route.line_id) {
                        if &line.commercial_mode_id == str {
                            return true;
                        }
                    }
                }
            }
            FilterPtType::PhysicalMode(str) => {
                if &vj.physical_mode_id == str {
                    return true;
                }
            }
            FilterPtType::Route(str) => {
                if &vj.route_id == str {
                    return true;
                }
            }
            FilterPtType::Line(str) => {
                if let Some(route) = model.routes.get(&vj.route_id) {
                    if &route.line_id == str {
                        return true;
                    }
                }
            }
            FilterPtType::Network(str) => {
                if let Some(route) = model.routes.get(&vj.route_id) {
                    if let Some(line) = model.lines.get(&route.line_id) {
                        if &line.network_id == str {
                            return true;
                        }
                    }
                }
            }
            _ => (),
        }
    }
    false
}

fn parse_filter_sp<'filter, T: AsRef<str>>(
    model: &Model,
    input: &'filter T,
) -> Option<FilterPtType<'filter>> {
    if let Some(stop_point) = input.as_ref().strip_prefix("stop_point:") {
        return if let Some(_) = model.stop_points.get(stop_point) {
            Some(FilterPtType::StopPoint(stop_point))
        } else {
            None
        };
    }
    if let Some(stop_area) = input.as_ref().strip_prefix("stop_area:") {
        return if let Some(_) = model.stop_areas.get(stop_area) {
            Some(FilterPtType::StopArea(stop_area))
        } else {
            None
        };
    }
    None
}

fn parse_filter_list_sp(model: &Model, sp: &StopPoint, filters: &[FilterPtType]) -> bool {
    for filter in filters.iter() {
        match filter {
            FilterPtType::StopPoint(str) => {
                if &sp.id == str {
                    return true;
                }
            }
            FilterPtType::StopArea(str) => {
                if let Some(sa) = model.stop_areas.get(&sp.stop_area_id) {
                    if &sa.id == str {
                        return true;
                    }
                }
            }
            _ => (),
        }
    }
    false
}
