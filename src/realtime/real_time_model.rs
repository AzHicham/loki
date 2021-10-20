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

use crate::{
    chrono::NaiveDate, time::SecondsSinceUTCDayStart, timetables::FlowDirection,
    transit_model::Model, Idx, NaiveDateTime, StopPoint,
    VehicleJourney as TransitModelVehicleJourney,
};

type TransitModelVehicleJourneyIdx = Idx<TransitModelVehicleJourney>;

pub struct RealTimeModel {
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

    pub new_stop_id_to_idx: HashMap<NewStopId, NewStop>,
    pub new_stops: Vec<StopData>,
}

pub struct DisruptionId {
    id: String,
}

#[derive(Eq, PartialEq, Hash, Clone)]
pub struct NewStopId {
    id: String,
}

#[derive(Clone)]
pub struct NewStop {
    idx: usize, // position in new_stops
}

#[derive(Eq, PartialEq, Hash, Clone)]
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
    stops: Vec<Stop>,
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
    stop: Stop,
    arrival_time: SecondsSinceUTCDayStart,
    departure_time: SecondsSinceUTCDayStart,
    flow_direction: FlowDirection,
}

pub type TransitModelStopIdx = Idx<StopPoint>;

pub enum Stop {
    Base(TransitModelStopIdx), // Stop_id in ntfs
    New(NewStop),              // Id of a stop added by real time
}

pub struct StopData {}
