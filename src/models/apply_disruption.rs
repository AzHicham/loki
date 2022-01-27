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
        DisruptionError, Impacted, LineId, NetworkId, RouteId, StopTime, TimePeriods,
        TripDisruption, VehicleJourneyId,
    },
    real_time_model::DisruptionIdx,
    RealTimeModel,
};
use crate::{
    models::{real_time_disruption::Informed, real_time_model::ImpactIdx},
    DataUpdate,
};

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

        for (idx, impact) in disruption.impacts.iter().enumerate() {
            let impact_idx = ImpactIdx { idx };
            self.apply_impact(impact, base_model, data, &disruption_idx, &impact_idx);
        }

        self.disruptions.push(disruption);
    }

    fn apply_impact<Data: DataTrait + DataUpdate>(
        &mut self,
        impact: &disruption::Impact,
        base_model: &BaseModel,
        data: &mut Data,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) {
        let model_periods = [base_model.time_period()];
        let model_periods = TimePeriods::new(&model_periods).unwrap(); // unwrap is safe here, because the input slice is not empty

        let application_periods =
            TimePeriods::new(&impact.application_periods).unwrap_or(model_periods);

        for pt_object in &impact.impacted_pt_objects {
            let result = match pt_object {
                Impacted::NetworkDeleted(network) => self.delete_network(
                    base_model,
                    data,
                    &network.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Impacted::LineDeleted(line) => self.delete_line(
                    base_model,
                    data,
                    &line.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Impacted::RouteDeleted(route) => self.delete_route(
                    base_model,
                    data,
                    &route.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Impacted::BaseTripDeleted(trip) => self.delete_base_vehicle_journey(
                    base_model,
                    data,
                    &trip.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Impacted::TripDeleted(vehicle_journey_id, date) => self.delete_trip(
                    base_model,
                    data,
                    &vehicle_journey_id.id,
                    date,
                    disruption_idx,
                    impact_idx,
                ),
                Impacted::BaseTripUpdated(trip_disruption) => self.update_base_trip(
                    base_model,
                    data,
                    trip_disruption,
                    disruption_idx,
                    impact_idx,
                ),
                Impacted::NewTripUpdated(trip_disruption) => self.update_new_trip(
                    base_model,
                    data,
                    trip_disruption,
                    disruption_idx,
                    impact_idx,
                ),
                Impacted::RailSection(_) => todo!(),
                Impacted::LineSection(_) => todo!(),
                Impacted::StopAreaDeleted(_) => todo!(),
                Impacted::StopPointDeleted(_) => todo!(),
            };
            if let Err(err) = result {
                error!("Error while applying impact {} : {:?}", impact.id, err);
            }
        }

        for pt_object in &impact.informed_pt_objects {
            let result = match pt_object {
                Informed::Network(network) => self.informed_network(
                    base_model,
                    &network.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Informed::Route(route) => self.informed_route(
                    base_model,
                    &route.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Informed::Line(line) => self.informed_line(
                    base_model,
                    &line.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Informed::Trip(trip) => self.informed_base_vehicle_journey(
                    base_model,
                    &trip.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Informed::StopArea(_) => todo!(),
                Informed::StopPoint(_) => todo!(),
                Informed::Unknown => todo!(),
            };
            if let Err(err) = result {
                error!(
                    "Error while storing informed impact {} : {:?}",
                    impact.id, err
                );
            }
        }
    }

    fn update_new_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        trip_disruption: &TripDisruption,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), DisruptionError> {
        let vehicle_journey_id = &trip_disruption.trip_id.id;
        let stop_times = &trip_disruption.stop_times;

        let has_base_vj_idx = base_model.vehicle_journey_idx(vehicle_journey_id);
        let date = trip_disruption.trip_date;
        let trip_exists_in_base = {
            match has_base_vj_idx {
                None => false,
                Some(vj_idx) => base_model.trip_exists(vj_idx, date),
            }
        };

        if trip_exists_in_base {
            return Err(DisruptionError::NewTripWithBaseId(
                VehicleJourneyId {
                    id: vehicle_journey_id.to_string(),
                },
                date,
            ));
        }

        if self.is_present(vehicle_journey_id, &date, base_model) {
            self.modify_trip(
                base_model,
                data,
                vehicle_journey_id,
                &date,
                stop_times,
                *disruption_idx,
                *impact_idx,
            )
        } else {
            self.add_trip(
                base_model,
                data,
                vehicle_journey_id,
                &date,
                stop_times,
                *disruption_idx,
                *impact_idx,
            )
        }
    }

    fn update_base_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        trip_disruption: &TripDisruption,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), DisruptionError> {
        let vehicle_journey_id = &trip_disruption.trip_id.id;
        let stop_times = &trip_disruption.stop_times;

        if let Some(base_vj_idx) = base_model.vehicle_journey_idx(vehicle_journey_id) {
            let date = trip_disruption.trip_date;
            let trip_exists_in_base = base_model.trip_exists(base_vj_idx, date);

            if !trip_exists_in_base {
                return Err(DisruptionError::ModifyAbsentTrip(
                    VehicleJourneyId {
                        id: vehicle_journey_id.to_string(),
                    },
                    date,
                ));
            }

            if self.is_present(vehicle_journey_id, &date, base_model) {
                self.modify_trip(
                    base_model,
                    data,
                    vehicle_journey_id,
                    &date,
                    stop_times,
                    *disruption_idx,
                    *impact_idx,
                )
            } else {
                self.add_trip(
                    base_model,
                    data,
                    vehicle_journey_id,
                    &date,
                    stop_times,
                    *disruption_idx,
                    *impact_idx,
                )
            }
        } else {
            Err(DisruptionError::VehicleJourneyAbsent(VehicleJourneyId {
                id: vehicle_journey_id.clone(),
            }))
        }
    }

    fn delete_network<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        network_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if base_model.contains_network_id(network_id) {
            return Err(DisruptionError::NetworkAbsent(NetworkId {
                id: network_id.to_string(),
            }));
        }

        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.network_name(base_vehicle_journey_idx) == Some(network_id) {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.delete_base_vehicle_journey(
                    base_model,
                    data,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                    impact_idx,
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!("Unexpected error while deleting a route {:?}", err);
                }
            }
        }
        Ok(())
    }

    fn delete_line<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        line_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_line_id(line_id) {
            return Err(DisruptionError::LineAbsent(LineId {
                id: line_id.to_string(),
            }));
        }
        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.line_name(base_vehicle_journey_idx) == Some(line_id) {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.delete_base_vehicle_journey(
                    base_model,
                    data,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                    impact_idx,
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!("Unexpected error while deleting a line {:?}", err);
                }
            }
        }
        Ok(())
    }

    fn delete_route<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        route_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_route_id(route_id) {
            return Err(DisruptionError::RouteAbsent(RouteId {
                id: route_id.to_string(),
            }));
        }
        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.route_name(base_vehicle_journey_idx) == route_id {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.delete_base_vehicle_journey(
                    base_model,
                    data,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                    impact_idx,
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!("Unexpected error while deleting a route {:?}", err);
                }
            }
        }
        Ok(())
    }

    fn delete_base_vehicle_journey<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if let Some(vehicle_journey_idx) = base_model.vehicle_journey_idx(vehicle_journey_id) {
            for date in application_periods.dates_possibly_concerned() {
                if let Some(trip_period) = base_model.trip_time_period(vehicle_journey_idx, &date) {
                    if application_periods.intersects(&trip_period) {
                        let result = self.delete_trip(
                            base_model,
                            data,
                            vehicle_journey_id,
                            &date,
                            disruption_idx,
                            impact_idx,
                        );
                        // we should never get a DeleteAbsentTrip error
                        // since we check in trip_time_period() that this trip exists
                        if let Err(err) = result {
                            error!(
                                "Unexpected error while deleting a base vehicle journey {:?}",
                                err
                            );
                        }
                    }
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
        impact_idx: &ImpactIdx,
    ) -> Result<(), DisruptionError> {
        debug!(
            "Deleting vehicle journey {} on day {}",
            vehicle_journey_id, date
        );
        let vj_idx = self
            .delete(
                *disruption_idx,
                *impact_idx,
                vehicle_journey_id,
                date,
                base_model,
            )
            .map_err(|_| {
                DisruptionError::DeleteAbsentTrip(
                    VehicleJourneyId {
                        id: vehicle_journey_id.to_string(),
                    },
                    *date,
                )
            })?;
        let removal_result = data.remove_real_time_vehicle(&vj_idx, date);
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
        impact_idx: ImpactIdx,
    ) -> Result<(), DisruptionError> {
        debug!(
            "Modifying vehicle journey {} on date {}",
            vehicle_journey_id, date
        );
        let (vj_idx, stop_times) = self
            .modify(
                disruption_idx,
                impact_idx,
                vehicle_journey_id,
                date,
                stop_times,
                base_model,
            )
            .map_err(|_| {
                DisruptionError::ModifyAbsentTrip(
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
        impact_idx: ImpactIdx,
    ) -> Result<(), DisruptionError> {
        debug!(
            "Adding a new vehicle journey {} on date {}",
            vehicle_journey_id, date
        );
        let (vj_idx, stop_times) = self
            .add(
                disruption_idx,
                impact_idx,
                vehicle_journey_id,
                date,
                stop_times,
                base_model,
            )
            .map_err(|_| {
                DisruptionError::AddPresentTrip(
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

    fn informed_network(
        &mut self,
        base_model: &BaseModel,
        network_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if base_model.contains_network_id(network_id) {
            return Err(DisruptionError::NetworkAbsent(NetworkId {
                id: network_id.to_string(),
            }));
        }

        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.network_name(base_vehicle_journey_idx) == Some(network_id) {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.informed_base_vehicle_journey(
                    base_model,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                    impact_idx,
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!("Unexpected error while deleting a route {:?}", err);
                }
            }
        }
        Ok(())
    }

    fn informed_line(
        &mut self,
        base_model: &BaseModel,
        line_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_line_id(line_id) {
            return Err(DisruptionError::LineAbsent(LineId {
                id: line_id.to_string(),
            }));
        }
        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.line_name(base_vehicle_journey_idx) == Some(line_id) {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.informed_base_vehicle_journey(
                    base_model,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                    impact_idx,
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!("Unexpected error while deleting a line {:?}", err);
                }
            }
        }
        Ok(())
    }

    fn informed_route(
        &mut self,
        base_model: &BaseModel,
        route_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_route_id(route_id) {
            return Err(DisruptionError::RouteAbsent(RouteId {
                id: route_id.to_string(),
            }));
        }
        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.route_name(base_vehicle_journey_idx) == route_id {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.informed_base_vehicle_journey(
                    base_model,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                    impact_idx,
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!("Unexpected error while deleting a route {:?}", err);
                }
            }
        }
        Ok(())
    }

    fn informed_base_vehicle_journey(
        &mut self,
        base_model: &BaseModel,
        vehicle_journey_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if let Some(vehicle_journey_idx) = base_model.vehicle_journey_idx(vehicle_journey_id) {
            for date in application_periods.dates_possibly_concerned() {
                if let Some(trip_period) = base_model.trip_time_period(vehicle_journey_idx, &date) {
                    if application_periods.intersects(&trip_period) {
                        self.insert_informed_linked_disruption(
                            vehicle_journey_id,
                            &date,
                            base_model,
                            *disruption_idx,
                            *impact_idx,
                        );
                    }
                }
            }
            Ok(())
        } else {
            Err(DisruptionError::VehicleJourneyAbsent(VehicleJourneyId {
                id: vehicle_journey_id.to_string(),
            }))
        }
    }
}
