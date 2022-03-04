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
        self,
        base_model::{BaseModel, BaseVehicleJourneyIdx},
        real_time_model::{ChaosImpactIdx, ChaosImpactObjectIdx, TripVersion},
        RealTimeModel, StopPointIdx, VehicleJourneyIdx,
    },
    time::calendar,
    transit_data::data_interface::{Data as DataTrait, DataUpdate},
};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::Deserialize;
use tracing::{debug, error, warn};

use super::{
    apply_disruption,
    time_periods::{intersection, TimePeriod, TimePeriods},
    Effect, VehicleJourneyId,
};

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

#[derive(Debug, Copy, Clone)]
pub enum Action {
    Alter,
    Inform,
    CancelAlteration,
    CancelInform,
}

#[derive(Debug, Clone)]
pub enum ChaosImpactError {
    VehicleJourneyAbsent(VehicleJourneyId),
    LineAbsent(LineId),
    RouteAbsent(RouteId),
    NetworkAbsent(NetworkId),
    StopPointAbsent(StopPointId),
    StopAreaAbsent(StopAreaId),
    DeletePresentTrip(VehicleJourneyId, NaiveDate),
}

pub fn store_and_apply_chaos_disruption<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    disruption: ChaosDisruption,
    base_model: &BaseModel,
    data: &mut Data,
) {
    debug!("Apply chaos disruption {}", disruption.id);
    let disruption_idx = real_time_model.chaos_disruptions.len();
    real_time_model.chaos_disruptions.push(disruption.clone());

    for (idx, impact) in disruption.impacts.iter().enumerate() {
        let chaos_impact_idx = ChaosImpactIdx {
            disruption_idx,
            impact_idx: idx,
        };
        apply_impact(
            real_time_model,
            impact,
            base_model,
            data,
            &chaos_impact_idx,
            false,
        );
    }
}

pub fn cancel_chaos_disruption<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    disruption_id: &str,
    base_model: &BaseModel,
    data: &mut Data,
) {
    debug!("Cancel chaos disruption {disruption_id}");

    let has_disruption_idx = real_time_model
        .chaos_disruptions
        .iter()
        .position(|disruption| disruption.id == disruption_id);
    if let Some(disruption_idx) = has_disruption_idx {
        let disruption = &real_time_model.chaos_disruptions[disruption_idx].clone();
        for (idx, impact) in disruption.impacts.iter().enumerate() {
            let chaos_impact_idx = ChaosImpactIdx {
                disruption_idx,
                impact_idx: idx,
            };
            apply_impact(
                real_time_model,
                impact,
                base_model,
                data,
                &chaos_impact_idx,
                true,
            );
        }
    } else {
        error!("Cannot cancel chaos disruption {disruption_id} since it was not found in present disruptions.");
    }
}

fn apply_impact<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    impact: &ChaosImpact,
    base_model: &BaseModel,
    data: &mut Data,
    impact_idx: &ChaosImpactIdx,
    cancel_impact: bool,
) {
    let model_period = [base_model.time_period()];
    // filter application_periods by model_period
    // by taking the intersection of theses two TimePeriods
    let application_periods: Vec<_> = impact
        .application_periods
        .iter()
        .filter_map(|application_periods| intersection(application_periods, &model_period[0]))
        .collect();

    if application_periods.is_empty() {
        return;
    }
    // unwrap is safe here because we checked if application_periods is empty or not
    let application_periods = TimePeriods::new(&application_periods).unwrap();

    let impact_action = if cancel_impact {
        Action::CancelAlteration
    } else {
        Action::Alter
    };

    for (object_idx, pt_object) in impact.impacted_pt_objects.iter().enumerate() {
        let object_idx = ChaosImpactObjectIdx::Impacted(object_idx);
        let result = match pt_object {
            Impacted::NetworkDeleted(network) => apply_on_network(
                real_time_model,
                base_model,
                data,
                &network.id,
                &application_periods,
                impact_idx,
                &object_idx,
                impact_action,
            ),
            Impacted::LineDeleted(line) => apply_on_line(
                real_time_model,
                base_model,
                data,
                &line.id,
                &application_periods,
                impact_idx,
                &object_idx,
                impact_action,
            ),
            Impacted::RouteDeleted(route) => apply_on_route(
                real_time_model,
                base_model,
                data,
                &route.id,
                &application_periods,
                impact_idx,
                &object_idx,
                impact_action,
            ),
            Impacted::BaseTripDeleted(trip) => apply_on_base_vehicle_journey(
                real_time_model,
                base_model,
                data,
                &trip.id,
                &application_periods,
                impact_idx,
                &object_idx,
                impact_action,
            ),
            Impacted::StopAreaDeleted(stop_area) => apply_on_stop_area(
                real_time_model,
                base_model,
                data,
                &stop_area.id,
                &application_periods,
                impact_idx,
                &object_idx,
                impact_action,
            ),
            Impacted::StopPointDeleted(stop_point) => apply_on_stop_point(
                real_time_model,
                base_model,
                data,
                &stop_point.id,
                &application_periods,
                impact_idx,
                &object_idx,
                impact_action,
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
    } else {
        Action::Inform
    };

    for (object_idx, pt_object) in impact.informed_pt_objects.iter().enumerate() {
        let object_idx = ChaosImpactObjectIdx::Informed(object_idx);
        let result = match pt_object {
            Informed::Network(network) => apply_on_network(
                real_time_model,
                base_model,
                data,
                &network.id,
                &application_periods,
                impact_idx,
                &object_idx,
                informed_action,
            ),
            Informed::Route(route) => apply_on_route(
                real_time_model,
                base_model,
                data,
                &route.id,
                &application_periods,
                impact_idx,
                &object_idx,
                informed_action,
            ),
            Informed::Line(line) => apply_on_line(
                real_time_model,
                base_model,
                data,
                &line.id,
                &application_periods,
                impact_idx,
                &object_idx,
                informed_action,
            ),
            Informed::Trip(trip) => apply_on_base_vehicle_journey(
                real_time_model,
                base_model,
                data,
                &trip.id,
                &application_periods,
                impact_idx,
                &object_idx,
                informed_action,
            ),
            Informed::StopArea(stop_area) => apply_on_stop_area(
                real_time_model,
                base_model,
                data,
                &stop_area.id,
                &application_periods,
                impact_idx,
                &object_idx,
                informed_action,
            ),
            Informed::StopPoint(stop_point) => apply_on_stop_point(
                real_time_model,
                base_model,
                data,
                &stop_point.id,
                &application_periods,
                impact_idx,
                &object_idx,
                informed_action,
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

fn apply_on_base_vehicle_journey<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    vehicle_journey_id: &str,
    application_periods: &TimePeriods,
    chaos_impact_idx: &ChaosImpactIdx,
    chaos_object_idx: &ChaosImpactObjectIdx,
    action: Action,
) -> Result<(), ChaosImpactError> {
    debug!(
        "Apply chaos disruption {}, {}-th impact, {:?} on vehicle journey {} ",
        real_time_model
            .get_chaos_disruption_and_impact(chaos_impact_idx)
            .0
            .id,
        chaos_impact_idx.impact_idx,
        action,
        vehicle_journey_id,
    );
    if let Some(vehicle_journey_idx) = base_model.vehicle_journey_idx(vehicle_journey_id) {
        apply_on_base_vehicle_journey_idx(
            real_time_model,
            base_model,
            data,
            vehicle_journey_idx,
            application_periods,
            chaos_impact_idx,
            chaos_object_idx,
            action,
        );
        Ok(())
    } else {
        Err(ChaosImpactError::VehicleJourneyAbsent(VehicleJourneyId {
            id: vehicle_journey_id.to_string(),
        }))
    }
}

fn apply_on_base_vehicle_journey_idx<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    base_vehicle_journey_idx: BaseVehicleJourneyIdx,
    application_periods: &TimePeriods,
    chaos_impact_idx: &ChaosImpactIdx,
    chaos_object_idx: &ChaosImpactObjectIdx,
    action: Action,
) {
    for date in application_periods.dates_possibly_concerned() {
        if let Some(trip_period) = base_model.trip_time_period(base_vehicle_journey_idx, date) {
            if application_periods.intersects(&trip_period) {
                dispatch_on_base_vehicle_journey(
                    real_time_model,
                    base_model,
                    data,
                    base_vehicle_journey_idx,
                    date,
                    chaos_impact_idx,
                    chaos_object_idx,
                    action,
                );
            }
        }
    }
}

fn dispatch_on_base_vehicle_journey<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    base_vehicle_journey_idx: BaseVehicleJourneyIdx,
    date: NaiveDate,
    chaos_impact_idx: &ChaosImpactIdx,
    chaos_object_idx: &ChaosImpactObjectIdx,
    action: Action,
) {
    debug!(
        "Apply chaos disruption {}, {}-th impact, {:?} on vehicle journey {} on {}",
        real_time_model
            .get_chaos_disruption_and_impact(chaos_impact_idx)
            .0
            .id,
        chaos_impact_idx.impact_idx,
        action,
        base_model.vehicle_journey_name(base_vehicle_journey_idx),
        date
    );
    match action {
        Action::Alter => {
            // we consider that Kirin information is more "fresh" than chaos
            // so if we have a Kirin information on this (vehicle_journey, date)
            // we do not apply chaos modifications
            let vehicle_journey_idx = VehicleJourneyIdx::Base(base_vehicle_journey_idx);
            if real_time_model
                .get_linked_kirin_disruption(&vehicle_journey_idx, date)
                .is_none()
            {
                if real_time_model.base_vehicle_journey_is_present(
                    base_vehicle_journey_idx,
                    date,
                    base_model,
                ) {
                    apply_disruption::delete_trip(
                        real_time_model,
                        base_model,
                        data,
                        &vehicle_journey_idx,
                        date,
                    );
                } else {
                    warn!("Chaos impact {:?} asked for removal of already absent vehicle journey {} on {}",
                        chaos_impact_idx,
                        base_model.vehicle_journey_name(base_vehicle_journey_idx),
                        date,
                    );
                }
            }

            real_time_model.link_chaos_impact(
                base_vehicle_journey_idx,
                date,
                base_model,
                chaos_impact_idx,
                chaos_object_idx,
            );
        }
        Action::Inform => {
            real_time_model.link_chaos_impact(
                base_vehicle_journey_idx,
                date,
                base_model,
                chaos_impact_idx,
                chaos_object_idx,
            );
        }
        Action::CancelAlteration => {
            cancel_impact(
                real_time_model,
                base_model,
                data,
                chaos_impact_idx,
                chaos_object_idx,
                base_vehicle_journey_idx,
                date,
            );
        }
        Action::CancelInform => {
            real_time_model.unlink_chaos_impact(
                base_vehicle_journey_idx,
                date,
                base_model,
                chaos_impact_idx,
                chaos_object_idx,
            );
        }
    }
}

fn apply_on_network<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    network_id: &str,
    application_periods: &TimePeriods,
    chaos_impact_idx: &ChaosImpactIdx,
    chaos_object_idx: &ChaosImpactObjectIdx,
    action: Action,
) -> Result<(), ChaosImpactError> {
    debug!(
        "Apply chaos disruption {}, {}-th impact, {:?} on network {} ",
        real_time_model
            .get_chaos_disruption_and_impact(chaos_impact_idx)
            .0
            .id,
        chaos_impact_idx.impact_idx,
        action,
        network_id,
    );
    if !base_model.contains_network_id(network_id) {
        return Err(ChaosImpactError::NetworkAbsent(NetworkId {
            id: network_id.to_string(),
        }));
    }

    for base_vehicle_journey_idx in base_model.vehicle_journeys() {
        if base_model.network_name(base_vehicle_journey_idx) == Some(network_id) {
            apply_on_base_vehicle_journey_idx(
                real_time_model,
                base_model,
                data,
                base_vehicle_journey_idx,
                application_periods,
                chaos_impact_idx,
                chaos_object_idx,
                action,
            );
        }
    }
    Ok(())
}

fn apply_on_line<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    line_id: &str,
    application_periods: &TimePeriods,
    chaos_impact_idx: &ChaosImpactIdx,
    chaos_object_idx: &ChaosImpactObjectIdx,
    action: Action,
) -> Result<(), ChaosImpactError> {
    debug!(
        "Apply chaos disruption {}, {}-th impact, {:?} on line {} ",
        real_time_model
            .get_chaos_disruption_and_impact(chaos_impact_idx)
            .0
            .id,
        chaos_impact_idx.impact_idx,
        action,
        line_id,
    );
    if !base_model.contains_line_id(line_id) {
        return Err(ChaosImpactError::LineAbsent(LineId {
            id: line_id.to_string(),
        }));
    }
    for base_vehicle_journey_idx in base_model.vehicle_journeys() {
        if base_model.line_name(base_vehicle_journey_idx) == Some(line_id) {
            apply_on_base_vehicle_journey_idx(
                real_time_model,
                base_model,
                data,
                base_vehicle_journey_idx,
                application_periods,
                chaos_impact_idx,
                chaos_object_idx,
                action,
            );
        }
    }
    Ok(())
}

fn apply_on_route<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    route_id: &str,
    application_periods: &TimePeriods,
    chaos_impact_idx: &ChaosImpactIdx,
    chaos_object_idx: &ChaosImpactObjectIdx,
    action: Action,
) -> Result<(), ChaosImpactError> {
    debug!(
        "Apply chaos disruption {}, {}-th impact, {:?} on route {} ",
        real_time_model
            .get_chaos_disruption_and_impact(chaos_impact_idx)
            .0
            .id,
        chaos_impact_idx.impact_idx,
        action,
        route_id,
    );
    if !base_model.contains_route_id(route_id) {
        return Err(ChaosImpactError::RouteAbsent(RouteId {
            id: route_id.to_string(),
        }));
    }
    for base_vehicle_journey_idx in base_model.vehicle_journeys() {
        if base_model.route_name(base_vehicle_journey_idx) == route_id {
            apply_on_base_vehicle_journey_idx(
                real_time_model,
                base_model,
                data,
                base_vehicle_journey_idx,
                application_periods,
                chaos_impact_idx,
                chaos_object_idx,
                action,
            );
        }
    }
    Ok(())
}

fn apply_on_stop_area<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    stop_area_id: &str,
    application_periods: &TimePeriods,
    chaos_impact_idx: &ChaosImpactIdx,
    chaos_object_idx: &ChaosImpactObjectIdx,
    action: Action,
) -> Result<(), ChaosImpactError> {
    debug!(
        "Apply chaos disruption {}, {}-th impact, {:?} on stop area {} ",
        real_time_model
            .get_chaos_disruption_and_impact(chaos_impact_idx)
            .0
            .id,
        chaos_impact_idx.impact_idx,
        action,
        stop_area_id,
    );
    if !base_model.contains_stop_area_id(stop_area_id) {
        return Err(ChaosImpactError::StopAreaAbsent(StopAreaId {
            id: stop_area_id.to_string(),
        }));
    }

    let is_stop_point_concerned = |stop_point: &StopPointIdx| {
        if let StopPointIdx::Base(base_stop_point) = stop_point {
            let stop_area = base_model.stop_area_name(*base_stop_point);
            stop_area == stop_area_id
        } else {
            false
        }
    };

    apply_on_stop_point_by_closure(
        real_time_model,
        base_model,
        data,
        is_stop_point_concerned,
        application_periods,
        chaos_impact_idx,
        chaos_object_idx,
        action,
    );

    Ok(())
}

fn apply_on_stop_point<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    stop_point_id: &str,
    application_periods: &TimePeriods,
    chaos_impact_idx: &ChaosImpactIdx,
    chaos_object_idx: &ChaosImpactObjectIdx,
    action: Action,
) -> Result<(), ChaosImpactError> {
    debug!(
        "Apply chaos disruption {}, {}-th impact, {:?} on stop point {} ",
        real_time_model
            .get_chaos_disruption_and_impact(chaos_impact_idx)
            .0
            .id,
        chaos_impact_idx.impact_idx,
        action,
        stop_point_id,
    );
    let stop_point_idx = base_model.stop_point_idx(stop_point_id).ok_or_else(|| {
        ChaosImpactError::StopPointAbsent(StopPointId {
            id: stop_point_id.to_string(),
        })
    })?;
    let is_stop_point_concerned =
        |stop_point: &StopPointIdx| *stop_point == StopPointIdx::Base(stop_point_idx);

    apply_on_stop_point_by_closure(
        real_time_model,
        base_model,
        data,
        is_stop_point_concerned,
        application_periods,
        chaos_impact_idx,
        chaos_object_idx,
        action,
    );

    Ok(())
}

fn apply_on_stop_point_by_closure<Data: DataTrait + DataUpdate, F: Fn(&StopPointIdx) -> bool>(
    real_time_model: &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    is_stop_point_concerned: F,
    application_periods: &TimePeriods,
    chaos_impact_idx: &ChaosImpactIdx,
    chaos_object_idx: &ChaosImpactObjectIdx,
    action: Action,
) {
    for base_vehicle_journey_idx in base_model.vehicle_journeys() {
        if let Ok(base_stop_times) = base_model.stop_times(base_vehicle_journey_idx) {
            let contains_concerned_stop_point = base_stop_times
                .clone()
                .any(|stop_time| is_stop_point_concerned(&stop_time.stop));
            if !contains_concerned_stop_point {
                continue;
            }
            let timezone = base_model
                .timezone(base_vehicle_journey_idx)
                .unwrap_or(chrono_tz::UTC);
            for date in application_periods.dates_possibly_concerned() {
                // check if the vehicle exists on the real time level
                if let Some(time_period) =
                    base_model.trip_time_period(base_vehicle_journey_idx, date)
                {
                    if application_periods.intersects(&time_period) {
                        let is_stop_time_concerned = |stop_time: &models::StopTime| {
                            if !is_stop_point_concerned(&stop_time.stop) {
                                return false;
                            }
                            let board_time =
                                calendar::compose(date, stop_time.board_time, timezone);
                            let debark_time =
                                calendar::compose(date, stop_time.debark_time, timezone);
                            application_periods.contains(&board_time)
                                || application_periods.contains(&debark_time)
                        };

                        let has_a_stop_time_concerned = base_stop_times
                            .clone()
                            .any(|stop_time| is_stop_time_concerned(&stop_time));

                        if !has_a_stop_time_concerned {
                            continue;
                        }

                        match action {
                            Action::Alter => {
                                remove_stop_points_from_trip(
                                    real_time_model,
                                    base_model,
                                    data,
                                    &is_stop_point_concerned,
                                    application_periods,
                                    base_vehicle_journey_idx,
                                    date,
                                );

                                real_time_model.link_chaos_impact(
                                    base_vehicle_journey_idx,
                                    date,
                                    base_model,
                                    chaos_impact_idx,
                                    chaos_object_idx,
                                );
                            }
                            Action::Inform => {
                                real_time_model.link_chaos_impact(
                                    base_vehicle_journey_idx,
                                    date,
                                    base_model,
                                    chaos_impact_idx,
                                    chaos_object_idx,
                                );
                            }
                            Action::CancelAlteration => {
                                cancel_impact(
                                    real_time_model,
                                    base_model,
                                    data,
                                    chaos_impact_idx,
                                    chaos_object_idx,
                                    base_vehicle_journey_idx,
                                    date,
                                );
                            }
                            Action::CancelInform => {
                                real_time_model.unlink_chaos_impact(
                                    base_vehicle_journey_idx,
                                    date,
                                    base_model,
                                    chaos_impact_idx,
                                    chaos_object_idx,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

fn remove_stop_points_from_trip<Data: DataTrait + DataUpdate, F: Fn(&StopPointIdx) -> bool>(
    real_time_model: &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    is_stop_point_concerned: &F,
    application_periods: &TimePeriods,
    base_vehicle_journey_idx: BaseVehicleJourneyIdx,
    date: NaiveDate,
) {
    // we consider that Kirin information is more "fresh" than chaos
    // so if we have a Kirin information on this (vehicle_journey, date)
    // we do not apply chaos modifications
    let vehicle_journey_idx = VehicleJourneyIdx::Base(base_vehicle_journey_idx);
    if real_time_model
        .get_linked_kirin_disruption(&vehicle_journey_idx, date)
        .is_some()
    {
        return;
    }

    let timezone = base_model
        .timezone(base_vehicle_journey_idx)
        .unwrap_or(chrono_tz::UTC);

    let is_stop_time_concerned = |stop_time: &models::StopTime| {
        if !is_stop_point_concerned(&stop_time.stop) {
            return false;
        }
        let board_time = calendar::compose(date, stop_time.board_time, timezone);
        let debark_time = calendar::compose(date, stop_time.debark_time, timezone);
        application_periods.contains(&board_time) || application_periods.contains(&debark_time)
    };

    let real_time_version =
        real_time_model.base_vehicle_journey_last_version(base_vehicle_journey_idx, date);
    let base_stop_times = base_model.stop_times(base_vehicle_journey_idx);

    let new_stop_times: Vec<_> = match (real_time_version, base_stop_times) {
        (Some(TripVersion::Deleted()), _) => {
            // the trip was deleted, so there is nothing to do
            return;
        }
        (None, Err(_)) => {
            // no real_time version, and no base stop times
            // there is nothing that can be done
            return;
        }
        (Some(TripVersion::Present(stop_times)), _) => {
            // We have a real time version

            let has_a_stop_time_concerned = stop_times
                .iter()
                .any(|stop_time| is_stop_time_concerned(stop_time));

            // no stop_time concenred on the real time level
            // so there is nothing to do
            if !has_a_stop_time_concerned {
                return;
            }

            stop_times
                .iter()
                .filter(|stop_time| !is_stop_time_concerned(stop_time))
                .cloned()
                .collect()
        }
        (None, Ok(stop_times)) => {
            // there is no real time version for this vehicle, so
            // we take the base schedule

            // if the trip does not exists on this day on the base schedule, nothing to do
            if !base_model.trip_exists(base_vehicle_journey_idx, date) {
                return;
            }
            let has_a_stop_time_concerned = stop_times
                .clone()
                .any(|stop_time| is_stop_time_concerned(&stop_time));

            if !has_a_stop_time_concerned {
                return;
            }

            stop_times
                .filter(|stop_time| !is_stop_time_concerned(stop_time))
                .collect()
        }
    };
    let vehicle_journey_idx = VehicleJourneyIdx::Base(base_vehicle_journey_idx);

    apply_disruption::modify_trip(
        real_time_model,
        base_model,
        data,
        &vehicle_journey_idx,
        &date,
        new_stop_times,
    );
}

fn cancel_impact<Data: DataTrait + DataUpdate>(
    real_time_model: &mut RealTimeModel,
    base_model: &BaseModel,
    data: &mut Data,
    chaos_impact_idx: &ChaosImpactIdx,
    chaos_object_idx: &ChaosImpactObjectIdx,
    base_vehicle_journey_idx: BaseVehicleJourneyIdx,
    date: NaiveDate,
) {
    debug!(
        "Cancel chaos disruption {}, {}-th impact,  vehicle journey {} on {}",
        real_time_model
            .get_chaos_disruption_and_impact(chaos_impact_idx)
            .0
            .id,
        chaos_impact_idx.impact_idx,
        base_model.vehicle_journey_name(base_vehicle_journey_idx),
        date,
    );

    let contains_impact_to_remove = {
        let has_linked_chaos_impacts =
            real_time_model.get_linked_chaos_impacts(base_vehicle_journey_idx, date);

        if let Some(linked_chaos_impacts) = has_linked_chaos_impacts {
            let val = (chaos_impact_idx.clone(), chaos_object_idx.clone());
            linked_chaos_impacts.contains(&val)
        } else {
            false
        }
    };

    if !contains_impact_to_remove {
        return;
    }

    real_time_model.unlink_chaos_impact(
        base_vehicle_journey_idx,
        date,
        base_model,
        chaos_impact_idx,
        chaos_object_idx,
    );

    // we consider that Kirin information is more "fresh" than chaos
    // so if we have a Kirin information on this (vehicle_journey, date)
    // we do not need to reapply chaos modifications
    let vehicle_journey_idx = VehicleJourneyIdx::Base(base_vehicle_journey_idx);
    if real_time_model
        .get_linked_kirin_disruption(&vehicle_journey_idx, date)
        .is_some()
    {
        return;
    }

    {
        let (_, impact_to_remove) =
            real_time_model.get_chaos_disruption_and_impact(chaos_impact_idx);
        // if the impact_to_remove has no impacted_pt_objects, it means that
        // this impact has not modified the stop_times of this vehicle_journey
        // so we have nothing to do
        if impact_to_remove.impacted_pt_objects.is_empty() {
            return;
        }
    }

    let vehicle_journey_idx = VehicleJourneyIdx::Base(base_vehicle_journey_idx);

    let base_stop_times: Vec<_> = match base_model.stop_times(base_vehicle_journey_idx) {
        Ok(stop_times) => stop_times.collect(),
        Err(_) => {
            return;
        }
    };

    // restore the vehicle to its base schedule
    if real_time_model.base_vehicle_journey_is_present(base_vehicle_journey_idx, date, base_model) {
        apply_disruption::modify_trip(
            real_time_model,
            base_model,
            data,
            &vehicle_journey_idx,
            &date,
            base_stop_times.clone(),
        );
    } else {
        apply_disruption::add_trip(
            real_time_model,
            base_model,
            data,
            vehicle_journey_idx.clone(),
            date,
            base_stop_times.clone(),
        );
    }

    real_time_model.set_base_trip_version(
        base_vehicle_journey_idx,
        &date,
        TripVersion::Present(base_stop_times),
    );

    let linked_chaos_impacts = {
        let has_linked_chaos_impacts =
            real_time_model.get_linked_chaos_impacts(base_vehicle_journey_idx, date);

        if let Some(linked_chaos_impacts) = has_linked_chaos_impacts {
            linked_chaos_impacts.to_vec()
        } else {
            // if there is no linked_impacts, nothing to do
            // TODO ? : reapply kirin disruption, if any ?
            return;
        }
    };

    // iterate all linked_chaos_impacts and apply them to this vj
    for (impact_idx, object_idx) in linked_chaos_impacts {
        debug!(
            "Reapplying disruption {} impact {:?} object {:?} on vehicle journey {:?} on {}",
            real_time_model
                .get_chaos_disruption_and_impact(&impact_idx)
                .0
                .id,
            impact_idx,
            object_idx,
            base_model.vehicle_journey_name(base_vehicle_journey_idx),
            date,
        );
        let (impacted_object, application_periods) = match object_idx {
            ChaosImpactObjectIdx::Informed(_) => {
                continue;
            }
            ChaosImpactObjectIdx::Impacted(idx) => {
                let (_, impact) = real_time_model.get_chaos_disruption_and_impact(&impact_idx);
                (
                    impact.impacted_pt_objects[idx].clone(),
                    impact.application_periods.clone(),
                )
            }
        };

        if application_periods.is_empty() {
            continue;
        }
        // unwrap is safe here because we checked if application_periods is empty or not
        let application_periods = TimePeriods::new(&application_periods).unwrap();

        match impacted_object {
            Impacted::NetworkDeleted(_)
            | Impacted::LineDeleted(_)
            | Impacted::RouteDeleted(_)
            | Impacted::BaseTripDeleted(_) => {
                if real_time_model.base_vehicle_journey_is_present(
                    base_vehicle_journey_idx,
                    date,
                    base_model,
                ) {
                    apply_disruption::delete_trip(
                        real_time_model,
                        base_model,
                        data,
                        &vehicle_journey_idx,
                        date,
                    );
                }
            }
            Impacted::StopAreaDeleted(StopAreaId { id }) => {
                if base_model.contains_stop_area_id(&id) {
                    let is_stop_point_concerned = |stop_point: &StopPointIdx| {
                        if let StopPointIdx::Base(base_stop_point) = stop_point {
                            let stop_area = base_model.stop_area_name(*base_stop_point);
                            stop_area == id
                        } else {
                            false
                        }
                    };
                    remove_stop_points_from_trip(
                        real_time_model,
                        base_model,
                        data,
                        &is_stop_point_concerned,
                        &application_periods,
                        base_vehicle_journey_idx,
                        date,
                    );
                }
            }
            Impacted::StopPointDeleted(StopPointId { id }) => {
                if let Some(stop_point_idx) = base_model.stop_point_idx(&id) {
                    let is_stop_point_concerned = |stop_point: &StopPointIdx| {
                        *stop_point == StopPointIdx::Base(stop_point_idx)
                    };
                    remove_stop_points_from_trip(
                        real_time_model,
                        base_model,
                        data,
                        &is_stop_point_concerned,
                        &application_periods,
                        base_vehicle_journey_idx,
                        date,
                    );
                }
            }
            Impacted::LineSection(_) => todo!(),
            Impacted::RailSection(_) => todo!(),
        }
    }
}
