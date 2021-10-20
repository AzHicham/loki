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

use std::collections::HashMap;

use crate::{
    chrono::NaiveDate, time::SecondsSinceUTCDayStart, timetables::FlowDirection,
    transit_model::Model, Idx, StopPoint, VehicleJourney as TransitModelVehicleJourney,
};

type TransitModelVehicleJourneyIdx = Idx<TransitModelVehicleJourney>;

pub struct RealTimeModel {
    base_model: Model,

    disruption_impacts: HashMap<String, ImpactedVehicleAndStops>,

    new_vehicle_journeys_id_to_idx: HashMap<String, NewVehicleJourney>,
    // indexed by NewVehicleJourney.idx
    new_vehicle_journeys_history: Vec<(String, VehicleJourneyHistory)>,

    // indexed by Idx<TransitModel>.get()
    base_vehicle_journeys_history: Vec<VehicleJourneyHistory>,

    new_stop_id_to_idx: HashMap<String, NewStop>,
    new_stops: Vec<StopData>,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct NewVehicleJourney {
    idx: usize, // position in new_vehicle_journeys_history
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum VehicleJourneyIdx {
    Base(TransitModelVehicleJourneyIdx),
    New(NewVehicleJourney),
}

#[derive(Debug, Clone)]
pub struct VehicleJourneyHistory {
    by_reference_date: HashMap<NaiveDate, TripHistory>,
}

#[derive(Debug, Clone)]
pub struct TripHistory {
    // String is the disruption id
    versions: Vec<(String, TripData)>, // all versions of this trip,
}

#[derive(Debug, Clone)]
pub enum TripData {
    Deleted(),              // the trip is currently disabled
    Present(Vec<StopTime>), // list of all stop times of this trip
}

#[derive(Debug, Clone)]
pub struct StopTime {
    stop: StopPointIdx,
    arrival_time: SecondsSinceUTCDayStart,
    departure_time: SecondsSinceUTCDayStart,
    flow_direction: FlowDirection,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct NewStop {
    idx: usize, // position in new_stops
}

pub type TransitModelStopIdx = Idx<StopPoint>;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum StopPointIdx {
    Base(TransitModelStopIdx), // Stop_id in ntfs
    New(NewStop),              // Id of a stop added by real time
}

pub struct StopData {
    name: String,
}

type TransitModelTransferIdx = Idx<transit_model::objects::Transfer>;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum TransferIdx {
    Base(TransitModelTransferIdx),
    New(usize),
}

pub struct ImpactedVehicleAndStops {
    vehicle_journey: VehicleJourneyIdx,
    stops: Vec<StopPointIdx>,
}

impl RealTimeModel {
    // pub fn base_model(&self) -> & Model {
    //     &self.base_model
    // }

    pub fn new(base_model: Model) -> Self {
        let nb_of_base_vj = base_model.vehicle_journeys.len();
        let empty_history = VehicleJourneyHistory {
            by_reference_date: HashMap::new(),
        };
        let base_vehicle_journeys_history = vec![empty_history; nb_of_base_vj];

        Self {
            base_model,
            disruption_impacts: HashMap::new(),
            new_vehicle_journeys_id_to_idx: HashMap::new(),
            new_vehicle_journeys_history: Vec::new(),
            base_vehicle_journeys_history,
            new_stop_id_to_idx: HashMap::new(),
            new_stops: Vec::new(),
        }
    }

    pub fn stop_point_idx(&self, stop_id: &str) -> Option<StopPointIdx> {
        if let Some(base_stop_point_id) = self.base_model.stop_points.get_idx(stop_id) {
            Some(StopPointIdx::Base(base_stop_point_id))
        } else if let Some(new_stop_idx) = self.new_stop_id_to_idx.get(stop_id) {
            Some(StopPointIdx::New(new_stop_idx.clone()))
        } else {
            None
        }
    }

    pub fn stop_point_name(&self, stop_idx: &StopPointIdx) -> &str {
        match stop_idx {
            StopPointIdx::Base(idx) => &self.base_model.stop_points[*idx].id,
            StopPointIdx::New(idx) => &self.new_stops[idx.idx].name,
        }
    }

    pub fn vehicle_journey_name(&self, vehicle_journey_idx: &VehicleJourneyIdx) -> &str {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => &self.base_model.vehicle_journeys[*idx].id,
            VehicleJourneyIdx::New(idx) => &self.new_vehicle_journeys_history[idx.idx].0,
        }
    }

    pub fn line_name(&self, vehicle_journey_idx: &VehicleJourneyIdx) -> &str {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let route_id = &self.base_model.vehicle_journeys[*idx].route_id;
                &self
                    .base_model
                    .routes
                    .get(route_id)
                    .map(|route| route.line_id.as_str())
                    .unwrap_or("unknown_line")
            }
            VehicleJourneyIdx::New(idx) => "unknown_line",
        }
    }

    pub fn route_name(&self, vehicle_journey_idx: &VehicleJourneyIdx) -> &str {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => &self.base_model.vehicle_journeys[*idx].route_id,
            VehicleJourneyIdx::New(idx) => "unknown_route",
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
                let has_history = self.base_vehicle_journeys_history[idx.get()].trip_data(date);
                if let Some((_, trip_data)) = has_history {
                    match trip_data {
                        TripData::Deleted() => None,
                        TripData::Present(stop_times) => stop_times
                            .get(stop_time_idx)
                            .map(|stop_time| stop_time.stop),
                    }
                } else {
                    self.base_model.vehicle_journeys[*idx]
                        .stop_times
                        .get(stop_time_idx)
                        .map(|stop_time| StopPointIdx::Base(stop_time.stop_point_idx))
                }
            }
            VehicleJourneyIdx::New(idx) => {
                let has_history = self.new_vehicle_journeys_history[idx.idx].1.trip_data(date);
                if let Some((_, trip_data)) = has_history {
                    match trip_data {
                        TripData::Deleted() => None,
                        TripData::Present(stop_times) => stop_times
                            .get(stop_time_idx)
                            .map(|stop_time| stop_time.stop),
                    }
                } else {
                    None
                }
            }
        }
    }

    // pub fn nb_of_stop_points(&self)-> usize {
    //     self.new_stops.len() + self.base_model.stop_points.len()
    // }

    // pub fn nb_of_transfers(&self) -> usize {
    //     self.base_model.transfers.len()
    // }

    // pub fn validity_period(&self) -> (NaiveDate, NaiveDate) {
    //     self.base_model.calculate_validity_period()
    //         .expect("Unable to calculate a validity period.")
    // }
}

impl VehicleJourneyHistory {
    pub fn new() -> Self {
        Self {
            by_reference_date: HashMap::new(),
        }
    }

    pub fn trip_data(&self, date: &NaiveDate) -> Option<&(String, TripData)> {
        self.by_reference_date
            .get(date)
            .map(|trip_history| trip_history.versions.last())
            .flatten()
    }
}
