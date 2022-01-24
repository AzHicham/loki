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
};
use tracing::{debug, error, trace};

use super::{
    real_time_disruption::{
        intersection, DateTimePeriod, DisruptionError, Impacted, LineId, NetworkId, RouteId,
        StopTime, TripDisruption, VehicleJourneyId,
    },
    real_time_model::{DisruptionIdx, Trip, TripData, TripVersion, UpdateError},
    RealTimeModel,
};
use crate::DataUpdate;

use super::{base_model::BaseModel, real_time_disruption as disruption, ModelRefs};

impl RealTimeModel {
    pub fn store_and_apply_disruption<Data: DataTrait + DataUpdate>(
        &mut self,
        disruption: disruption::Disruption,
        base_model: &BaseModel,
        data: &mut Data,
    ) {
        let disruption_idx = DisruptionIdx {
            idx: self.disruptions.len(),
        };

        for impact in &disruption.impacts {
            self.apply_impact(impact, base_model, data, &disruption_idx);
        }

        self.disruptions.push(disruption);
    }

    fn apply_impact<Data: DataTrait + DataUpdate>(
        &mut self,
        impact: &disruption::Impact,
        base_model: &BaseModel,
        data: &mut Data,
        disruption_idx: &DisruptionIdx,
    ) -> Result<(), UpdateError> {
        let validity_period = base_model.validity_period();
        let calendar_period = DateTimePeriod::new(
            validity_period.0.and_hms(0, 0, 0),
            validity_period.1.and_hms(12, 59, 59),
        );

        if let Ok(calendar_period) = calendar_period {
            let application_periods: Vec<DateTimePeriod> = impact
                .application_periods
                .iter()
                .filter_map(|ap| intersection(ap, &calendar_period))
                .collect();

            for pt_object in &impact.impacted_pt_objects {
                match pt_object {
                    Impacted::NetworkDeleted(network) => self.delete_network(
                        base_model,
                        data,
                        &network.id,
                        &application_periods,
                        disruption_idx,
                    ),
                    Impacted::LineDeleted(line) => self.delete_line(
                        base_model,
                        data,
                        &line.id,
                        &application_periods,
                        disruption_idx,
                    ),
                    Impacted::RouteDeleted(route) => self.delete_route(
                        base_model,
                        data,
                        &route.id,
                        &application_periods,
                        disruption_idx,
                    ),
                    Impacted::TripDeleted(trip) => self.delete_vehicle_journey(
                        base_model,
                        data,
                        &trip.id,
                        &application_periods,
                        &disruption_idx,
                    ),
                    Impacted::BaseTripUpdated(trip_disruption) => self.update_base_trip(
                        base_model,
                        data,
                        trip_disruption,
                        &application_periods,
                        disruption_idx,
                    ),
                    Impacted::NewTripUpdated(trip_disruption) => self.update_new_trip(
                        base_model,
                        data,
                        trip_disruption,
                        &application_periods,
                        disruption_idx,
                    ),
                    Impacted::RailSection(_) => todo!(),
                    Impacted::LineSection(_) => todo!(),
                    Impacted::StopAreaDeleted(_) => todo!(),
                    Impacted::StopPointDeleted(_) => todo!(),
                };
            }
        }
        Ok(())
    }

    fn update_new_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        trip_disruption: &TripDisruption,
        application_periods: &[DateTimePeriod],
        disruption_idx: &DisruptionIdx,
    ) -> Result<(), DisruptionError> {
        let vehicle_journey_id = &trip_disruption.trip_id.id;
        let stop_times = &trip_disruption.stop_times;

        let has_base_vj_idx = base_model.vehicle_journey_idx(vehicle_journey_id);

        for application_period in application_periods.iter() {
            for datetime in application_period {
                let date = datetime.date();
                let trip_exists_in_base = {
                    match has_base_vj_idx {
                        None => false,
                        Some(vj_idx) => base_model.trip_exists(vj_idx, date),
                    }
                };
                if trip_exists_in_base {
                    error!("Cannot apply UpdateNewTrip disruption for vehicle journey {} on {} since a base vehicle journey exists on this date.", vehicle_journey_id, date);
                    continue;
                }

                if self.is_present(vehicle_journey_id, &date, base_model) {
                    self.modify_trip(
                        base_model,
                        data,
                        vehicle_journey_id,
                        &date,
                        stop_times,
                        *disruption_idx,
                    );
                } else {
                    self.add_trip(
                        base_model,
                        data,
                        vehicle_journey_id,
                        &date,
                        stop_times,
                        *disruption_idx,
                    );
                }
            }
        }
        Ok(())
    }

    fn update_base_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        trip_disruption: &TripDisruption,
        application_periods: &[DateTimePeriod],
        disruption_idx: &DisruptionIdx,
    ) -> Result<(), DisruptionError> {
        let vehicle_journey_id = &trip_disruption.trip_id.id;
        let stop_times = &trip_disruption.stop_times;

        if let Some(base_vj_idx) = base_model.vehicle_journey_idx(vehicle_journey_id) {
            for application_period in application_periods.iter() {
                for datetime in application_period {
                    let date = datetime.date();

                    let trip_exists_in_base = base_model.trip_exists(base_vj_idx, date);

                    if !trip_exists_in_base {
                        error!("Cannot apply UpdateBaseTrip disruption since the base vehicle journey {} is not valid on {}", vehicle_journey_id, date);
                        continue;
                    }

                    if self.is_present(vehicle_journey_id, &date, base_model) {
                        self.modify_trip(
                            base_model,
                            data,
                            vehicle_journey_id,
                            &date,
                            stop_times,
                            *disruption_idx,
                        );
                    } else {
                        self.add_trip(
                            base_model,
                            data,
                            vehicle_journey_id,
                            &date,
                            stop_times,
                            *disruption_idx,
                        );
                    }
                }
            }
            Ok(())
        } else {
            return Err(DisruptionError::VehicleJourneyAbsent(VehicleJourneyId {
                id: vehicle_journey_id.clone(),
            }));
        }
    }

    fn delete_network<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        network_id: &str,
        application_periods: &[DateTimePeriod],
        disruption_idx: &DisruptionIdx,
    ) -> Result<(), DisruptionError> {
        if base_model.contains_network_id(network_id) {
            return Err(DisruptionError::NetworkAbsent(NetworkId {
                id: network_id.to_string(),
            }));
        }

        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.network_name(base_vehicle_journey_idx) == Some(network_id) {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                self.delete_vehicle_journey(
                    base_model,
                    data,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                );
            }
        }
        // TODO : new vehicle journeys may also belong to the network to be deleted
        Ok(())
    }

    fn delete_line<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        line_id: &str,
        application_periods: &[DateTimePeriod],
        disruption_idx: &DisruptionIdx,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_line_id(line_id) {
            return Err(DisruptionError::LineAbsent(LineId {
                id: line_id.to_string(),
            }));
        }
        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.line_name(base_vehicle_journey_idx) == Some(line_id) {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                self.delete_vehicle_journey(
                    base_model,
                    data,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                );
            }
        }
        // TODO : new vehicle journeys may also belong to the line to be deleted
        Ok(())
    }

    fn delete_route<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        route_id: &str,
        application_periods: &[DateTimePeriod],
        disruption_idx: &DisruptionIdx,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_route_id(route_id) {
            return Err(DisruptionError::RouteAbsent(RouteId {
                id: route_id.to_string(),
            }));
        }
        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.route_name(base_vehicle_journey_idx) == route_id {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                self.delete_vehicle_journey(
                    base_model,
                    data,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                );
            }
        }
        // TODO : new vehicle journeys may also belong to the route to be deleted
        Ok(())
    }

    fn delete_vehicle_journey<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        application_periods: &[DateTimePeriod],
        disruption_idx: &DisruptionIdx,
    ) -> Result<(), DisruptionError> {
        if let Some(_) = self.vehicle_journey_idx(vehicle_journey_id, base_model) {
            for application_period in application_periods.iter() {
                for datetime in application_period {
                    let date = datetime.date();
                    self.delete_trip(base_model, data, vehicle_journey_id, &date, disruption_idx);
                }
            }
            Ok(())
        } else {
            Err(DisruptionError::VehicleJourneyAbsent(VehicleJourneyId {
                id: vehicle_journey_id.to_string(),
            }))
        }
    }

    fn delete_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        disruption_idx: &DisruptionIdx,
    ) -> Result<(), UpdateError> {
        debug!(
            "Deleting vehicle journey {} on day {}",
            vehicle_journey_id, date
        );
        let vj_idx = self.delete(*disruption_idx, vehicle_journey_id, date, base_model)?;
        let removal_result = data.remove_real_time_vehicle(&vj_idx, &date);
        if let Err(removal_error) = removal_result {
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
        Ok(())
    }

    pub fn modify_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        stop_times: &[StopTime],
        disruption_idx: DisruptionIdx,
    ) -> Result<(), UpdateError> {
        debug!(
            "Modifying vehicle journey {} on date {}",
            vehicle_journey_id, date
        );
        let (vj_idx, stop_times) = self.modify(
            disruption_idx,
            vehicle_journey_id,
            date,
            stop_times,
            base_model,
        )?;
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

        if let Err(err) = modify_result {
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
        Ok(())
    }

    fn add_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        stop_times: &[StopTime],
        disruption_idx: DisruptionIdx,
    ) -> Result<(), UpdateError> {
        debug!(
            "Adding a new vehicle journey {} on date {}",
            vehicle_journey_id, date
        );
        let (vj_idx, stop_times) = self.add(
            disruption_idx,
            vehicle_journey_id,
            date,
            stop_times,
            base_model,
        )?;
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
}
