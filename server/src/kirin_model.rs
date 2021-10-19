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
use std::{collections::HashMap, ops::Not};

use failure::{format_err, Error};
use launch::loki::{
    chrono::NaiveDate, transit_model::Model, Idx, NaiveDateTime, StopPoint,
    VehicleJourney as TransitModelVehicleJourney,
};

use crate::chaos_proto;

type TransitModelVehicleJourneyIdx = Idx<TransitModelVehicleJourney>;

pub struct KirinModel {
    pub disruption_impacts: HashMap<DisruptionId, ImpactedVehicleAndStops>,

    pub new_vehicle_journeys_id_to_idx: HashMap<NewVehicleJourneyId, NewVehicleJourney>,
    // indexed by NewVehicleJourney.idx
    pub new_vehicle_journeys_history: Vec<(NewVehicleJourneyId, VehicleJourneyHistory)>,

    // maps a vehicle_journey_idx to its history.
    // the base schedule is not included in this history
    // if the vehicle_journey_idx is not present in this map,
    // it means that this vehicle_journey follows its base schedule
    pub base_vehicle_journeys_idx_map: HashMap<TransitModelVehicleJourneyIdx, BaseVehicleJourney>,
    // indexed by BaseVehicleJourney.idx
    pub base_vehicle_journeys_history: Vec<(TransitModelVehicleJourneyIdx, VehicleJourneyHistory)>,

    pub new_stop_id_to_idx: HashMap<NewStopId, StopId>,
    pub new_stops: Vec<StopData>,
}

pub struct DisruptionId {
    id: String,
}

pub struct NewStopId {
    id: String,
}

pub struct NewStop {
    idx: usize, // position in new_stops
}

#[derive(Eq, PartialEq, Hash)]
pub struct BaseVehicleJourneyId {
    id: String,
}

#[derive(Eq, PartialEq, Hash)]
pub struct NewVehicleJourneyId {
    id: String,
}

#[derive(Copy, Clone)]
pub struct BaseVehicleJourney {
    idx: usize, // position in base_vehicle_journeys_history
}

#[derive(Copy, Clone)]
pub struct NewVehicleJourney {
    idx: usize, // position in new_vehicle_journeys_history
}

pub struct ImpactedVehicleAndStops {
    vehicle_journey: VehicleJourney,
    stops: Vec<StopId>,
}

pub enum VehicleJourney {
    Base(BaseVehicleJourney),
    New(NewVehicleJourney),
}

pub struct VehicleJourneyHistory {
    by_reference_date: HashMap<NaiveDate, TripHistory>,
}

pub struct TripHistory {
    versions: Vec<(DisruptionId, TripData)>, // all versions of this trip,
}

pub enum TripData {
    Deleted(),              // the trip is currently disabled
    Present(Vec<StopTime>), // list of all stop times of this trip
}

pub struct StopTime {
    stop: StopId,
    arrival_time: NaiveDateTime,
    departure_time: NaiveDateTime,
}

pub type TransitModelStopIdx = Idx<StopPoint>;

pub enum StopId {
    Base(TransitModelStopIdx), // Stop_id in ntfs
    RealTime(NewStop),         // Id of a stop added by real time
}

pub struct StopData {}

pub enum DataUpdateMessage {
    Delete(Trip),
    Add(Trip, Vec<StopTime>),
    Update(Trip, Vec<StopTime>),
}

pub struct Trip {
    vehicle_journey: VehicleJourney,
    reference_date: NaiveDate,
}

impl KirinModel {
    pub fn handle_kirin_protobuf(
        &mut self,
        model: &Model,
        trip_update: &chaos_proto::gtfs_realtime::TripUpdate,
    ) -> Result<(), Error> {
        let vehicle_id = {
            let trip_proto = trip_update.get_trip();
            if trip_proto.has_trip_id().not() {
                return Err(format_err!(
                    "Received a TripUpdate with an empty self.trip.trip_id. I cannot handle it."
                ));
            }
            trip_proto.get_trip_id();
        };

        let date = {
            if trip_update.get_trip().has_start_date().not() {
                return Err(format_err!(
                    "Received a TripUpdate with an empty self.trip.start_time. I cannot handle it."
                ));
            }
            NaiveDate::parse_from_str(trip_update.get_trip().get_start_date(), "%Y%m%d")
                .map_err(|err| format_err!("Could not parse start date {}", err))?
        };

        // if trip is cancelled
        if chaos_proto::kirin::exts::effect.get(trip_update)
            == Some(chaos_proto::gtfs_realtime::Alert_Effect::NO_SERVICE)
        {
            // check if the status of this trip is not already cancelled
            // otherwise, add cancellation to history, and forms an update message to the transit_data
        }

        // here trip is added and or/modified
        // check if trip is already known : then it must be a modified
        // check if trip is not already known, but it is in base schedule : add to base_vehicle_journeys_history/idx_map
        // if not in base schedule, this a new trip to be added to new_vehicle_journey_history/id_to_idx

        Ok(())
    }

    fn new_vehicle_journeys_contains(&self, vehicle_id: &str, date: &NaiveDate) -> Option<Trip> {
        let new_vj_id = NewVehicleJourneyId {
            id: vehicle_id.to_string(),
        };

        let new_vj_idx = self.new_vehicle_journeys_id_to_idx.get(&new_vj_id)?;
        let history = self.new_vehicle_journeys_history[new_vj_idx.idx].1;
        if history.by_reference_date.contains_key(date) {
            let vehicle_journey = VehicleJourney::New(new_vj_idx.clone());
            let result = Trip {
                vehicle_journey,
                reference_date: date.clone(),
            };
            return Some(result);
        }
        return None;
    }

    fn base_vehicle_journeys_contains(
        &self,
        vehicle_id: &str,
        date: &NaiveDate,
        model: &Model,
    ) -> Option<Trip> {
        // if vehicle_id is not a valid id for base vehicle journeys, it can't be a modified base vehicle journey
        // and we can return early.
        let base_vj_transit_model_idx = model.vehicle_journeys.get_idx(vehicle_id)?;

        let base_vj = self
            .base_vehicle_journeys_idx_map
            .get(&base_vj_transit_model_idx)?;

        let history = self.base_vehicle_journeys_history[base_vj.idx].1;

        if history.by_reference_date.contains_key(date) {
            let vehicle_journey = VehicleJourney::Base(base_vj.clone());
            let result = Trip {
                vehicle_journey,
                reference_date: date.clone(),
            };
            return Some(result);
        }
        return None;
    }

    // check if (vehicle_id, date) refers to a known trip
    // Returns Some(trip) if
    //  - vehicle_id is a Base vehicle, and we already received some modification for its course on date
    //  - vehicle_id is a New vehicle, and we already received some modification for its course on date
    // Returns None otherwise
    fn contains_trip(&self, vehicle_id: &str, date: &NaiveDate, model: &Model) -> Option<Trip> {
        if let Some(trip) = self.base_vehicle_journeys_contains(vehicle_id, date, model) {
            return Some(trip);
        }
        if let Some(trip) = self.new_vehicle_journeys_contains(vehicle_id, date) {
            return Some(trip);
        }
        return None;
    }

    fn try_delete_trip(&mut self, vehicle_id: &str, date: NaiveDate) {}
}
