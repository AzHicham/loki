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

use loki::{tracing::warn, transit_model::Model, Idx, StopPoint, VehicleJourney};

pub enum StopFilter<'a> {
    StopPoint(&'a str),
    StopArea(&'a str),
}

pub enum VehicleFilter<'a> {
    Line(&'a str),
    Route(&'a str),
    Network(&'a str),
    PhysicalMode(&'a str),
    CommercialMode(&'a str),
}

impl<'a> VehicleFilter<'a> {
    pub fn applies_on(&self, idx: &Idx<VehicleJourney>, model: &Model) -> bool {
        let vj = &model.vehicle_journeys[*idx];
        match self {
            VehicleFilter::Line(line_id) => {
                let has_route = model.routes.get(&vj.route_id);
                if let Some(route) = has_route {
                    route.line_id.as_str() == *line_id
                } else {
                    warn!("Applying a filter on an invalid vehicle_journey idx {:?}. Its route id {} is unknown.", idx, vj.route_id);
                    false
                }
            }
            VehicleFilter::Route(route_id) => vj.route_id.as_str() == *route_id,
            VehicleFilter::Network(network_id) => {
                let has_route = model.routes.get(&vj.route_id);
                if let Some(route) = has_route {
                    let has_line = model.lines.get(&route.line_id);
                    if let Some(line) = has_line {
                        line.network_id.as_str() == *network_id
                    } else {
                        warn!("Applying a filter on an invalid vehicle_journey idx {:?}. Its line id {} is unknown.", idx, route.line_id);
                        false
                    }
                } else {
                    warn!("Applying a filter on an invalid vehicle_journey idx {:?}. Its route id {} is unknown.", idx, vj.route_id);
                    false
                }
            }
            VehicleFilter::PhysicalMode(physical_mode_id) => {
                vj.physical_mode_id.as_str() == *physical_mode_id
            }
            VehicleFilter::CommercialMode(commercial_mode_id) => {
                let has_route = model.routes.get(&vj.route_id);
                if let Some(route) = has_route {
                    let has_line = model.lines.get(&route.line_id);
                    if let Some(line) = has_line {
                        line.commercial_mode_id.as_str() == *commercial_mode_id
                    } else {
                        warn!("Applying a filter on an invalid vehicle_journey idx {:?}. Its line id {} is unknown.", idx, route.line_id);
                        false
                    }
                } else {
                    warn!("Applying a filter on an invalid vehicle_journey idx {:?}. Its route id {} is unknown.", idx, vj.route_id);
                    false
                }
            }
        }
    }
}

impl<'a> StopFilter<'a> {
    pub fn applies_on(&self, idx: &Idx<StopPoint>, model: &Model) -> bool {
        let stop_point = &model.stop_points[*idx];
        match self {
            StopFilter::StopPoint(stop_point_id) => stop_point.id.as_str() == *stop_point_id,
            StopFilter::StopArea(stop_area_id) => {
                let has_stop_area = model.stop_areas.get(&stop_point.stop_area_id);
                if let Some(stop_area) = has_stop_area {
                    stop_area.id.as_str() == *stop_area_id
                } else {
                    warn!("Applying a filter on an invalid stop_point idx {:?}. Its stop_area id {} is unknown.", idx, stop_point.stop_area_id);
                    false
                }
            }
        }
    }
}

enum Filter<'a> {
    Stop(StopFilter<'a>),
    Vehicle(VehicleFilter<'a>),
}

pub struct Filters<'a> {
    allowed_vehicles: Vec<VehicleFilter<'a>>,
    forbidden_vehicles: Vec<VehicleFilter<'a>>,
    allowed_stops: Vec<StopFilter<'a>>,
    forbidden_stops: Vec<StopFilter<'a>>,
}

impl<'a> Filters<'a> {
    pub fn is_vehicle_journey_valid(&self, idx: &Idx<VehicleJourney>, model: &Model) -> bool {
        // if *one* forbidden filter applies, then the vehicle_journey is invalid
        for forbid_filter in self.forbidden_vehicles.iter() {
            if forbid_filter.applies_on(idx, model) {
                return false;
            }
        }
        // if there is no allowed_filter, then the vehicle_journey is valid
        if self.allowed_vehicles.is_empty() {
            return true;
        }

        // if *one* allowed_filter applies, then the vehicle_journey is valid
        for allowed_filter in self.allowed_vehicles.iter() {
            if allowed_filter.applies_on(idx, model) {
                return true;
            }
        }

        // there is some allowed filters, but none of them applies, so the vehicle_journey is invalid
        false
    }
    pub fn is_stop_point_valid(&self, idx: &Idx<StopPoint>, model: &Model) -> bool {
        // if *one* forbidden filter applies, then the idx is invalid
        for forbid_filter in self.forbidden_stops.iter() {
            if forbid_filter.applies_on(idx, model) {
                return false;
            }
        }
        // if there is no allowed_filter, then the idx is valid
        if self.allowed_stops.is_empty() {
            return true;
        }

        // if *one* allowed_filter applies, then the idx is valid
        for allowed_filter in self.allowed_stops.iter() {
            if allowed_filter.applies_on(idx, model) {
                return true;
            }
        }

        // there is some allowed filters, but none of them applies, so the idx is invalid
        false
    }

    pub fn new<T>(
        model: &Model,
        forbidden_uri: &'a [T],
        allowed_uri: &'a [T],
    ) -> Option<Filters<'a>>
    where
        T: AsRef<str>,
    {
        let (allowed_vehicle_filters, allowed_stop_filters) = {
            let mut allowed_vehicle_filters = Vec::new();
            let mut allowed_stop_filters = Vec::new();
            for filter_str in allowed_uri {
                let filter_result = parse_filter(model, filter_str.as_ref(), "allowed_id[]");
                if let Ok(filter) = filter_result {
                    match filter {
                        Filter::Stop(stop_filter) => allowed_stop_filters.push(stop_filter),
                        Filter::Vehicle(vehicle_filter) => {
                            allowed_vehicle_filters.push(vehicle_filter)
                        }
                    }
                }
            }
            (allowed_vehicle_filters, allowed_stop_filters)
        };

        let (forbiddden_vehicle_filters, forbidden_stop_filters) = {
            let mut forbiddden_vehicle_filters = Vec::new();
            let mut forbidden_stop_filters = Vec::new();
            for filter_str in forbidden_uri {
                let filter_result = parse_filter(model, filter_str.as_ref(), "forbidden_id[]");
                if let Ok(filter) = filter_result {
                    match filter {
                        Filter::Stop(stop_filter) => forbidden_stop_filters.push(stop_filter),
                        Filter::Vehicle(vehicle_filter) => {
                            forbiddden_vehicle_filters.push(vehicle_filter)
                        }
                    }
                }
            }
            (forbiddden_vehicle_filters, forbidden_stop_filters)
        };

        let has_no_filter = allowed_stop_filters.is_empty()
            && allowed_vehicle_filters.is_empty()
            && forbidden_stop_filters.is_empty()
            && forbiddden_vehicle_filters.is_empty();

        if has_no_filter {
            None
        } else {
            let result = Filters {
                allowed_stops: allowed_stop_filters,
                forbidden_stops: forbidden_stop_filters,
                allowed_vehicles: allowed_vehicle_filters,
                forbidden_vehicles: forbiddden_vehicle_filters,
            };
            Some(result)
        }
    }
}

fn parse_filter<'a>(
    model: &Model,
    filter_str: &'a str,
    filter_provenance: &str,
) -> Result<Filter<'a>, ()> {
    if let Some(line_id) = filter_str.strip_prefix("line:") {
        if model.lines.contains_id(line_id) {
            let filter = Filter::Vehicle(VehicleFilter::Line(line_id));
            return Ok(filter);
        } else {
            warn!(
                "Unknown line id {} in {} filter {}. I'll ignore it.",
                line_id, filter_provenance, filter_str
            );
            return Err(());
        }
    }
    if let Some(route_id) = filter_str.strip_prefix("route:") {
        if model.routes.contains_id(route_id) {
            let filter = Filter::Vehicle(VehicleFilter::Route(route_id));
            return Ok(filter);
        } else {
            warn!(
                "Unknown route id {} in {} filter {}. I'll ignore it.",
                route_id, filter_provenance, filter_str
            );
            return Err(());
        }
    }
    if let Some(network_id) = filter_str.strip_prefix("network:") {
        if model.networks.contains_id(network_id) {
            let filter = Filter::Vehicle(VehicleFilter::Network(network_id));
            return Ok(filter);
        } else {
            warn!(
                "Unknown network id {} in {} filter {}. I'll ignore it.",
                network_id, filter_provenance, filter_str
            );
            return Err(());
        }
    }

    if let Some(physical_mode_id) = filter_str.strip_prefix("physical_mode:") {
        if model.physical_modes.contains_id(physical_mode_id) {
            let filter = Filter::Vehicle(VehicleFilter::PhysicalMode(physical_mode_id));
            return Ok(filter);
        } else {
            warn!(
                "Unknown physical_mode id {} in {} filter {}. I'll ignore it.",
                physical_mode_id, filter_provenance, filter_str
            );
            return Err(());
        }
    }

    if let Some(commercial_model_id) = filter_str.strip_prefix("commercial_mode:") {
        if model.commercial_modes.contains_id(commercial_model_id) {
            let filter = Filter::Vehicle(VehicleFilter::CommercialMode(commercial_model_id));
            return Ok(filter);
        } else {
            warn!(
                "Unknown commercial_mode id {} in {} filter {}. I'll ignore it.",
                commercial_model_id, filter_provenance, filter_str
            );
            return Err(());
        }
    }

    if let Some(stop_point_id) = filter_str.strip_prefix("stop_point:") {
        if model.stop_points.contains_id(stop_point_id) {
            let filter = Filter::Stop(StopFilter::StopPoint(stop_point_id));
            return Ok(filter);
        } else {
            warn!(
                "Unknown stop_point id {} in {} filter {}. I'll ignore it.",
                stop_point_id, filter_provenance, filter_str
            );
            return Err(());
        }
    }

    if let Some(stop_area_id) = filter_str.strip_prefix("stop_area:") {
        if model.stop_areas.contains_id(stop_area_id) {
            let filter = Filter::Stop(StopFilter::StopArea(stop_area_id));
            return Ok(filter);
        } else {
            warn!(
                "Unknown stop_area id {} in {} filter {}. I'll ignore it.",
                stop_area_id, filter_provenance, filter_str
            );
            return Err(());
        }
    }

    warn!(
        "Invalid {} filter : {}. I'll ignore it.",
        filter_provenance, filter_str
    );
    Err(())
}
