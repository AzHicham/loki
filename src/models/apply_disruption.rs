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
    chrono::NaiveDate,
    transit_data::{
        data_interface::Data as DataTrait, handle_insertion_error, handle_modify_error,
        handle_removal_error,
    },
    models::{real_time_model::UpdateError, VehicleJourneyIdx}
};

use tracing::{debug, error, trace, warn};

use super::{
    real_time_disruption::{
        TimePeriods,  VehicleJourneyId,
    },

    RealTimeModel,
};
use crate::models::real_time_disruption::intersection;
use crate::{

    DataUpdate,
};

use super::{base_model::BaseModel, real_time_disruption as disruption, ModelRefs};


impl RealTimeModel {
    

    //----------------------------------------------------------------------------------------
    // elementary functions operating on trips (VJ + date)
    // Used for chaos and kirin
    fn add_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        stop_times: Vec<super::StopTime>,
    ) -> Result<(), UpdateError> {
        debug!(
            "Adding a new vehicle journey {} on date {}",
            vehicle_journey_id, date
        );
        let (vj_idx, stop_times) = self
            .add(vehicle_journey_id, date, stop_times, base_model)
            .map_err(|_| {
                UpdateError::AddPresentTrip(
                    VehicleJourneyId {
                        id: vehicle_journey_id.to_string(),
                    },
                    *date,
                )
            })?;
        trace!(
            "New vehicle journey {} on date {} stored in real time model. Stop times : {:#?} ",
            vehicle_journey_id,
            date,
            stop_times
        );
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
            vj_idx,
        );
        let model_ref = ModelRefs {
            base: base_model,
            real_time: self,
        };
        if let Err(err) = insert_result {
            handle_insertion_error(
                &model_ref,
                data.calendar().first_date(),
                data.calendar().last_date(),
                &err,
            );
        }

        Ok(())
    }

    pub fn modify_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        stop_times: Vec<super::StopTime>,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), UpdateError> {
        debug!(
            "Modifying vehicle journey {} on date {}",
            vehicle_journey_id, date
        );
        let (vj_idx, stop_times) = self
            .modify(vehicle_journey_id, date, stop_times, base_model)
            .map_err(|_| {
                UpdateError::ModifyAbsentTrip(
                    VehicleJourneyId {
                        id: vehicle_journey_id.to_string(),
                    },
                    *date,
                )
            })?;
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
            &vj_idx,
        );
        match modify_result {
            Ok(_) => self.insert_informed_linked_disruption(
                vehicle_journey_id,
                date,
                base_model,
                *disruption_idx,
                *impact_idx,
            ),
            Err(err) => {
                let model_ref = ModelRefs {
                    base: base_model,
                    real_time: self,
                };
                handle_modify_error(
                    &model_ref,
                    data.calendar().first_date(),
                    data.calendar().last_date(),
                    &err,
                );
            }
        }
        Ok(())
    }

    fn delete_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), UpdateError> {
        debug!(
            "Deleting vehicle journey {} on day {}",
            vehicle_journey_id, date
        );
        let vj_idx = self
            .delete(vehicle_journey_id, date, base_model)
            .map_err(|_| {
                UpdateError::DeleteAbsentTrip(
                    VehicleJourneyId {
                        id: vehicle_journey_id.to_string(),
                    },
                    *date,
                )
            })?;
        let removal_result = data.remove_real_time_vehicle(&vj_idx, date);
        match removal_result {
            Ok(_) => self.insert_informed_linked_disruption(
                vehicle_journey_id,
                date,
                base_model,
                *disruption_idx,
                *impact_idx,
            ),
            Err(removal_error) => {
                let model_ref = ModelRefs {
                    base: base_model,
                    real_time: self,
                };
                handle_removal_error(
                    &model_ref,
                    data.calendar().first_date(),
                    data.calendar().last_date(),
                    &removal_error,
                );
            }
        }
        Ok(())
    }

    fn restore_base_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), UpdateError> {
        debug!(
            "Restore vehicle journey {} on day {}",
            vehicle_journey_id, date
        );
        let (vj_idx, stop_times) = self
            .restore_base_vehicle_journey(vehicle_journey_id, date, base_model)
            .map_err(|_| {
                UpdateError::ModifyAbsentTrip(
                    VehicleJourneyId {
                        id: vehicle_journey_id.to_string(),
                    },
                    *date,
                )
            })?;

        let dates = std::iter::once(*date);
        let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
        let flows = stop_times.iter().map(|stop_time| stop_time.flow_direction);
        let board_times = stop_times.iter().map(|stop_time| stop_time.board_time);
        let debark_times = stop_times.iter().map(|stop_time| stop_time.debark_time);

        let result = data.insert_real_time_vehicle(
            stops,
            flows,
            board_times,
            debark_times,
            base_model.loads_data(),
            dates,
            &chrono_tz::UTC,
            VehicleJourneyIdx::Base(vj_idx),
        );
        match result {
            Ok(_) => self.cancel_informed_linked_disruption(
                vehicle_journey_id,
                date,
                base_model,
                *disruption_idx,
                *impact_idx,
            ),
            Err(err) => {
                let model_ref = ModelRefs {
                    base: base_model,
                    real_time: self,
                };
                handle_insertion_error(
                    &model_ref,
                    data.calendar().first_date(),
                    data.calendar().last_date(),
                    &err,
                );
            }
        }
        Ok(())
    }

}
