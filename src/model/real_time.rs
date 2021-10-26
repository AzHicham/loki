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

use std::collections::HashMap;
use std::hash::Hash;

use crate::time::SecondsSinceTimezonedDayStart;
use crate::transit_data::{handle_insertion_errors, handle_removal_errors};
use crate::{
    chrono::NaiveDate, timetables::FlowDirection, transit_model::Model, 
};

use crate::{DataUpdate, LoadsData};

use super::{ModelRefs, StopPointIdx, TransitModelVehicleJourneyIdx, VehicleJourneyIdx};

pub struct RealTimeModel {
    // base_model: Model,
    pub(super) disruption_impacts: HashMap<String, ImpactedVehicleAndStops>,

    pub(super) new_vehicle_journeys_id_to_idx: HashMap<String, NewVehicleJourneyIdx>,
    // indexed by NewVehicleJourney.idx
    pub(super) new_vehicle_journeys_history: Vec<(String, VehicleJourneyHistory)>,

    // gives position in base_vehicle_journeys_history, if any
    pub(super) base_vehicle_journeys_idx_to_history: HashMap<TransitModelVehicleJourneyIdx, usize>,
    pub(super) base_vehicle_journeys_history: Vec<VehicleJourneyHistory>,

    pub(super) new_stop_id_to_idx: HashMap<String, NewStopPointIdx>,
    pub(super) new_stops: Vec<StopData>,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct NewVehicleJourneyIdx {
    pub idx: usize, // position in new_vehicle_journeys_history
}

#[derive(Debug, Clone)]
pub struct VehicleJourneyHistory {
    by_reference_date: HashMap<NaiveDate, TripHistory>,
}

#[derive(Debug, Clone)]
pub struct TripHistory {
    // String is the disruption id
    versions: Vec<(String, TripData)>, // all versions of this trip,
}

#[derive(Debug, Clone)]
pub enum TripData {
    Deleted(),              // the trip is currently disabled
    Present(Vec<StopTime>), // list of all stop times of this trip
}

#[derive(Debug, Clone)]
pub struct StopTime {
    pub stop: StopPointIdx,
    pub arrival_time: SecondsSinceTimezonedDayStart,
    pub departure_time: SecondsSinceTimezonedDayStart,
    pub flow_direction: FlowDirection,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct NewStopPointIdx {
    pub idx: usize, // position in new_stops
}

pub struct StopData {
    pub(super) name: String,
}

pub struct ImpactedVehicleAndStops {
    vehicle_journey: VehicleJourneyIdx,
    stops: Vec<StopPointIdx>,
}

impl RealTimeModel {
    // pub fn base_model(&self) -> & Model {
    //     &self.base_model
    // }

    pub fn apply_disruption<Data: DataUpdate>(
        &mut self,
        disruption: &super::disruption::Disruption,
        model: &Model,
        loads_data: &LoadsData,
        data: &mut Data,
    ) {
        for update in disruption.updates.iter() {
            self.apply_update(&disruption.id, update, model, loads_data, data);
        }
    }

    pub fn apply_update<Data: DataUpdate>(
        &mut self,
        disruption_id: &str,
        update: &super::disruption::Update,
        model: &Model,
        loads_data: &LoadsData,
        data: &mut Data,
    ) {
        match update {
            super::disruption::Update::Delete(trip) => {
                let vj_idx = self.delete(disruption_id, trip, model);
                let removal_result = data.remove_vehicle(&vj_idx, &trip.reference_date);
                if let Err(removal_error) = removal_result {
                    let model_ref = ModelRefs{ base : model, real_time : &self};
                    handle_removal_errors(&model_ref, data.calendar(), std::iter::once(removal_error))
                }
                
            }
            super::disruption::Update::Add(trip, stop_times) => {
                let (vj_idx, stop_times) = self.add(disruption_id, trip, stop_times, model);
                let dates = std::iter::once(&trip.reference_date);
                let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
                let flows = stop_times
                    .iter()
                    .map(|stop_time| stop_time.flow_direction.clone());
                let board_times = stop_times
                    .iter()
                    .map(|stop_time| stop_time.departure_time.clone());
                let debark_times = stop_times
                    .iter()
                    .map(|stop_time| stop_time.arrival_time.clone());
                let insertion_errors = data.add_vehicle(
                    stops,
                    flows,
                    board_times,
                    debark_times,
                    loads_data,
                    dates,
                    &chrono_tz::UTC,
                    vj_idx,
                );
                let model_ref = ModelRefs{ base : model, real_time : &self};
                handle_insertion_errors(&model_ref, data.calendar(), &insertion_errors);
            }
            super::disruption::Update::Modify(trip, stop_times) => {
                let (vj_idx, stop_times) = self.modify(disruption_id, trip, stop_times, model);
                let dates = std::iter::once(&trip.reference_date);
                let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
                let flows = stop_times
                    .iter()
                    .map(|stop_time| stop_time.flow_direction.clone());
                let board_times = stop_times
                    .iter()
                    .map(|stop_time| stop_time.departure_time.clone());
                let debark_times = stop_times
                    .iter()
                    .map(|stop_time| stop_time.arrival_time.clone());
                let (removal_errors, insertion_errors) = data.modify_vehicle(
                    stops,
                    flows,
                    board_times,
                    debark_times,
                    loads_data,
                    dates,
                    &chrono_tz::UTC,
                    vj_idx,
                );
                let model_ref = ModelRefs{ base : model, real_time : &self};
                handle_insertion_errors(&model_ref, data.calendar(), &insertion_errors);
                handle_removal_errors(&model_ref, data.calendar(), removal_errors.into_iter())
            }
        }
    }

    pub fn delete(
        &mut self,
        disruption_id: &str,
        trip: &super::disruption::Trip,
        model: &Model,
    ) -> VehicleJourneyIdx {
        let (idx, history) = self.get_or_insert_history(trip, model, disruption_id);
        history
            .versions
            .push((disruption_id.to_string(), TripData::Deleted()));
        idx
    }

    pub fn add(
        &mut self,
        disruption_id: &str,
        trip: &super::disruption::Trip,
        stop_times: &[super::disruption::StopTime],
        model: &Model,
    ) -> (VehicleJourneyIdx, Vec<StopTime>) {
        let stop_times = self.make_stop_times(stop_times, model);
        let (idx, history) = self.get_or_insert_history(trip, model, disruption_id);
        history.versions.push((
            disruption_id.to_string(),
            TripData::Present(stop_times.clone()),
        ));
        (idx, stop_times)
    }

    pub fn modify(
        &mut self,
        disruption_id: &str,
        trip: &super::disruption::Trip,
        stop_times: &[super::disruption::StopTime],
        model: &Model,
    ) -> (VehicleJourneyIdx, Vec<StopTime>) {
        let stop_times = self.make_stop_times(stop_times, model);
        let (idx, history) = self.get_or_insert_history(trip, model, disruption_id);
        history.versions.push((
            disruption_id.to_string(),
            TripData::Present(stop_times.clone()),
        ));
        (idx, stop_times)
    }

    fn get_or_insert_history(
        &mut self,
        trip: &super::disruption::Trip,
        model: &Model,
        disruption_id: &str,
    ) -> (VehicleJourneyIdx, &mut TripHistory) {
        if let Some(transit_model_idx) = model.vehicle_journeys.get_idx(&trip.vehicle_journey_id) {
            let trip_history = self.get_or_insert_base_vehicle_journey_history(
                &transit_model_idx,
                &trip.reference_date,
            );
            let idx = VehicleJourneyIdx::Base(transit_model_idx);
            (idx, trip_history)
        } else {
            self.get_or_insert_new_vehicle_journey_history(
                &trip.vehicle_journey_id,
                &trip.reference_date,
                disruption_id,
            )
        }
    }

    fn get_or_insert_base_vehicle_journey_history(
        &mut self,
        idx: &TransitModelVehicleJourneyIdx,
        date: &NaiveDate,
    ) -> &mut TripHistory {
        let base_vj_history = &mut self.base_vehicle_journeys_history;
        let pos = self
            .base_vehicle_journeys_idx_to_history
            .entry(*idx)
            .or_insert_with(|| {
                let pos = base_vj_history.len();
                base_vj_history.push(VehicleJourneyHistory::new());
                pos
            });

        let vj_history = &mut self.base_vehicle_journeys_history[*pos];
        vj_history
            .by_reference_date
            .entry(date.clone())
            .or_insert_with(|| TripHistory {
                versions: Vec::new(),
            })
    }

    fn get_or_insert_new_vehicle_journey_history(
        &mut self,
        id: &str,
        date: &NaiveDate,
        disruption_id: &str,
    ) -> (VehicleJourneyIdx, &mut TripHistory) {
        let new_vj_history = &mut self.new_vehicle_journeys_history;
        let idx = self
            .new_vehicle_journeys_id_to_idx
            .entry(id.to_string())
            .or_insert_with(|| {
                let idx = NewVehicleJourneyIdx {
                    idx: new_vj_history.len(),
                };
                new_vj_history.push((disruption_id.to_string(), VehicleJourneyHistory::new()));
                idx
            });

        let vj_history = &mut self.new_vehicle_journeys_history[idx.idx].1;
        let trip_history = vj_history
            .by_reference_date
            .entry(date.clone())
            .or_insert_with(|| TripHistory {
                versions: Vec::new(),
            });

        let idx = VehicleJourneyIdx::New(idx.clone());
        (idx, trip_history)
    }

    fn make_stop_times(
        &mut self,
        stop_times: &[super::disruption::StopTime],
        model: &Model,
    ) -> Vec<StopTime> {
        let mut result = Vec::new();
        for stop_time in stop_times {
            let stop_id = stop_time.stop_id.as_str();
            let stop_idx = self.get_or_insert_stop(stop_id, model);
            result.push(StopTime {
                stop: stop_idx,
                departure_time: stop_time.departure_time,
                arrival_time: stop_time.arrival_time,
                flow_direction: stop_time.flow_direction,
            });
        }
        result
    }

    fn get_or_insert_stop(&mut self, stop_id: &str, model: &Model) -> StopPointIdx {
        if let Some(idx) = model.stop_points.get_idx(stop_id) {
            StopPointIdx::Base(idx)
        } else if let Some(idx) = self.new_stop_id_to_idx.get(stop_id) {
            StopPointIdx::New(idx.clone())
        } else {
            let idx = NewStopPointIdx {
                idx: self.new_stops.len(),
            };
            self.new_stop_id_to_idx
                .insert(stop_id.to_string(), idx.clone());
            StopPointIdx::New(idx.clone())
        }
    }

    pub(super) fn base_vehicle_journey_last_version(
        &self,
        idx: &TransitModelVehicleJourneyIdx,
        date: &NaiveDate,
    ) -> Option<&TripData> {
        self.base_vehicle_journeys_idx_to_history
            .get(idx)
            .map(|pos| {
                self.base_vehicle_journeys_history[*pos]
                    .trip_data(date)
                    .map(|(_, trip_data)| trip_data)
            })
            .flatten()
    }

    pub(super) fn new_vehicle_journey_last_version(
        &self,
        idx: &NewVehicleJourneyIdx,
        date: &NaiveDate,
    ) -> Option<&TripData> {
        self.new_vehicle_journeys_history[idx.idx]
            .1
            .trip_data(date)
            .map(|(_, trip_data)| trip_data)
    }

    pub fn new() -> Self {
        Self {
            disruption_impacts: HashMap::new(),
            new_vehicle_journeys_id_to_idx: HashMap::new(),
            new_vehicle_journeys_history: Vec::new(),
            base_vehicle_journeys_idx_to_history: HashMap::new(),
            base_vehicle_journeys_history: Vec::new(),
            new_stop_id_to_idx: HashMap::new(),
            new_stops: Vec::new(),
        }
    }
}

impl VehicleJourneyHistory {
    pub fn new() -> Self {
        Self {
            by_reference_date: HashMap::new(),
        }
    }

    pub fn trip_data(&self, date: &NaiveDate) -> Option<&(String, TripData)> {
        self.by_reference_date
            .get(date)
            .map(|trip_history| trip_history.versions.last())
            .flatten()
    }
}
