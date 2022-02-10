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
    }, models::{base_model::{BaseModel, BaseVehicleJourneyIdx}, real_time_model::ChaosImpactIdx, RealTimeModel, StopPointIdx}, time::calendar,
};


use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::Deserialize;
use tracing::error;


use super::{TimePeriod, Effect, intersection, TimePeriods};

#[derive(Debug, Clone)]
pub struct ChaosDisruption {
    pub id: String,
    pub reference: Option<String>,
    pub contributor: Option<String>,
    pub publication_period: TimePeriod,
    pub cause: Cause,
    pub tags: Vec<Tag>,
    pub properties: Vec<DisruptionProperty>,
    pub impacts: Vec<ChaosImpact>,
}

#[derive(Debug, Clone)]
pub struct ChaosImpact {
    pub id: String,
    pub updated_at: NaiveDateTime,
    pub application_periods: Vec<TimePeriod>,
    pub application_patterns: Vec<ApplicationPattern>,
    pub severity: Severity,
    pub messages: Vec<Message>,
    pub impacted_pt_objects: Vec<Impacted>,
    pub informed_pt_objects: Vec<Informed>,
}

#[derive(Debug, Clone)]
pub enum Impacted {
    NetworkDeleted(NetworkId),
    LineDeleted(LineId),
    RouteDeleted(RouteId),

    RailSection(RailSection),
    LineSection(LineSection),
    StopAreaDeleted(StopAreaId),
    StopPointDeleted(StopPointId),
    BaseTripDeleted(VehicleJourneyId),
}

#[derive(Debug, Clone)]
pub enum Informed {
    Network(NetworkId),
    Line(LineId),
    Route(RouteId),
    Trip(VehicleJourneyId),
    StopArea(StopAreaId),
    StopPoint(StopPointId),
}

#[derive(Debug, Clone)]
pub struct NetworkId {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct LineId {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct RouteId {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct StopPointId {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct StopAreaId {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct VehicleJourneyId {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct LineSection {
    pub line: LineId,
    pub start: StopAreaId,
    pub end: StopAreaId,
    pub routes: Vec<RouteId>,
}

#[derive(Debug, Clone)]
pub struct RailSection {
    pub line: LineId,
    pub start: StopAreaId,
    pub end: StopAreaId,
    pub routes: Vec<RouteId>,
    pub blocked_stop_area: Vec<BlockedStopArea>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BlockedStopArea {
    pub id: String,
    pub order: u32,
}


#[derive(Debug, Clone)]
pub enum ChannelType {
    Web,
    Sms,
    Email,
    Mobile,
    Notification,
    Twitter,
    Facebook,
    UnknownType,
    Title,
    Beacon,
}


#[derive(Default, Debug, Clone)]
pub struct Cause {
    pub wording: String,
    pub category: String,
}

#[derive(Default, Debug, Clone)]
pub struct DisruptionProperty {
    pub key: String,
    pub type_: String,
    pub value: String,
}

#[derive(Default, Debug, Clone)]
pub struct Tag {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Severity {
    pub wording: Option<String>,
    pub color: Option<String>,
    pub priority: Option<i32>,
    pub effect: Effect,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub text: String,
    pub channel_id: Option<String>,
    pub channel_name: String,
    pub channel_content_type: Option<String>,
    pub channel_types: Vec<ChannelType>,
}

#[derive(Debug, Clone)]
pub struct ApplicationPattern {
    pub begin_date: NaiveDate,
    pub end_date: NaiveDate,
    pub time_slots: Vec<TimeSlot>,
    pub week_pattern: [bool; 7],
}

#[derive(Debug, Clone)]
pub struct TimeSlot {
    //TODO : determine in which timezone are these ?
    // can we use SecondsTimezoneDayStart/SecondsSinceUtcDayStart ?
    pub begin: NaiveTime,
    pub end: NaiveTime,
}

pub enum Action {
    Alter,
    Inform,
    CancelAlteration,
    CancelInform,
}

#[derive(Debug, Clone)]
pub enum ChaosImpactError {
    VehicleJourneyAbsent(VehicleJourneyId),
    NetworkAbsent(NetworkId),
}

impl RealTimeModel {




    pub fn store_and_apply_disruption<Data: DataTrait + DataUpdate>(
        &mut self,
        disruption: ChaosDisruption,
        base_model: &BaseModel,
        data: &mut Data,
    ) {
        let disruption_idx =  self.disruptions.len();

        for (idx, impact) in disruption.impacts.iter().enumerate() {
            let impact_idx = ChaosImpactIdx { 
                disruption_idx, 
                impact_idx : idx
            };
            self.apply_impact(impact, base_model, data, &disruption_idx, &impact_idx);
        }

        self.disruptions.push(disruption);
    }

    fn apply_impact<Data: DataTrait + DataUpdate>(
        &mut self,
        impact: &ChaosImpact,
        base_model: &BaseModel,
        data: &mut Data,
        impact_idx: &ChaosImpactIdx,
        cancel_impact : bool,
    ) {
        let model_period = [base_model.time_period()];
        // filter application_periods by model_period
        // by taking the intersection of theses two TimePeriodsapplication_periods
        let application_periods: Vec<_> = impact
            .application_periods
            .iter()
            .filter_map(|application_periods| intersection(application_periods, &model_period[0]))
            .collect();

        if application_periods.is_empty() {
            return;
        }
        // unwrap is sfe here because we checked if application_periods is empty or not
        let application_periods = TimePeriods::new(&application_periods).unwrap();

        let impact_action = if cancel_impact {
            Action::CancelAlteration
        }
        else {
            Action::Alter
        };

        for pt_object in &impact.impacted_pt_objects {
            let result = match pt_object {
                Impacted::NetworkDeleted(network) => self.apply_on_network(
                    base_model,
                    data,
                    &network.id,
                    &application_periods,
                    impact_idx,
                    &impact_action,
                ),
                Impacted::LineDeleted(line) => self.apply_on_line(
                    base_model,
                    data,
                    &line.id,
                    &application_periods,
                    impact_idx,
                    &impact_action,
                ),
                Impacted::RouteDeleted(route) => self.apply_on_route(
                    base_model,
                    data,
                    &route.id,
                    &application_periods,
                    impact_idx,
                    &impact_action,
                ),
                Impacted::BaseTripDeleted(trip) => self.apply_on_base_vehicle_journey(
                    base_model,
                    data,
                    &trip.id,
                    &application_periods,
                    impact_idx,
                    &impact_action,
                ),
                Impacted::StopAreaDeleted(stop_area) => self.apply_on_stop_area(
                    base_model,
                    data,
                    &stop_area.id,
                    &application_periods,
                    impact_idx,
                    &impact_action,
                ),
                Impacted::StopPointDeleted(stop_point) => self.apply_on_stop_point(
                    base_model,
                    data,
                    &[&stop_point.id],
                    &application_periods,
                    impact_idx,
                    &impact_action,
                ),
                Impacted::RailSection(_) => todo!(),
                Impacted::LineSection(_) => todo!(),

            };
            if let Err(err) = result {
                error!("Error while applying impact {} : {:?}", impact.id, err);
            }
        }

        let informed_action = if cancel_impact {
            Action::CancelInform
        }
        else {
            Action::Inform
        };

        for pt_object in &impact.informed_pt_objects {
            let result = match pt_object {
                Informed::Network(network) => self.apply_on_network(
                    base_model,
                    data,
                    &network.id,
                    &application_periods,
                    impact_idx,
                    &informed_action,
                ),
                Informed::Route(route) => self.apply_on_route(
                    base_model,
                    data,
                    &route.id,
                    &application_periods,
                    impact_idx,
                    &informed_action,
                ),
                Informed::Line(line) => self.apply_on_line(
                    base_model,
                    data,
                    &line.id,
                    &application_periods,
                    impact_idx,
                    &informed_action,
                ),
                Informed::Trip(trip) => self.apply_on_base_vehicle_journey(
                    base_model,
                    data,
                    &trip.id,
                    &application_periods,
                    impact_idx,
                    &informed_action,
                ),
                Informed::StopArea(stop_area) => self.apply_on_stop_area(
                    base_model,
                    data,
                    &stop_area.id,
                    &application_periods,
                    impact_idx,
                    &informed_action,
                ),
                Informed::StopPoint(stop_point) => self.apply_on_stop_point(
                    base_model,
                    data,
                    &[&stop_point.id],
                    &application_periods,
                    &application_periods,
                    impact_idx,
                    &informed_action,
                ),
            };
            if let Err(err) = result {
                error!(
                    "Error while storing informed impact {} : {:?}",
                    impact.id, err
                );
            }
        }
    }


    fn dispatch_on_base_vehicle_journey<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_idx: BaseVehicleJourneyIdx,
        date: &NaiveDate,
        chaos_impact_idx : &ChaosImpactIdx,
        action: &Action,
    ) {
        match action {
            Action::Alter => {
                let result = self.delete_trip(
                    base_model,
                    data,
                    vehicle_journey_idx,
                    date,
                    chaos_impact_idx
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
            Action::ApplyInform => self.insert_informed_linked_disruption(
                vehicle_journey_idx,
                date,
                base_model,
                chaos_impact_idx
            ),
            Action::CancelImpact => {
                let result = self.restore_base_trip(
                    base_model,
                    data,
                    vehicle_journey_idx,
                    date,
                    chaos_impact_idx
                );
                if let Err(err) = result {
                    error!(
                        "Unexpected error while restoring a base vehicle journey {:?}",
                        err
                    );
                }
            }
            Action::CancelInform => self.cancel_informed_linked_disruption(
                vehicle_journey_idx,
                date,
                base_model,
                chaos_impact_idx
            ),
        }
    }


    fn apply_on_base_vehicle_journey<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        application_periods: &TimePeriods,
        chaos_impact_idx : &ChaosImpactIdx,
        action: &Action,
    ) -> Result<(), ChaosImpactError> {
        if let Some(vehicle_journey_idx) = base_model.vehicle_journey_idx(vehicle_journey_id) {
            for date in application_periods.dates_possibly_concerned() {
                if let Some(trip_period) = base_model.trip_time_period(vehicle_journey_idx, &date) {
                    if application_periods.intersects(&trip_period) {
                        self.dispatch_on_base_vehicle_journey(
                            base_model,
                            data,
                            vehicle_journey_id,
                            &date,
                            chaos_impact_idx,
                            action
                        );
                    }
                }
            }
            Ok(())
        } else {
            Err(ChaosImpactError::VehicleJourneyAbsent(VehicleJourneyId {
                id: vehicle_journey_id.to_string(),
            }))
        }
    }

    fn apply_on_network<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        network_id: &str,
        application_periods: &TimePeriods,
        chaos_impact_idx : &ChaosImpactIdx,
        action: &Action,
    ) -> Result<(), ChaosImpactError> {
        if !base_model.contains_network_id(network_id) {
            return Err(ChaosImpactError::NetworkAbsent(NetworkId {
                id: network_id.to_string(),
            }));
        }

        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.network_name(base_vehicle_journey_idx) == Some(network_id) {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.apply_on_base_vehicle_journey(
                    base_model,
                    data,
                    vehicle_journey_id,
                    application_periods,
                    chaos_impact_idx,
                    action
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!("Unexpected error while deleting a route {:?}", err);
                }
            }
        }
        Ok(())
    }

    fn apply_on_line<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        line_id: &str,
        application_periods: &TimePeriods,
        chaos_impact_idx : &ChaosImpactIdx,
        action: &Action,
    ) -> Result<(), ChaosImpactError> {
        if !base_model.contains_line_id(line_id) {
            return Err(ChaosImpactError::LineAbsent(LineId {
                id: line_id.to_string(),
            }));
        }
        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.line_name(base_vehicle_journey_idx) == Some(line_id) {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.apply_on_base_vehicle_journey(
                    base_model,
                    data,
                    vehicle_journey_id,
                    application_periods,
                    chaos_impact_idx,
                    action
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!("Unexpected error while deleting a line {:?}", err);
                }
            }
        }
        Ok(())
    }

    fn apply_on_route<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        route_id: &str,
        application_periods: &TimePeriods,
        chaos_impact_idx : &ChaosImpactIdx,
        action: &Action,
    ) -> Result<(), ChaosImpactError> {
        if !base_model.contains_route_id(route_id) {
            return Err(ChaosImpactError::RouteAbsent(RouteId {
                id: route_id.to_string(),
            }));
        }
        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.route_name(base_vehicle_journey_idx) == route_id {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.apply_on_base_vehicle_journey(
                    base_model,
                    data,
                    vehicle_journey_id,
                    application_periods,
                    chaos_impact_idx,
                    action
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!("Unexpected error while deleting a route {:?}", err);
                }
            }
        }
        Ok(())
    }

    fn apply_on_stop_area<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        stop_area_id: &str,
        application_periods: &TimePeriods,
        chaos_impact_idx : &ChaosImpactIdx,
        action: &Action,
    ) -> Result<(), ChaosImpactError> {
        if !base_model.contains_stop_area_id(stop_area_id) {
            return Err(ChaosImpactError::StopAreaAbsent(StopAreaId {
                id: stop_area_id.to_string(),
            }));
        }
        let mut concerned_stop_point = Vec::new();
        for stop_point in base_model.stop_points() {
            let stop_area_of_stop_point = base_model.stop_area_name(stop_point);
            if stop_area_id == stop_area_of_stop_point {
                let stop_point_id = base_model.stop_point_id(stop_point);
                concerned_stop_point.push(stop_point_id);
            }
        }
        let result = self.apply_on_stop_point(
            base_model,
            data,
            &concerned_stop_point,
            application_periods,
            chaos_impact_idx,
                            action
        );
        if let Err(err) = result {
            error!("Error while deleting stop area {}. {:?}", stop_area_id, err);
        }
        Ok(())
    }

    fn apply_on_stop_point<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        stop_point_id: &[&str],
        application_periods: &TimePeriods,
        chaos_impact_idx : &ChaosImpactIdx,
        action: &Action,
    ) -> Result<(), ChaosImpactError> {
        let stop_point_idx: Vec<StopPointIdx> = stop_point_id
            .iter()
            .filter_map(|id| {
                let stop_point_idx = self.stop_point_idx(id, base_model);
                if stop_point_idx.is_none() {
                    let err = ChaosImpactError::StopPointAbsent(StopPointId { id: id.to_string() });
                    error!("Error while deleting stop point {}. {:?}", id, err);
                }
                stop_point_idx
            })
            .collect();

        for vehicle_journey_idx in base_model.vehicle_journeys() {
            let vehicle_journey_id = base_model.vehicle_journey_name(vehicle_journey_idx);
            if let Ok(base_stop_times) = base_model.stop_times(vehicle_journey_idx) {
                let contains_stop_point = base_stop_times
                    .clone()
                    .any(|stop_time| stop_point_idx.iter().any(|sp| sp == &stop_time.stop));
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
                                let concerned_stop_point =
                                    stop_point_idx.iter().any(|sp| sp == &stop_time.stop);
                                if !concerned_stop_point {
                                    return false;
                                }
                                let board_time =
                                    calendar::compose(&date, &stop_time.board_time, &timezone);
                                let debark_time =
                                    calendar::compose(&date, &stop_time.debark_time, &timezone);
                                application_periods.contains(&board_time)
                                    || application_periods.contains(&debark_time)
                            };

                            let stop_times: Vec<_> = base_stop_times
                                .clone()
                                .filter(|stop_time| !is_stop_time_concerned(stop_time))
                                .collect();

                            // if size changed it means that our vehicle is affected
                            // and need to be modified
                            if stop_times.len() != base_stop_times.len() {
                                self.dispatch_for_stop_point(
                                    base_model,
                                    data,
                                    vehicle_journey_id,
                                    &date,
                                    stop_times,
                                    chaos_impact_idx,
                                    action
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }


    fn dispatch_for_stop_point<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        stop_times: Vec<super::StopTime>,
        chaos_impact_idx : &ChaosImpactIdx,
        action: &Action,
    ) {
        match action {
            Action::Alter => {
                let result = self.modify_trip(
                    base_model,
                    data,
                    vehicle_journey_id,
                    date,
                    stop_times,
                    chaos_impact_idx,
                );
                if let Err(err) = result {
                    error!("Error while deleting stop point. {:?}", err);
                }
            }
            Action::Inform => self.insert_informed_linked_disruption(
                vehicle_journey_id,
                date,
                base_model,
                chaos_impact_idx,
            ),
            Action::CancelAlteration => {
                let result = self.restore_base_trip(
                    base_model,
                    data,
                    vehicle_journey_id,
                    date,
                    chaos_impact_idx
                );
                if let Err(err) = result {
                    error!(
                        "Unexpected error while restoring a base vehicle journey {:?}",
                        err
                    );
                }
            }
            Action::CancelInform => self.cancel_informed_linked_disruption(
                vehicle_journey_id,
                date,
                base_model,
                chaos_impact_idx
            ),
        }
    }

}