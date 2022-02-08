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
    time::{SecondsSinceTimezonedDayStart},
    timetables::FlowDirection,
};
use chrono::{ NaiveDate, NaiveDateTime};

use std::{
    fmt::{Debug},
};

use super::{TimePeriod, Effect, VehicleJourneyId};


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

impl RealTimeModel {
    //----------------------------------------------------------------------------------------
    // functions operating on TC objects for KIRIN
    fn update_new_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        trip_disruption: &TripDisruption,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), DisruptionError> {
        let vehicle_journey_id = &trip_disruption.trip_id.id;

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
        let stop_times = self.make_stop_times(&trip_disruption.stop_times, base_model);

        if self.is_present(vehicle_journey_id, &date, base_model) {
            self.modify_trip(
                base_model,
                data,
                vehicle_journey_id,
                &date,
                stop_times,
                disruption_idx,
                impact_idx,
            )
        } else {
            self.add_trip(base_model, data, vehicle_journey_id, &date, stop_times)
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
        let stop_times = self.make_stop_times(&trip_disruption.stop_times, base_model);

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
                    disruption_idx,
                    impact_idx,
                )
            } else {
                self.add_trip(base_model, data, vehicle_journey_id, &date, stop_times)
            }
        } else {
            Err(DisruptionError::VehicleJourneyAbsent(VehicleJourneyId {
                id: vehicle_journey_id.clone(),
            }))
        }
    }
}