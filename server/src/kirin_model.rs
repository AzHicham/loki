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
    chrono::NaiveDate, time::SecondsSinceUTCDayStart, timetables::FlowDirection,
    transit_model::Model, Idx, NaiveDateTime, StopPoint,
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
        feed_entity: &chaos_proto::gtfs_realtime::FeedEntity,
    ) -> Result<(), Error> {
        let disruption_id = DisruptionId {
            id: feed_entity.get_id().to_string(),
        };
        if feed_entity.has_trip_update().not() {
            return Err(format_err!("Feed entity has no trip_update"));
        }
        let trip_update = feed_entity.get_trip_update();

        if let Some(effect) = chaos_proto::kirin::exts::effect.get(trip_update) {
            use chaos_proto::gtfs_realtime::Alert_Effect::*;
            match effect {
                NO_SERVICE => {
                    self.try_delete_trip(disruption_id, trip_update, model);
                }
                ADDITIONAL_SERVICE => {
                    self.try_add_trip(disruption_id, trip_update, model);
                }
                REDUCED_SERVICE | SIGNIFICANT_DELAYS | DETOUR | MODIFIED_SERVICE => {
                    self.try_modify_trip(disruption_id, trip_update, model);
                }

                OTHER_EFFECT | UNKNOWN_EFFECT | STOP_MOVED => {
                    return Err(format_err!("Unhandle effect on FeedEntity: {:?}", effect));
                }
            }
        } else {
            return Err(format_err!("No effect on FeedEntity."));
        }

        Ok(())
    }

    fn get_or_insert_trip(
        &mut self,
        vehicle_id: &str,
        date: &NaiveDate,
        model: &Model,
    ) -> (Trip, &mut TripHistory) {
        // is vehicle_id a Base vehicle_journey ?

        let (trip, vehicle_journey_history) =
        // is vehicle_id a Base vehicle_journey ?
        if let Some(base_vj_transit_model_idx) = model.vehicle_journeys.get_idx(vehicle_id) {
            let base_vj = self
                .base_vehicle_journeys_idx_map
                .entry(base_vj_transit_model_idx)
                // we have not encountered this Base vehicle_journey yet
                // let's add it to our structures
                .or_insert_with(|| {
                    let result = BaseVehicleJourney {
                        idx : self.base_vehicle_journeys_history.len()
                    };
                    self.base_vehicle_journeys_history.push((base_vj_transit_model_idx, VehicleJourneyHistory::new()));

                    result
                });

            let trip = Trip {
                vehicle_journey : VehicleJourney::Base(base_vj.clone()),
                reference_date: date.clone(),
            };
            let vehicle_journey_history = & mut self.base_vehicle_journeys_history[base_vj.idx].1;
            (trip, vehicle_journey_history)

        }
        else {
            let new_vj_id = NewVehicleJourneyId {
                id : vehicle_id.to_string()
            };
            let new_vj_idx = self.new_vehicle_journeys_id_to_idx
                                    .entry(new_vj_id)
                                    // we have not encountered this New vehicle yet,
                                    // let's add it to our structures
                                    .or_insert_with( || {
                                        let new_vj_idx = NewVehicleJourney {
                                            idx : self.new_vehicle_journeys_history.len()
                                        };
                                        self.new_vehicle_journeys_history.push((new_vj_id, VehicleJourneyHistory::new()));
                                        new_vj_idx
                                    });
            let trip = Trip {
                vehicle_journey : VehicleJourney::New(new_vj_idx.clone()),
                reference_date : date.clone(),
            };
            let vehicle_journey_history = & mut self.new_vehicle_journeys_history[new_vj_idx.idx].1;
            (trip, vehicle_journey_history)
        };

        let trip_history = vehicle_journey_history
            .by_reference_date
            .entry(date.clone())
            // we have no history for this (vehicle_id, date) so let's add an empty history
            .or_insert_with(|| TripHistory::new());

        return (trip, trip_history);
    }

    fn try_delete_trip(
        &mut self,
        disruption_id: DisruptionId,
        trip_update: &chaos_proto::gtfs_realtime::TripUpdate,
        model: &Model,
    ) -> Result<DataUpdateMessage, Error> {
        let vehicle_id = vehicle_id(trip_update)?;
        let date = reference_date(trip_update)?;

        let (trip, history) = self.get_or_insert_trip(vehicle_id, &date, model);

        match (trip.vehicle_journey, history.versions.last()) {
            (VehicleJourney::New(_), None) => {
                return Err(format_err!("Trying to delete the New vehicle {} on date {}, but we have no history for this trip.", vehicle_id, date));
            }
            (_, Some((_, TripData::Deleted()))) => {
                return Err(format_err!(
                    "Trying to delete the vehicle {} on date {}, but this trip is already deleted.",
                    vehicle_id,
                    date
                ));
            }
            _ => {
                history.versions.push((disruption_id, TripData::Deleted()));
                return Ok(DataUpdateMessage::Delete(trip));
            }
        }
    }

    fn try_modify_trip(
        &mut self,
        disruption_id: DisruptionId,
        trip_update: &chaos_proto::gtfs_realtime::TripUpdate,
        model: &Model,
    ) -> Result<DataUpdateMessage, Error> {
        let vehicle_id = vehicle_id(trip_update)?;
        let date = reference_date(trip_update)?;

        let (trip, history) = self.get_or_insert_trip(vehicle_id, &date, model);

        match (trip.vehicle_journey, history.versions.last()) {
            (VehicleJourney::New(_), None) => Err(format_err!(
                "Trying to modify a New vehicle {} on date {}, but this trip does not exists.",
                vehicle_id,
                date
            )),
            (_, Some((_, TripData::Deleted()))) => Err(format_err!(
                "Trying to modify the vehicle {} on date {}, but this trip is deleted.",
                vehicle_id,
                date
            )),
            _ => {
                let stop_times = self.create_stop_times_from_proto(
                    trip_update.get_stop_time_update(),
                    &date,
                    model,
                )?;
                history
                    .versions
                    .push((disruption_id, TripData::Present(stop_times)));
                Ok(DataUpdateMessage::Update(trip, stop_times))
            }
        }
    }

    fn try_add_trip(
        &mut self,
        disruption_id: DisruptionId,
        trip_update: &chaos_proto::gtfs_realtime::TripUpdate,
        model: &Model,
    ) -> Result<DataUpdateMessage, Error> {
        let vehicle_id = vehicle_id(trip_update)?;
        let date = reference_date(trip_update)?;

        let (trip, history) = self.get_or_insert_trip(vehicle_id, &date, model);

        match (trip.vehicle_journey, history.versions.last()) {
            (VehicleJourney::Base(_), None) => Err(format_err!(
                "Trying to add Base vehicle {} on date {}, but this trip is already present.",
                vehicle_id,
                date
            )),
            (_, Some((_, TripData::Present(_)))) => Err(format_err!(
                "Trying to add the vehicle {} on date {}, but this trip is already present.",
                vehicle_id,
                date
            )),
            _ => {
                let stop_times = self.create_stop_times_from_proto(
                    trip_update.get_stop_time_update(),
                    &date,
                    model,
                )?;
                history
                    .versions
                    .push((disruption_id, TripData::Present(stop_times)));
                Ok(DataUpdateMessage::Add(trip, stop_times))
            }
        }
    }

    fn create_stop_times_from_proto(
        &mut self,
        proto: &[chaos_proto::gtfs_realtime::TripUpdate_StopTimeUpdate],
        reference_date: &NaiveDate,
        model: &Model,
    ) -> Result<Vec<StopTime>, Error> {
        proto
            .iter()
            .map(|p| self.create_stop_time_from_proto(p, reference_date, model))
            .collect()
    }

    fn create_stop_time_from_proto(
        &mut self,
        proto: &chaos_proto::gtfs_realtime::TripUpdate_StopTimeUpdate,
        reference_date: &NaiveDate,
        model: &Model,
    ) -> Result<StopTime, Error> {
        let has_arrival_time = if proto.has_arrival() {
            let arrival_time = read_time(proto.get_arrival(), reference_date)
                .map_err(|err| format_err!("StopTime has a bad arrival time : {}", err))?;
            Some(arrival_time)
        } else {
            None
        };

        let has_departure_time = if proto.has_departure() {
            let departure_time = read_time(proto.get_departure(), reference_date)
                .map_err(|err| format_err!("StopTime has a bad departure time : {}", err))?;
            Some(departure_time)
        } else {
            None
        };

        let (arrival_time, departure_time) = match (has_arrival_time, has_departure_time) {
            (Some(arrival_time), Some(departure_time)) => (arrival_time, departure_time),
            (Some(arrival_time), None) => (arrival_time, arrival_time),
            (None, Some(departure_time)) => (departure_time, departure_time),
            (None, None) => {
                return Err(format_err!(
                    "StopTime does not have an arrival time nor a departure time."
                ));
            }
        };

        let can_board = if proto.has_departure() {
            read_status(proto.get_departure())
                .map_err(|err| format_err!("StopTime has a bad departure status : {}", err))?
        } else {
            false
        };

        let can_debark = if proto.has_arrival() {
            read_status(proto.get_arrival())
                .map_err(|err| format_err!("StopTime has a bad arrival status : {}", err))?
        } else {
            false
        };

        let flow_direction = match (can_board, can_debark) {
            (true, true) => FlowDirection::BoardAndDebark,
            (true, false) => FlowDirection::BoardOnly,
            (false, true) => FlowDirection::DebarkOnly,
            (false, false) => FlowDirection::NoBoardDebark,
        };

        if proto.has_stop_id().not() {
            return Err(format_err!("StopTime does not have a stop_id."));
        }
        let stop_id = proto.get_stop_id();
        let stop = self.get_or_insert_stop(stop_id, model);

        let stop_time = StopTime {
            stop,
            arrival_time,
            departure_time,
            flow_direction,
        };

        Ok(stop_time)
    }

    fn get_or_insert_stop(&mut self, stop_id: &str, model: &Model) -> Stop {
        if let Some(stop_point_idx) = model.stop_points.get_idx(stop_id) {
            Stop::Base(stop_point_idx)
        } else {
            let new_stop_id = NewStopId {
                id: stop_id.to_string(),
            };
            let new_stop_idx = self
                .new_stop_id_to_idx
                .entry(new_stop_id)
                .or_insert_with(|| {
                    let new_stop_idx = NewStop {
                        idx: self.new_stops.len(),
                    };
                    self.new_stops.push(StopData {});
                    new_stop_idx
                });
            Stop::New(new_stop_idx.clone())
        }
    }
}

fn read_time(
    proto: &chaos_proto::gtfs_realtime::TripUpdate_StopTimeEvent,
    reference_date: &NaiveDate,
) -> Result<SecondsSinceUTCDayStart, Error> {
    if proto.has_time().not() {
        return Err(format_err!("The protobuf time field is empty."));
    }
    // this is a unix timestamp
    let time_i64 = proto.get_time();
    let naive_datetime = NaiveDateTime::from_timestamp_opt(time_i64, 0).ok_or_else(|| {
        format_err!(
            "Could not parse the time value {} as a unix timestamp.",
            time_i64
        )
    })?;

    let reference_date_at_midnight = reference_date.and_hms(0, 0, 0);
    let duration_from_ref = naive_datetime.signed_duration_since(reference_date_at_midnight);
    let duration_i64 = duration_from_ref.num_seconds();
    SecondsSinceUTCDayStart::from_seconds_i64(duration_i64).ok_or_else(|| {
        format_err!(
            "Could not translate the duration of {} seconds to SecondsSinceUTCDayStart.",
            duration_i64
        )
    })
}

fn read_status(
    proto: &chaos_proto::gtfs_realtime::TripUpdate_StopTimeEvent,
) -> Result<bool, Error> {
    use chaos_proto::kirin::StopTimeEventStatus::*;
    if let Some(stop_time_event_status) =
        chaos_proto::kirin::exts::stop_time_event_status.get(proto)
    {
        match stop_time_event_status {
            SCHEDULED | ADDED | ADDED_FOR_DETOUR => Ok(true),

            DELETED_FOR_DETOUR | DELETED => Ok(false),

            NO_DATA => Err(format_err!("No_data in stop time event status.")),
        }
    } else {
        Ok(false)
    }
}

fn vehicle_id(trip_update: &chaos_proto::gtfs_realtime::TripUpdate) -> Result<&str, Error> {
    let trip_proto = trip_update.get_trip();
    if trip_proto.has_trip_id().not() {
        return Err(format_err!(
            "Received a TripUpdate with an empty self.trip.trip_id."
        ));
    }
    Ok(trip_proto.get_trip_id())
}

fn reference_date(
    trip_update: &chaos_proto::gtfs_realtime::TripUpdate,
) -> Result<NaiveDate, Error> {
    if trip_update.get_trip().has_start_date().not() {
        return Err(format_err!(
            "Received a TripUpdate with an empty self.trip.start_time. I cannot handle it."
        ));
    }
    NaiveDate::parse_from_str(trip_update.get_trip().get_start_date(), "%Y%m%d")
        .map_err(|err| format_err!("Could not parse start date {}", err))
}

impl VehicleJourneyHistory {
    pub fn new() -> Self {
        let map = HashMap::new();
        Self {
            by_reference_date: map,
        }
    }
}

impl TripHistory {
    pub fn new() -> Self {
        Self {
            versions: Vec::new(),
        }
    }
}
