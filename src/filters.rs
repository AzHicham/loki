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

use crate::{
    models::{
        base_model::{
            PREFIX_ID_COMMERCIAL_MODE, PREFIX_ID_LINE, PREFIX_ID_NETWORK, PREFIX_ID_PHYSICAL_MODE,
            PREFIX_ID_ROUTE, PREFIX_ID_STOP_AREA, PREFIX_ID_STOP_POINT,
        },
        ModelRefs, StopPointIdx, VehicleJourneyIdx,
    },
    tracing::warn,
};

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
    pub fn applies_on(&self, idx: &VehicleJourneyIdx, model: &ModelRefs<'_>) -> bool {
        match self {
            VehicleFilter::Line(line_id) => {
                let vj_line_id = model.line_name(idx);
                vj_line_id == *line_id
            }
            VehicleFilter::Route(route_id) => {
                let vj_route_id = model.route_name(idx);
                vj_route_id == *route_id
            }
            VehicleFilter::Network(network_id) => {
                let vj_network_id = model.network_name(idx);
                vj_network_id == *network_id
            }
            VehicleFilter::PhysicalMode(physical_mode_id) => {
                let vj_physical_mode_id = model.physical_mode_name(idx);
                vj_physical_mode_id == *physical_mode_id
            }
            VehicleFilter::CommercialMode(commercial_mode_id) => {
                let vj_commercial_mode_id = model.commercial_mode_name(idx);
                vj_commercial_mode_id == *commercial_mode_id
            }
        }
    }
}

impl<'a> StopFilter<'a> {
    pub fn applies_on(&self, idx: &StopPointIdx, model: &ModelRefs<'_>) -> bool {
        match self {
            StopFilter::StopPoint(stop_point_id) => *stop_point_id == model.stop_point_name(idx),
            StopFilter::StopArea(stop_area_id) => *stop_area_id == model.stop_area_name(idx),
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
    pub fn is_vehicle_journey_valid(&self, idx: &VehicleJourneyIdx, model: &ModelRefs<'_>) -> bool {
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
    pub fn is_stop_point_valid(&self, idx: &StopPointIdx, model: &ModelRefs<'_>) -> bool {
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
        model: &ModelRefs<'_>,
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
    model: &ModelRefs<'_>,
    filter_str: &'a str,
    filter_provenance: &str,
) -> Result<Filter<'a>, ()> {
    if let Some(line_id) = filter_str.strip_prefix(PREFIX_ID_LINE) {
        if model.contains_line_id(line_id) {
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
    if let Some(route_id) = filter_str.strip_prefix(PREFIX_ID_ROUTE) {
        if model.contains_route_id(route_id) {
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
    if let Some(network_id) = filter_str.strip_prefix(PREFIX_ID_NETWORK) {
        if model.contains_network_id(network_id) {
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

    if let Some(physical_mode_id) = filter_str.strip_prefix(PREFIX_ID_PHYSICAL_MODE) {
        if model.contains_physical_mode_id(physical_mode_id) {
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

    if let Some(commercial_model_id) = filter_str.strip_prefix(PREFIX_ID_COMMERCIAL_MODE) {
        if model.contains_commercial_model_id(commercial_model_id) {
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

    if let Some(stop_point_id) = filter_str.strip_prefix(PREFIX_ID_STOP_POINT) {
        if model.contains_stop_point_id(stop_point_id) {
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

    if let Some(stop_area_id) = filter_str.strip_prefix(PREFIX_ID_STOP_AREA) {
        if model.contains_stop_area_id(stop_area_id) {
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
