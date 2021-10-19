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

use launch::loki::{chrono::NaiveDate, transit_model::Model, Idx, NaiveDateTime, StopPoint};

use crate::chaos_proto;

pub struct KirinModel {
    pub disruption_impacts: HashMap<DisruptionId, ImpactedVehicleAndStops>,

    // maps a vehicle_journey_id to its history.
    // the base schedule is not included in this history
    // if the vehicle_journey_id is not present in this map,
    // it means that this vehicle_journey follows its base schedule
    pub base_vehicle_journeys_id_to_idx: HashMap<BaseVehicleJourneyId, BaseVehicleJourney>,

    pub base_vehicle_journeys_history: Vec<(BaseVehicleJourneyId, VehicleJourneyHistory)>,

    pub new_vehicle_journeys_id_to_idx: HashMap<NewVehicleJourneyId, NewVehicleJourney>,

    pub new_vehicle_journeys_history: Vec<(NewVehicleJourneyId, VehicleJourneyHistory)>,

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

pub struct BaseVehicleJourneyId {
    id: String,
}

pub struct NewVehicleJourneyId {
    id: String,
}

pub struct BaseVehicleJourney {
    idx: usize, // position in base_vehicle_journeys_history
}

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
    history_by_reference_date: HashMap<NaiveDate, TripHistory>,
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
    ) {
    }
}
