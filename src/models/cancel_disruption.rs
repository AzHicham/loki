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
    transit_data::{data_interface::Data as DataTrait, handle_insertion_error},
};

use tracing::{debug, error, warn};

use super::{
    real_time_disruption::{
        DisruptionError, Impacted, LineId, NetworkId, RouteId, StopAreaId, StopPointId,
        TimePeriods, VehicleJourneyId,
    },
    real_time_model::DisruptionIdx,
    RealTimeModel,
};
use crate::{
    models::{real_time_disruption::Informed, real_time_model::ImpactIdx, VehicleJourneyIdx},
    time::calendar,
    DataUpdate,
};

use super::{base_model::BaseModel, real_time_disruption as disruption, ModelRefs};

impl RealTimeModel {
    pub fn cancel_disruption_by_id<Data: DataTrait + DataUpdate>(
        &mut self,
        disruption_id: &str,
        base_model: &BaseModel,
        data: &mut Data,
    ) {
        let disruption_idx = self
            .disruptions
            .iter()
            .position(|disruption| disruption.id == disruption_id);

        if let Some(idx) = disruption_idx {
            let disruption = self.disruptions[idx].clone();
            let disruption_idx = DisruptionIdx { idx };
            for (idx, impact) in disruption.impacts.iter().enumerate() {
                let impact_idx = ImpactIdx { idx };
                self.cancel_impact(impact, base_model, data, disruption_idx, impact_idx);
            }
        } else {
            warn!("Cannot cancel disruption with id {}, as it was not found in realtime_model.disruptions",
                   disruption_id)
        }
    }

    fn cancel_impact<Data: DataTrait + DataUpdate>(
        &mut self,
        impact: &disruption::Impact,
        base_model: &BaseModel,
        data: &mut Data,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
    ) {
        let model_periods = [base_model.time_period()];
        let model_periods = TimePeriods::new(&model_periods).unwrap(); // unwrap is safe here, because the input slice is not empty

        let application_periods =
            TimePeriods::new(&impact.application_periods).unwrap_or(model_periods);

        for pt_object in &impact.impacted_pt_objects {
            let result = match pt_object {
                Impacted::BaseTripDeleted(trip) => self.cancel_impact_on_base_vehicle_journey(
                    base_model,
                    data,
                    &trip.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Impacted::NetworkDeleted(network) => self.cancel_impact_on_network(
                    base_model,
                    data,
                    &network.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Impacted::LineDeleted(line) => self.cancel_impact_on_line(
                    base_model,
                    data,
                    &line.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Impacted::RouteDeleted(route) => self.cancel_impact_on_route(
                    base_model,
                    data,
                    &route.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Impacted::StopPointDeleted(stop_point) => self.cancel_impact_on_stop_point(
                    base_model,
                    data,
                    &stop_point.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Impacted::StopAreaDeleted(stop_area) => self.cancel_impact_on_stop_area(
                    base_model,
                    data,
                    &stop_area.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Impacted::RailSection(_) => todo!(),
                Impacted::LineSection(_) => todo!(),
                Impacted::TripDeleted(_, _)
                | Impacted::NewTripUpdated(_)
                | Impacted::BaseTripUpdated(_) => Err(DisruptionError::CancelKirinDisruption),
            };
            if let Err(err) = result {
                error!("Error while cancelling impact {} : {:?}", impact.id, err);
            }
        }

        for pt_object in &impact.informed_pt_objects {
            let result = match pt_object {
                Informed::Network(network) => self.cancel_informed_on_network(
                    base_model,
                    &network.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Informed::Route(route) => self.cancel_informed_on_route(
                    base_model,
                    &route.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Informed::Line(line) => self.cancel_informed_on_line(
                    base_model,
                    &line.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Informed::Trip(trip) => self.cancel_informed_on_base_vehicle_journey(
                    base_model,
                    &trip.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Informed::StopArea(stop_area) => self.cancel_informed_on_stop_area(
                    base_model,
                    &stop_area.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Informed::StopPoint(stop_point) => self.cancel_informed_on_stop_point(
                    base_model,
                    &stop_point.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                ),
                Informed::Unknown => todo!(),
            };
            if let Err(err) = result {
                error!(
                    "Error while cancelling informed impact {} : {:?}",
                    impact.id, err
                );
            }
        }
    }

    fn cancel_impact_on_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
    ) -> Result<(), DisruptionError> {
        debug!(
            "Restore vehicle journey {} on day {}",
            vehicle_journey_id, date
        );
        let (vj_idx, stop_times) = self
            .restore_base_vehicle_journey(
                disruption_idx,
                impact_idx,
                vehicle_journey_id,
                date,
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
        if let Err(err) = result {
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
        Ok(())
    }

    fn cancel_impact_on_base_vehicle_journey<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if let Some(vehicle_journey_idx) = base_model.vehicle_journey_idx(vehicle_journey_id) {
            for date in application_periods.dates_possibly_concerned() {
                if let Some(trip_period) = base_model.trip_time_period(vehicle_journey_idx, &date) {
                    if application_periods.intersects(&trip_period) {
                        let result = self.cancel_impact_on_trip(
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
                                "Unexpected error while cancelling impact on a base vehicle journey {:?}",
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

    fn cancel_impact_on_network<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        network_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_network_id(network_id) {
            return Err(DisruptionError::NetworkAbsent(NetworkId {
                id: network_id.to_string(),
            }));
        }

        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.network_name(base_vehicle_journey_idx) == Some(network_id) {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.cancel_impact_on_base_vehicle_journey(
                    base_model,
                    data,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                    impact_idx,
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!(
                        "Unexpected error while cancelling impact on a network {:?}",
                        err
                    );
                }
            }
        }
        Ok(())
    }

    fn cancel_impact_on_line<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        line_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_line_id(line_id) {
            return Err(DisruptionError::LineAbsent(LineId {
                id: line_id.to_string(),
            }));
        }
        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.line_name(base_vehicle_journey_idx) == Some(line_id) {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.cancel_impact_on_base_vehicle_journey(
                    base_model,
                    data,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                    impact_idx,
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!(
                        "Unexpected error while cancelling impact on a line {:?}",
                        err
                    );
                }
            }
        }
        Ok(())
    }

    fn cancel_impact_on_route<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        route_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_route_id(route_id) {
            return Err(DisruptionError::RouteAbsent(RouteId {
                id: route_id.to_string(),
            }));
        }
        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.route_name(base_vehicle_journey_idx) == route_id {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.cancel_impact_on_base_vehicle_journey(
                    base_model,
                    data,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                    impact_idx,
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!(
                        "Unexpected error while cancelling impact on a route {:?}",
                        err
                    );
                }
            }
        }
        Ok(())
    }

    fn cancel_impact_on_stop_area<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        stop_area_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_stop_area_id(stop_area_id) {
            return Err(DisruptionError::StopAreaAbsent(StopAreaId {
                id: stop_area_id.to_string(),
            }));
        }
        for stop_point in base_model.stop_points() {
            let stop_area_of_stop_point = base_model.stop_area_name(stop_point);
            if stop_area_id == stop_area_of_stop_point {
                let stop_point_id = base_model.stop_point_id(stop_point);
                let result = self.cancel_impact_on_stop_point(
                    base_model,
                    data,
                    stop_point_id,
                    application_periods,
                    disruption_idx,
                    impact_idx,
                );
                if let Err(err) = result {
                    error!(
                        "Error while cancelling impact on a stop area {}. {:?}",
                        stop_area_id, err
                    );
                }
            }
        }
        Ok(())
    }

    fn cancel_impact_on_stop_point<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        stop_point_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
    ) -> Result<(), DisruptionError> {
        let stop_point_idx = self
            .stop_point_idx(stop_point_id, base_model)
            .ok_or_else(|| {
                DisruptionError::StopPointAbsent(StopPointId {
                    id: stop_point_id.to_string(),
                })
            })?;

        for vehicle_journey_idx in base_model.vehicle_journeys() {
            let vehicle_journey_id = base_model.vehicle_journey_name(vehicle_journey_idx);
            if let Ok(base_stop_times) = base_model.stop_times(vehicle_journey_idx) {
                let contains_stop_point = base_stop_times
                    .clone()
                    .any(|stop_time| stop_time.stop == stop_point_idx);
                if !contains_stop_point {
                    continue;
                }
                let timezone = base_model
                    .timezone(vehicle_journey_idx)
                    .unwrap_or(chrono_tz::UTC);
                for date in application_periods.dates_possibly_concerned() {
                    if let Some(time_period) =
                        base_model.trip_time_period(vehicle_journey_idx, &date)
                    {
                        if application_periods.intersects(&time_period) {
                            let is_stop_time_concerned = |stop_time: &super::StopTime| {
                                if stop_time.stop != stop_point_idx {
                                    return false;
                                }
                                let board_time =
                                    calendar::compose(&date, &stop_time.board_time, &timezone);
                                let debark_time =
                                    calendar::compose(&date, &stop_time.debark_time, &timezone);
                                application_periods.contains(&board_time)
                                    || application_periods.contains(&debark_time)
                            };
                            let is_trip_concerned = base_stop_times
                                .clone()
                                .any(|stop_time| is_stop_time_concerned(&stop_time));

                            if !is_trip_concerned {
                                continue;
                            }

                            let result = self.cancel_impact_on_trip(
                                base_model,
                                data,
                                vehicle_journey_id,
                                &date,
                                disruption_idx,
                                impact_idx,
                            );
                            if let Err(err) = result {
                                error!(
                                    "Error while cancelling impact on a stop point {}. {:?}",
                                    stop_point_id, err
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn cancel_informed_on_trip(
        &mut self,
        base_model: &BaseModel,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
    ) -> Result<(), DisruptionError> {
        debug!(
            "Restore vehicle journey {} on day {}",
            vehicle_journey_id, date
        );
        self.cancel_informed_linked_disruption(
            vehicle_journey_id,
            date,
            base_model,
            disruption_idx,
            impact_idx,
        );

        Ok(())
    }

    fn cancel_informed_on_base_vehicle_journey(
        &mut self,
        base_model: &BaseModel,
        vehicle_journey_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if let Some(vehicle_journey_idx) = base_model.vehicle_journey_idx(vehicle_journey_id) {
            for date in application_periods.dates_possibly_concerned() {
                if let Some(trip_period) = base_model.trip_time_period(vehicle_journey_idx, &date) {
                    if application_periods.intersects(&trip_period) {
                        let result = self.cancel_informed_on_trip(
                            base_model,
                            vehicle_journey_id,
                            &date,
                            disruption_idx,
                            impact_idx,
                        );
                        // we should never get a DeleteAbsentTrip error
                        // since we check in trip_time_period() that this trip exists
                        if let Err(err) = result {
                            error!(
                                "Unexpected error while cancelling impact on a base vehicle journey {:?}",
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

    fn cancel_informed_on_network(
        &mut self,
        base_model: &BaseModel,
        network_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_network_id(network_id) {
            return Err(DisruptionError::NetworkAbsent(NetworkId {
                id: network_id.to_string(),
            }));
        }

        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.network_name(base_vehicle_journey_idx) == Some(network_id) {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.cancel_informed_on_base_vehicle_journey(
                    base_model,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                    impact_idx,
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!(
                        "Unexpected error while cancelling impact on a route {:?}",
                        err
                    );
                }
            }
        }
        Ok(())
    }

    fn cancel_informed_on_line(
        &mut self,
        base_model: &BaseModel,
        line_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_line_id(line_id) {
            return Err(DisruptionError::LineAbsent(LineId {
                id: line_id.to_string(),
            }));
        }
        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.line_name(base_vehicle_journey_idx) == Some(line_id) {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.cancel_informed_on_base_vehicle_journey(
                    base_model,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                    impact_idx,
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!(
                        "Unexpected error while cancelling impact on a line {:?}",
                        err
                    );
                }
            }
        }
        Ok(())
    }

    fn cancel_informed_on_route(
        &mut self,
        base_model: &BaseModel,
        route_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_route_id(route_id) {
            return Err(DisruptionError::RouteAbsent(RouteId {
                id: route_id.to_string(),
            }));
        }
        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.route_name(base_vehicle_journey_idx) == route_id {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.cancel_informed_on_base_vehicle_journey(
                    base_model,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                    impact_idx,
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!(
                        "Unexpected error while cancelling impact on a route {:?}",
                        err
                    );
                }
            }
        }
        Ok(())
    }

    fn cancel_informed_on_stop_area(
        &mut self,
        base_model: &BaseModel,
        stop_area_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_stop_area_id(stop_area_id) {
            return Err(DisruptionError::StopAreaAbsent(StopAreaId {
                id: stop_area_id.to_string(),
            }));
        }
        for stop_point in base_model.stop_points() {
            let stop_area_of_stop_point = base_model.stop_area_name(stop_point);
            if stop_area_id == stop_area_of_stop_point {
                let stop_point_id = base_model.stop_point_id(stop_point);
                let result = self.cancel_informed_on_stop_point(
                    base_model,
                    stop_point_id,
                    application_periods,
                    disruption_idx,
                    impact_idx,
                );
                if let Err(err) = result {
                    error!(
                        "Error while restoring stop area {}. {:?}",
                        stop_area_id, err
                    );
                }
            }
        }
        Ok(())
    }

    fn cancel_informed_on_stop_point(
        &mut self,
        base_model: &BaseModel,
        stop_point_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
    ) -> Result<(), DisruptionError> {
        let stop_point_idx = self
            .stop_point_idx(stop_point_id, base_model)
            .ok_or_else(|| {
                DisruptionError::StopPointAbsent(StopPointId {
                    id: stop_point_id.to_string(),
                })
            })?;

        for vehicle_journey_idx in base_model.vehicle_journeys() {
            let vehicle_journey_id = base_model.vehicle_journey_name(vehicle_journey_idx);
            if let Ok(base_stop_times) = base_model.stop_times(vehicle_journey_idx) {
                let contains_stop_point = base_stop_times
                    .clone()
                    .any(|stop_time| stop_time.stop == stop_point_idx);
                if !contains_stop_point {
                    continue;
                }
                let timezone = base_model
                    .timezone(vehicle_journey_idx)
                    .unwrap_or(chrono_tz::UTC);
                for date in application_periods.dates_possibly_concerned() {
                    if let Some(time_period) =
                        base_model.trip_time_period(vehicle_journey_idx, &date)
                    {
                        if application_periods.intersects(&time_period) {
                            let is_stop_time_concerned = |stop_time: &super::StopTime| {
                                if stop_time.stop != stop_point_idx {
                                    return false;
                                }
                                let board_time =
                                    calendar::compose(&date, &stop_time.board_time, &timezone);
                                let debark_time =
                                    calendar::compose(&date, &stop_time.debark_time, &timezone);
                                application_periods.contains(&board_time)
                                    || application_periods.contains(&debark_time)
                            };
                            let is_trip_concerned = base_stop_times
                                .clone()
                                .any(|stop_time| is_stop_time_concerned(&stop_time));

                            if !is_trip_concerned {
                                continue;
                            }

                            let result = self.cancel_informed_on_trip(
                                base_model,
                                vehicle_journey_id,
                                &date,
                                disruption_idx,
                                impact_idx,
                            );
                            if let Err(err) = result {
                                error!(
                                    "Error while restoring stop point {}. {:?}",
                                    stop_point_id, err
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
