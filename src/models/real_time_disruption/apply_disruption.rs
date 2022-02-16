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

use tracing::debug;

use crate::{
    chrono::NaiveDate,
    models::{self, real_time_model::TripVersion, RealTimeModel, VehicleJourneyIdx},
    transit_data::{
        data_interface::Data as DataTrait, handle_insertion_error, handle_modify_error,
        handle_removal_error,
    },
};

use crate::models::{base_model::BaseModel, ModelRefs};

use crate::DataUpdate;

pub(super) fn delete_trip<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    vehicle_journey_idx: &VehicleJourneyIdx,
    date: NaiveDate,
) {
    {
        let model_ref = ModelRefs {
            base: base_model,
            real_time: real_time_model,
        };
        debug!(
            "Deleting vehicle journey {} on {}",
            model_ref.vehicle_journey_name(vehicle_journey_idx),
            date
        );
    }

    let trip_version = TripVersion::Deleted();
    match vehicle_journey_idx {
        VehicleJourneyIdx::Base(base_idx) => {
            real_time_model.set_base_trip_version(*base_idx, &date, trip_version);
        }
        VehicleJourneyIdx::New(new_vj_idx) => {
            real_time_model.set_new_trip_version(*new_vj_idx, &date, trip_version);
        }
    }
    let removal_result = data.remove_real_time_vehicle(&vehicle_journey_idx, &date);
    if let Err(err) = removal_result {
        let model_ref = ModelRefs {
            base: base_model,
            real_time: real_time_model,
        };
        handle_removal_error(
            &model_ref,
            data.calendar().first_date(),
            data.calendar().last_date(),
            &err,
        );
    }
}

pub(super) fn add_trip<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    vehicle_journey_idx: VehicleJourneyIdx,
    date: &NaiveDate,
    stop_times: Vec<models::StopTime>,
) {
    {
        let model_ref = ModelRefs {
            base: base_model,
            real_time: real_time_model,
        };
        debug!(
            "Adding vehicle journey {} on {}",
            model_ref.vehicle_journey_name(&vehicle_journey_idx),
            date
        );
    }
    let dates = std::iter::once(*date);
    let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
    let flows = stop_times.iter().map(|stop_time| stop_time.flow_direction);
    let board_times = stop_times.iter().map(|stop_time| stop_time.board_time);
    let debark_times = stop_times.iter().map(|stop_time| stop_time.debark_time);
    let insert_result = data.insert_real_time_vehicle(
        stops,
        flows,
        board_times,
        debark_times,
        base_model.loads_data(),
        dates,
        &chrono_tz::UTC,
        vehicle_journey_idx,
    );
    let model_ref = ModelRefs {
        base: base_model,
        real_time: real_time_model,
    };
    if let Err(err) = insert_result {
        handle_insertion_error(
            &model_ref,
            data.calendar().first_date(),
            data.calendar().last_date(),
            &err,
        );
    }
}

pub fn modify_trip<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    vehicle_journey_idx: &VehicleJourneyIdx,
    date: &NaiveDate,
    stop_times: Vec<models::StopTime>,
) {
    {
        let model_ref = ModelRefs {
            base: base_model,
            real_time: real_time_model,
        };
        debug!(
            "Modifying vehicle journey {} on {}",
            model_ref.vehicle_journey_name(&vehicle_journey_idx),
            date
        );
    }
    let dates = std::iter::once(*date);
    let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
    let flows = stop_times.iter().map(|stop_time| stop_time.flow_direction);
    let board_times = stop_times.iter().map(|stop_time| stop_time.board_time);
    let debark_times = stop_times.iter().map(|stop_time| stop_time.debark_time);

    let modify_result = data.modify_real_time_vehicle(
        stops,
        flows,
        board_times,
        debark_times,
        base_model.loads_data(),
        dates,
        &chrono_tz::UTC,
        vehicle_journey_idx,
    );
    if let Err(err) = modify_result {
        let model_ref = ModelRefs {
            base: base_model,
            real_time: real_time_model,
        };
        handle_modify_error(
            &model_ref,
            data.calendar().first_date(),
            data.calendar().last_date(),
            &err,
        );
    }
}
