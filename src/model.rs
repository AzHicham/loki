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

pub mod real_time;
pub mod disruption;
pub mod infos;

use chrono::NaiveDate;
use typed_index_collection::{Id, Idx};

use crate::{transit_data};

use self::real_time::{NewStopPointIdx, NewVehicleJourneyIdx, RealTimeModel, TripData};

pub type TransitModelVehicleJourneyIdx = Idx<transit_data::VehicleJourney>;

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum VehicleJourneyIdx {
    Base(TransitModelVehicleJourneyIdx),
    New(NewVehicleJourneyIdx),
}

pub type TransitModelStopPointIdx = Idx<transit_data::StopPoint>;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum StopPointIdx {
    Base(TransitModelStopPointIdx), // Stop_id in ntfs
    New(NewStopPointIdx),              // Id of a stop added by real time
}


pub type TransitModelTransferIdx = Idx<transit_model::objects::Transfer>;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum TransferIdx {
    Base(TransitModelTransferIdx),
    New(usize),
}

pub struct ModelRefs<'model> {
    pub base : & 'model transit_model::Model,
    pub real_time : & 'model RealTimeModel,
}


impl<'model> ModelRefs<'model> {

    pub fn new(    
        base : & 'model transit_model::Model,
        real_time : & 'model RealTimeModel,
    ) -> Self {
        Self {
            base, 
            real_time
        }
    }

    pub fn stop_point_idx(&self, stop_id: &str) -> Option<StopPointIdx> {
        if let Some(base_stop_point_id) = self.base.stop_points.get_idx(stop_id) {
            Some(StopPointIdx::Base(base_stop_point_id))
        } else if let Some(new_stop_idx) = self.real_time.new_stop_id_to_idx.get(stop_id) {
            Some(StopPointIdx::New(new_stop_idx.clone()))
        } else {
            None
        }
    }

    pub fn stop_point_name<'a>(
        &'a self,
        stop_idx: &StopPointIdx,
    ) -> &'a str {
        match stop_idx {
            StopPointIdx::Base(idx) => &self.base.stop_points[*idx].id,
            StopPointIdx::New(idx) => &self.real_time.new_stops[idx.idx].name,
        }
    }

    pub fn stop_area_name<'a>(&'a self, stop_idx: &StopPointIdx) -> &'a str {
        match stop_idx {
            StopPointIdx::Base(idx) => &self.base.stop_points[*idx].stop_area_id,
            StopPointIdx::New(_idx) => "unknown_stop_area",
        }
    }

    pub fn vehicle_journey_name<'a>(
        &'a self,
        vehicle_journey_idx: &VehicleJourneyIdx,
    ) -> &'a str {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => &self.base.vehicle_journeys[*idx].id,
            VehicleJourneyIdx::New(idx) => &self.real_time.new_vehicle_journeys_history[idx.idx].0,
        }
    }

    pub fn line_name<'a>(
        &'a self,
        vehicle_journey_idx: &VehicleJourneyIdx,
    ) -> &'a str {
        let unknown_line = "unknown_line";
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                self.base_vehicle_journey_route(idx)
                    .map(|route| route.line_id.as_str())
                    .unwrap_or(unknown_line)
            }
            VehicleJourneyIdx::New(_idx) => unknown_line,
        }
    }


    fn base_vehicle_journey_line(&self, idx : TransitModelVehicleJourneyIdx) -> Option<& transit_model::objects::Line> {
        self.base_vehicle_journey_route(&idx)
            .map(|route|
                self.base.lines.get(route.line_id.as_str())
            )
            .flatten()

    }

    pub fn route_name<'a>(
        &'a self,
        vehicle_journey_idx: &VehicleJourneyIdx,
    ) -> &'a str {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => &self.base.vehicle_journeys[*idx].route_id,
            VehicleJourneyIdx::New(_idx) => "unknown_route",
        }
    }


    fn base_vehicle_journey_route(&self, idx : &TransitModelVehicleJourneyIdx) -> Option<& transit_model::objects::Route> {
        let route_id = &self.base.vehicle_journeys[*idx].route_id;
        self.base
            .routes
            .get(route_id)
    }
    pub fn network_name<'a>(
        &'a self,
        vehicle_journey_idx: &VehicleJourneyIdx,
    ) -> &'a str {
        let unknown_network = "unknown_network";
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let route_id = &self.base.vehicle_journeys[*idx].route_id;
                let has_route = self.base.routes.get(route_id);
                if let Some(route) = has_route {
                    let has_line = self.base.lines.get(&route.line_id);
                    if let Some(line) = has_line {
                        line.network_id.as_str()
                    } else {
                        unknown_network
                    }
                } else {
                    unknown_network
                }
            }
            VehicleJourneyIdx::New(_idx) => unknown_network,
        }
    }

    pub fn physical_mode_name<'a>(
        &'a self,
        vehicle_journey_idx: &VehicleJourneyIdx,
    ) -> &'a str {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => &self.base.vehicle_journeys[*idx].physical_mode_id,
            VehicleJourneyIdx::New(_idx) => "unknown_physical_mode",
        }
    }

    pub fn commercial_mode_name<'a>(
        &'a self,
        vehicle_journey_idx: &VehicleJourneyIdx,
    ) -> &'a str {
        let unknown_commercial_mode = "unknown_commercial_mode";
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let vj = &self.base.vehicle_journeys[*idx];
                let has_route = self.base.routes.get(&vj.route_id);
                if let Some(route) = has_route {
                    let has_line = self.base.lines.get(&route.line_id);
                    if let Some(line) = has_line {
                        line.commercial_mode_id.as_str()
                    } else {
                        unknown_commercial_mode
                    }
                } else {
                    unknown_commercial_mode
                }
            }
            VehicleJourneyIdx::New(_idx) => unknown_commercial_mode,
        }
    }


    pub fn stop_point_at(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        stop_time_idx: usize,
        date: &NaiveDate,
    ) -> Option<StopPointIdx> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let has_history = self.real_time.base_vehicle_journey_last_version(idx, date);
                if let Some(trip_data) = has_history {
                    match trip_data {
                        TripData::Deleted() => None,
                        TripData::Present(stop_times) => stop_times
                            .get(stop_time_idx)
                            .map(|stop_time| stop_time.stop.clone()),
                    }
                } else {
                    self.base.vehicle_journeys[*idx]
                        .stop_times
                        .get(stop_time_idx)
                        .map(|stop_time| StopPointIdx::Base(stop_time.stop_point_idx))
                }
            }
            VehicleJourneyIdx::New(idx) => {
                let has_history = self.real_time.new_vehicle_journeys_history[idx.idx].1.trip_data(date);
                if let Some((_, trip_data)) = has_history {
                    match trip_data {
                        TripData::Deleted() => None,
                        TripData::Present(stop_times) => stop_times
                            .get(stop_time_idx)
                            .map(|stop_time| stop_time.stop.clone()),
                    }
                } else {
                    None
                }
            }
        }
    }

    pub fn nb_of_new_stops(&self) -> usize {
        self.real_time.new_stops.len()
    }

    pub fn new_stops(&self) -> impl Iterator<Item = NewStopPointIdx> {
        let range = 0..self.nb_of_new_stops();
        range.map(|idx| NewStopPointIdx { idx })
    }

    pub fn nb_of_base_stops(&self) -> usize {
        self.base.stop_points.len()
    }

    pub fn base_stop_points(&self) -> impl Iterator<Item = TransitModelStopPointIdx> + 'model {
        self.base.stop_points.iter()
            .map(|(idx, _)| idx)
    }

    pub fn nb_of_new_vehicle_journeys(&self) -> usize {
        self.real_time.new_vehicle_journeys_history.len()
    }

    pub fn new_vehicle_journeys(&self) -> impl Iterator<Item = NewVehicleJourneyIdx> {
        let range = 0..self.nb_of_new_vehicle_journeys();
        range.map(|idx| NewVehicleJourneyIdx { idx })
    }

    pub fn nb_of_base_vehicle_journeys(&self) -> usize {
        self.base.vehicle_journeys.len()
    }

    pub fn base_vehicle_journeys(&self) -> impl Iterator<Item = TransitModelVehicleJourneyIdx> + 'model {
        self.base.vehicle_journeys.iter()
            .map(|(idx, _)| idx)
    }

    pub fn contains_line_id(&self, id : &str) -> bool {
        self.base.lines.contains_id(id)
    }

    pub fn contains_route_id(&self, id : &str) -> bool {
        self.base.routes.contains_id(id)
    }

    pub fn contains_network_id(&self, id : &str) -> bool {
        self.base.networks.contains_id(id)
    }

    pub fn contains_physical_mode_id(&self, id : &str) -> bool {
        self.base.physical_modes.contains_id(id)
    }

    pub fn contains_commercial_model_id(&self, id : &str) -> bool {
        self.base.commercial_modes.contains_id(id)
    }

    pub fn contains_stop_point_id(&self, id : &str) -> bool {
        if self.base.stop_points.contains_id(id){
            true
        }
        else {
            self.real_time.new_stop_id_to_idx.contains_key(id)
        }
    }

    pub fn contains_stop_area_id(&self, id : &str) -> bool {
        self.base.stop_areas.contains_id(id)
    }

}
