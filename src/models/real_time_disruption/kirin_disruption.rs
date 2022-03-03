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
        base_model::BaseModel,
        real_time_model::{KirinDisruptionIdx, TripVersion},
        VehicleJourneyIdx,
    },
    transit_data::data_interface::{Data as DataTrait, DataUpdate},
};

use crate::{
    models::RealTimeModel, time::SecondsSinceTimezonedDayStart, timetables::FlowDirection,
};
use chrono::{NaiveDate, NaiveDateTime};
use tracing::{debug, error};

use std::fmt::Debug;

use super::{apply_disruption, time_periods::TimePeriod, Effect, VehicleJourneyId};

#[derive(Debug, Clone)]
pub struct KirinDisruption {
    pub id: String,

    pub contributor: Option<String>,
    pub message: Option<String>,

    pub updated_at: NaiveDateTime,
    pub application_period: TimePeriod,
    pub effect: Effect,

    pub trip_id: VehicleJourneyId,
    pub trip_date: NaiveDate,

    pub update: UpdateType,
}

#[derive(Debug, Clone)]
pub enum UpdateType {
    TripDeleted(),
    BaseTripUpdated(UpdateData),
    NewTripUpdated(UpdateData),
}

#[derive(Debug, Clone)]
pub struct UpdateData {
    pub stop_times: Vec<StopTime>,
    pub company_id: Option<String>,
    pub physical_mode_id: Option<String>,
    pub headsign: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StopTime {
    pub stop_id: String,
    pub arrival_time: SecondsSinceTimezonedDayStart,
    pub departure_time: SecondsSinceTimezonedDayStart,
    pub flow_direction: FlowDirection,
}

#[derive(Debug, Clone)]
pub enum KirinUpdateError {
    NewTripWithBaseId(VehicleJourneyId, NaiveDate),
    BaseVehicleJourneyUnknown(VehicleJourneyId),
    BaseTripAbsent(VehicleJourneyId, NaiveDate),
    DeleteAbsentTrip(VehicleJourneyId, NaiveDate),
}

pub fn store_and_apply_kirin_disruption<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    disruption: KirinDisruption,
    base_model: &BaseModel,
    data: &mut Data,
) {
    let kirin_disruption_idx = KirinDisruptionIdx {
        idx: real_time_model.kirin_disruptions.len(),
    };

    let date = disruption.trip_date;
    let vehicle_journey_id = disruption.trip_id.id.as_str();

    let result = match &disruption.update {
        UpdateType::TripDeleted() => delete_trip(
            real_time_model,
            base_model,
            data,
            vehicle_journey_id,
            date,
            kirin_disruption_idx,
        ),
        UpdateType::BaseTripUpdated(update_data) => update_base_trip(
            real_time_model,
            base_model,
            data,
            vehicle_journey_id,
            date,
            update_data,
            kirin_disruption_idx,
        ),
        UpdateType::NewTripUpdated(update_data) => update_new_trip(
            real_time_model,
            base_model,
            data,
            vehicle_journey_id,
            date,
            update_data,
            kirin_disruption_idx,
        ),
    };
    if let Err(err) = result {
        error!(
            "Error while applying kirin disruption {}. {:?}",
            disruption.id, err
        );
    }
    real_time_model.kirin_disruptions.push(disruption);
}

fn update_new_trip<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    vehicle_journey_id: &str,
    date: NaiveDate,
    update_data: &UpdateData,
    kirin_disruption_idx: KirinDisruptionIdx,
) -> Result<(), KirinUpdateError> {
    debug!("Kirin update on new trip {vehicle_journey_id}");
    if let Some(base_vj_idx) = base_model.vehicle_journey_idx(vehicle_journey_id) {
        if base_model.trip_exists(base_vj_idx, date) {
            return Err(KirinUpdateError::NewTripWithBaseId(
                VehicleJourneyId {
                    id: vehicle_journey_id.to_string(),
                },
                date,
            ));
        }
    }

    let new_vehicle_journey_idx = real_time_model.insert_new_vehicle_journey(vehicle_journey_id);

    let stop_times = real_time_model.make_stop_times(update_data.stop_times.as_slice(), base_model);

    let trip_version = TripVersion::Present(stop_times.clone());

    let has_previous_trip_version =
        real_time_model.set_new_trip_version(new_vehicle_journey_idx, &date, trip_version);

    let vehicle_journey_idx = VehicleJourneyIdx::New(new_vehicle_journey_idx);

    match has_previous_trip_version {
        Some(TripVersion::Present(_)) => {
            apply_disruption::modify_trip(
                real_time_model,
                base_model,
                data,
                &vehicle_journey_idx,
                &date,
                stop_times,
            );
        }
        Some(TripVersion::Deleted()) | None => {
            apply_disruption::add_trip(
                real_time_model,
                base_model,
                data,
                vehicle_journey_idx.clone(),
                date,
                stop_times,
            );
        }
    }

    real_time_model.set_linked_kirin_disruption(&vehicle_journey_idx, date, kirin_disruption_idx);

    Ok(())
}

fn update_base_trip<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    vehicle_journey_id: &str,
    date: NaiveDate,
    update_data: &UpdateData,
    kirin_disruption_idx: KirinDisruptionIdx,
) -> Result<(), KirinUpdateError> {
    debug!("Kirin update on base trip {vehicle_journey_id}");
    let base_vj_idx = base_model
        .vehicle_journey_idx(vehicle_journey_id)
        .ok_or_else(|| {
            KirinUpdateError::BaseVehicleJourneyUnknown(VehicleJourneyId {
                id: vehicle_journey_id.to_string(),
            })
        })?;

    let trip_exists_in_base = base_model.trip_exists(base_vj_idx, date);
    if !trip_exists_in_base {
        return Err(KirinUpdateError::BaseTripAbsent(
            VehicleJourneyId {
                id: vehicle_journey_id.to_string(),
            },
            date,
        ));
    }

    let vehicle_journey_idx = VehicleJourneyIdx::Base(base_vj_idx);

    let stop_times = real_time_model.make_stop_times(update_data.stop_times.as_slice(), base_model);

    let trip_version = TripVersion::Present(stop_times.clone());

    let has_previous_real_time_version =
        real_time_model.set_base_trip_version(base_vj_idx, &date, trip_version);

    match has_previous_real_time_version {
        Some(TripVersion::Deleted()) => {
            apply_disruption::add_trip(
                real_time_model,
                base_model,
                data,
                vehicle_journey_idx.clone(),
                date,
                stop_times,
            );
        }
        Some(TripVersion::Present(_))
        | None  // if None, it means we have no real time version of this base vehicle,
                // so the vehicle is present on the real time level in transit_data
                // as its real time version is the same as the base version
         => {
            apply_disruption::modify_trip(
                real_time_model,
                base_model,
                data,
                &vehicle_journey_idx,
                &date,
                stop_times,
            );
        }
    }

    real_time_model.set_linked_kirin_disruption(&vehicle_journey_idx, date, kirin_disruption_idx);

    Ok(())
}

fn delete_trip<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    vehicle_journey_id: &str,
    date: NaiveDate,
    kirin_disruption_idx: KirinDisruptionIdx,
) -> Result<(), KirinUpdateError> {
    debug!("Kirin delete trip {vehicle_journey_id}");

    let has_base_vj = base_model
        .vehicle_journey_idx(vehicle_journey_id)
        .and_then(|base_vj_idx| {
            if base_model.trip_exists(base_vj_idx, date) {
                Some(VehicleJourneyIdx::Base(base_vj_idx))
            } else {
                None
            }
        });

    let has_new_vj = real_time_model
        .new_vehicle_journey_idx(vehicle_journey_id)
        .and_then(|new_vj_idx| {
            if real_time_model.new_vehicle_journey_is_present(new_vj_idx, date) {
                Some(VehicleJourneyIdx::New(new_vj_idx))
            } else {
                None
            }
        });

    let vj_idx = match (has_base_vj, has_new_vj) {
        (Some(idx), _) => idx,
        (_, Some(idx)) => idx,
        _ => {
            return Err(KirinUpdateError::DeleteAbsentTrip(
                VehicleJourneyId {
                    id: vehicle_journey_id.to_string(),
                },
                date,
            ));
        }
    };

    apply_disruption::delete_trip(real_time_model, base_model, data, &vj_idx, date);

    real_time_model.set_linked_kirin_disruption(&vj_idx, date, kirin_disruption_idx);

    Ok(())
}
