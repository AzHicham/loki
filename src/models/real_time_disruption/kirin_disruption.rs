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

    transit_data::{
        data_interface::Data as DataTrait,
        data_interface::DataUpdate,
    }, 
    models::{
        base_model::BaseModel, 
        real_time_model::{KirinDisruptionIdx, TripVersion}, 
        ModelRefs, 
        VehicleJourneyIdx}
        , 
};

use crate::{
    time::{SecondsSinceTimezonedDayStart},
    timetables::FlowDirection, models::RealTimeModel,
};
use chrono::{ NaiveDate, NaiveDateTime};
use tracing::{ error};


use std::{
    fmt::{Debug},
};

use super::{ apply_disruption, time_periods::TimePeriod, Effect, VehicleJourneyId};


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

    pub update : UpdateType,
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
pub enum KirinUpdateError{
    NewTripWithBaseId(VehicleJourneyId),
    BaseVehicleJourneyAbsent(VehicleJourneyId),
    DeleteAbsentTrip(VehicleJourneyId),
}


pub fn store_and_apply_kirin_disruption<Data: DataTrait + DataUpdate>(
    real_time_model : &mut RealTimeModel,
    disruption: KirinDisruption,
    base_model: &BaseModel,
    data: &mut Data,
) {
    let kirin_disruption_idx = KirinDisruptionIdx {
        idx : real_time_model.kirin_disruptions.len()
    };

    let date = disruption.trip_date;
    let vehicle_journey_id = disruption.trip_id.id.as_str();

    let result = match &disruption.update {
        UpdateType::TripDeleted() => delete_trip(real_time_model, base_model, data, vehicle_journey_id, date, kirin_disruption_idx),
        UpdateType::BaseTripUpdated(update_data) => update_base_trip(real_time_model, base_model, data, vehicle_journey_id, date, &update_data, kirin_disruption_idx),
        UpdateType::NewTripUpdated(update_data) => update_new_trip(real_time_model, base_model, data, vehicle_journey_id, date, &update_data, kirin_disruption_idx),
    };
    if let Err(err) = result {
        error!("Error while applying kirin disruption {}. {:?}", disruption.id, err);
    }

    real_time_model.kirin_disruptions.push(disruption);

}



fn update_new_trip<Data: DataTrait + DataUpdate>(
    real_time_model : &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    vehicle_journey_id : &str,
    date : NaiveDate,
    update_data: &UpdateData,
    kirin_disruption_idx : KirinDisruptionIdx
) -> Result<(), KirinUpdateError> {

    if base_model.vehicle_journey_idx(vehicle_journey_id).is_some() {
        return Err(KirinUpdateError::NewTripWithBaseId(
            VehicleJourneyId {
                id: vehicle_journey_id.to_string(),
            },
        ));
    }

    let new_vehicle_journey_idx = real_time_model.insert_new_vehicle_journey(vehicle_journey_id);

    let stop_times = real_time_model.make_stop_times(update_data.stop_times.as_slice(), base_model);

    let trip_version = TripVersion::Present(stop_times.clone());

    let has_previous_trip_version = real_time_model.set_new_trip_version(new_vehicle_journey_idx, &date, trip_version);

    let vehicle_journey_idx = VehicleJourneyIdx::New(new_vehicle_journey_idx);

    match has_previous_trip_version {
        Some(_) => {
            apply_disruption::modify_trip(
                real_time_model,
                base_model,
                data,
                &vehicle_journey_idx,
                &date,
                stop_times,
            );
        },
        None => {
            apply_disruption::add_trip(
                real_time_model,
                base_model,
                data,
                vehicle_journey_idx.clone(),
                &date,
                stop_times,
            );
        }
    }

    real_time_model.set_linked_kirin_disruption(&vehicle_journey_idx, date, kirin_disruption_idx);

    Ok(())
}

fn update_base_trip<Data: DataTrait + DataUpdate>(
    real_time_model : &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    vehicle_journey_id : &str,
    date : NaiveDate,
    update_data: &UpdateData,
    kirin_disruption_idx : KirinDisruptionIdx,
) -> Result<(), KirinUpdateError> {
    
    let base_vj_idx = base_model.vehicle_journey_idx(vehicle_journey_id).ok_or_else(|| 
        KirinUpdateError::BaseVehicleJourneyAbsent(VehicleJourneyId {
            id: vehicle_journey_id.to_string(),
        })
    )?;

    let vehicle_journey_idx = VehicleJourneyIdx::Base(base_vj_idx);

    let stop_times = real_time_model.make_stop_times(update_data.stop_times.as_slice(), base_model);

    let trip_version = TripVersion::Present(stop_times.clone());

    let has_previous_trip_version = real_time_model.set_base_trip_version(base_vj_idx, &date, trip_version);

    match has_previous_trip_version {
        Some(_) => {
            apply_disruption::modify_trip(
                real_time_model,
                base_model,
                data,
                &vehicle_journey_idx,
                &date,
                stop_times,
            );
        },
        None => {
            apply_disruption::add_trip(
                real_time_model,
                base_model,
                data,
                vehicle_journey_idx.clone(),
                &date,
                stop_times,
            );
        }
    }

    real_time_model.set_linked_kirin_disruption(&vehicle_journey_idx, date, kirin_disruption_idx);


    Ok(())

}

fn delete_trip<Data: DataTrait + DataUpdate>(
    real_time_model : &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    vehicle_journey_id : &str,
    date : NaiveDate,
    kirin_disruption_idx : KirinDisruptionIdx,
) -> Result<(), KirinUpdateError> {



    let vj_idx = {
        let model_refs = ModelRefs {
            base: base_model,
            real_time: real_time_model,
        };
        model_refs.vehicle_journey_idx(vehicle_journey_id)
            .ok_or_else(|| KirinUpdateError::DeleteAbsentTrip(VehicleJourneyId{id : vehicle_journey_id.to_string()}))
    }?;

    
    apply_disruption::delete_trip(real_time_model, base_model, data, &vj_idx, date);

    real_time_model.set_linked_kirin_disruption(&vj_idx, date, kirin_disruption_idx);

    Ok(())

}

// fn add_trip<Data: DataTrait + DataUpdate>(
//     real_time_model : &mut RealTimeModel,
//     base_model: &BaseModel,
//     data: &mut Data,
//     vehicle_journey_idx: VehicleJourneyIdx,
//     date: &NaiveDate,
//     stop_times: Vec<models::StopTime>,
// )  {

//     let dates = std::iter::once(*date);
//     let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
//     let flows = stop_times.iter().map(|stop_time| stop_time.flow_direction);
//     let board_times = stop_times.iter().map(|stop_time| stop_time.board_time);
//     let debark_times = stop_times.iter().map(|stop_time| stop_time.debark_time);
//     let insert_result = data.insert_real_time_vehicle(
//         stops,
//         flows,
//         board_times,
//         debark_times,
//         base_model.loads_data(),
//         dates,
//         &chrono_tz::UTC,
//         vehicle_journey_idx,
//     );
//     let model_ref = ModelRefs {
//         base: base_model,
//         real_time: real_time_model,
//     };
//     if let Err(err) = insert_result {
//         handle_insertion_error(
//             &model_ref,
//             data.calendar().first_date(),
//             data.calendar().last_date(),
//             &err,
//         );
//     }
// }

// fn modify_trip<Data: DataTrait + DataUpdate>(
//     real_time_model : &mut RealTimeModel,
//     base_model: &BaseModel,
//     data: &mut Data,
//     vehicle_journey_idx: &VehicleJourneyIdx,
//     date: &NaiveDate,
//     stop_times: Vec<models::StopTime>,
// ) {

//     let dates = std::iter::once(*date);
//     let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
//     let flows = stop_times.iter().map(|stop_time| stop_time.flow_direction);
//     let board_times = stop_times.iter().map(|stop_time| stop_time.board_time);
//     let debark_times = stop_times.iter().map(|stop_time| stop_time.debark_time);

//     let modify_result = data.modify_real_time_vehicle(
//         stops,
//         flows,
//         board_times,
//         debark_times,
//         base_model.loads_data(),
//         dates,
//         &chrono_tz::UTC,
//         vehicle_journey_idx,
//     );
//     if let Err(err) =  modify_result {
//         let model_ref = ModelRefs {
//             base: base_model,
//             real_time: real_time_model,
//         };
//         handle_modify_error(
//             &model_ref,
//             data.calendar().first_date(),
//             data.calendar().last_date(),
//             &err,
//         );
        
//     }
// }

