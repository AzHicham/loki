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

use std::{collections::HashMap, hash::Hash};
use tracing::warn;

use crate::chrono::NaiveDate;

use super::{
    base_model::{BaseModel, BaseVehicleJourneyIdx},
    real_time_disruption as disruption, StopPointIdx, StopTime, StopTimeIdx, VehicleJourneyIdx,
};

pub struct RealTimeModel {
    pub(super) new_vehicle_journeys_id_to_idx: HashMap<String, NewVehicleJourneyIdx>,
    // indexed by NewVehicleJourney.idx
    pub(super) new_vehicle_journeys_history: Vec<(String, VehicleJourneyHistory)>,

    pub(super) base_vehicle_journeys_idx_to_history:
        HashMap<BaseVehicleJourneyIdx, VehicleJourneyHistory>,

    pub(super) new_stop_id_to_idx: HashMap<String, NewStopPointIdx>,
    pub(super) new_stops: Vec<StopData>,

    pub(super) disruptions: Vec<disruption::Disruption>,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct NewVehicleJourneyIdx {
    pub idx: usize, // position in new_vehicle_journeys_history
}

type LinkedDisruption = Vec<(DisruptionIdx, Vec<ImpactIdx>)>;

#[derive(Debug, Clone)]
pub struct VehicleJourneyHistory {
    by_reference_date: HashMap<NaiveDate, TripVersion>,
    linked_disruption: HashMap<NaiveDate, LinkedDisruption>,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct DisruptionIdx {
    pub(super) idx: usize, // position in RealTimeModel.disruptions
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct ImpactIdx {
    pub(super) idx: usize, // position in RealTimeModel.disruptions
}

impl DisruptionIdx {
    // this here only to facilitate tests
    // do not use elsewhere
    pub fn new(idx: usize) -> Self {
        Self { idx }
    }
}

impl ImpactIdx {
    // this here only to facilitate tests
    // do not use elsewhere
    pub fn new(idx: usize) -> Self {
        Self { idx }
    }
}

#[derive(Debug, Clone)]
pub struct TripVersion {
    trip_data: TripData,
}

#[derive(Debug, Clone)]
pub enum TripData {
    Deleted(),              // the trip is currently disabled
    Present(Vec<StopTime>), // list of all stop times of this trip
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct NewStopPointIdx {
    pub idx: usize, // position in new_stops
}

pub struct StopData {
    pub(super) name: String,
}

#[derive(Clone)]
pub struct RealTimeStopTimes<'a> {
    pub(super) inner: std::slice::Iter<'a, StopTime>,
}

#[derive(Debug, Clone)]
pub struct Trip {
    pub vehicle_journey_id: String,
    pub reference_date: NaiveDate,
}

#[derive(Debug, Clone)]
pub enum Update {
    Delete(Trip),
    Add(Trip, Vec<StopTime>),
    Modify(Trip, Vec<StopTime>),
}

#[derive(Debug, Clone)]
pub enum UpdateError {
    DeleteAbsentTrip(Trip),
    ModifyAbsentTrip(Trip),
    AddPresentTrip(Trip),
}

impl RealTimeModel {
    pub fn delete(
        &mut self,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        base_model: &BaseModel,
    ) -> Result<VehicleJourneyIdx, UpdateError> {
        if self.is_present(vehicle_journey_id, date, base_model) {
            let trip_version = TripVersion {
                trip_data: TripData::Deleted(),
            };
            let idx = self.set_version(
                vehicle_journey_id,
                date,
                base_model,
                trip_version,
                disruption_idx,
                impact_idx,
            );

            Ok(idx)
        } else {
            let err = UpdateError::DeleteAbsentTrip(Trip {
                vehicle_journey_id: vehicle_journey_id.to_string(),
                reference_date: *date,
            });
            Err(err)
        }
    }

    pub fn add(
        &mut self,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        stop_times: &[disruption::StopTime],
        base_model: &BaseModel,
    ) -> Result<(VehicleJourneyIdx, Vec<StopTime>), UpdateError> {
        if self.is_present(vehicle_journey_id, date, base_model) {
            let err = UpdateError::AddPresentTrip(Trip {
                vehicle_journey_id: vehicle_journey_id.to_string(),
                reference_date: *date,
            });
            Err(err)
        } else {
            let stop_times = self.make_stop_times(stop_times, base_model);
            let trip_version = TripVersion {
                trip_data: TripData::Present(stop_times.clone()),
            };
            let idx = self.set_version(
                vehicle_journey_id,
                date,
                base_model,
                trip_version,
                disruption_idx,
                impact_idx,
            );
            Ok((idx, stop_times))
        }
    }

    pub fn modify(
        &mut self,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        stop_times: &[disruption::StopTime],
        base_model: &BaseModel,
    ) -> Result<(VehicleJourneyIdx, Vec<StopTime>), UpdateError> {
        if !self.is_present(vehicle_journey_id, date, base_model) {
            let err = UpdateError::ModifyAbsentTrip(Trip {
                vehicle_journey_id: vehicle_journey_id.to_string(),
                reference_date: *date,
            });
            Err(err)
        } else {
            let stop_times = self.make_stop_times(stop_times, base_model);
            let trip_version = TripVersion {
                trip_data: TripData::Present(stop_times.clone()),
            };
            let idx = self.set_version(
                vehicle_journey_id,
                date,
                base_model,
                trip_version,
                disruption_idx,
                impact_idx,
            );
            Ok((idx, stop_times))
        }
    }

    pub fn is_present(
        &self,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        base_model: &BaseModel,
    ) -> bool {
        if let Some(transit_model_idx) = base_model.vehicle_journey_idx(vehicle_journey_id) {
            let last_version = self.base_vehicle_journey_last_version(&transit_model_idx, date);
            match last_version {
                Some(&TripData::Deleted()) => false,
                Some(&TripData::Present(_)) => true,
                None => base_model.trip_exists(transit_model_idx, *date),
            }
        } else {
            let has_new_vj_idx = self.new_vehicle_journeys_id_to_idx.get(vehicle_journey_id);
            if let Some(new_vj_idx) = has_new_vj_idx {
                let last_version = self.new_vehicle_journey_last_version(new_vj_idx, date);
                match last_version {
                    None => false,
                    Some(&TripData::Deleted()) => false,
                    Some(&TripData::Present(_)) => true,
                }
            } else {
                false
            }
        }
    }

    fn set_version(
        &mut self,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        base_model: &BaseModel,
        trip_version: TripVersion,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
    ) -> VehicleJourneyIdx {
        let (history, vj_idx) = if let Some(transit_model_idx) =
            base_model.vehicle_journey_idx(vehicle_journey_id)
        {
            let history = self
                .base_vehicle_journeys_idx_to_history
                .entry(transit_model_idx)
                .or_insert_with(VehicleJourneyHistory::new);
            let vj_idx = VehicleJourneyIdx::Base(transit_model_idx);
            (history, vj_idx)
        } else {
            let histories = &mut self.new_vehicle_journeys_history;
            let idx = self
                .new_vehicle_journeys_id_to_idx
                .entry(vehicle_journey_id.to_string())
                .or_insert_with(|| {
                    let idx = histories.len();
                    histories.push((vehicle_journey_id.to_string(), VehicleJourneyHistory::new()));
                    NewVehicleJourneyIdx { idx }
                });
            let history = &mut self.new_vehicle_journeys_history[idx.idx].1;
            let vj_idx = VehicleJourneyIdx::New(idx.clone());
            (history, vj_idx)
        };

        history.by_reference_date.insert(*date, trip_version);
        RealTimeModel::insert_linked_disruption(
            &mut history.linked_disruption,
            disruption_idx,
            impact_idx,
            vehicle_journey_id,
            date,
        );
        vj_idx
    }

    pub fn insert_informed_linked_disruption(
        &mut self,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        base_model: &BaseModel,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
    ) {
        let history = if let Some(transit_model_idx) =
            base_model.vehicle_journey_idx(vehicle_journey_id)
        {
            self.base_vehicle_journeys_idx_to_history
                .entry(transit_model_idx)
                .or_insert_with(VehicleJourneyHistory::new)
        } else {
            // This case should never happen
            // Informed impact from chaos never affect a new_vehicle_journeys
            // but only base vehicle_journey
            let histories = &mut self.new_vehicle_journeys_history;
            let idx = self
                .new_vehicle_journeys_id_to_idx
                .entry(vehicle_journey_id.to_string())
                .or_insert_with(|| {
                    let idx = histories.len();
                    histories.push((vehicle_journey_id.to_string(), VehicleJourneyHistory::new()));
                    NewVehicleJourneyIdx { idx }
                });
            &mut self.new_vehicle_journeys_history[idx.idx].1
        };

        RealTimeModel::insert_linked_disruption(
            &mut history.linked_disruption,
            disruption_idx,
            impact_idx,
            vehicle_journey_id,
            date,
        );
    }

    fn insert_linked_disruption(
        linked_disruptions_map: &mut HashMap<NaiveDate, LinkedDisruption>,
        disruption_idx: DisruptionIdx,
        impact_idx: ImpactIdx,
        vehicle_journey_id: &str,
        date: &NaiveDate,
    ) {
        let linked_disruptions = linked_disruptions_map
            .entry(*date)
            .or_insert_with(LinkedDisruption::new);

        let find_disruption = &mut linked_disruptions
            .iter_mut()
            .find(|(disruption, _)| *disruption == disruption_idx);

        match find_disruption {
            Some((_, ref mut impacts)) => {
                let find_impact = &mut impacts.iter().find(|impact| **impact == impact_idx);
                match find_impact {
                    Some(_) => {
                        warn!("Disruption : {:?} and Impact : {:?} already link to this vehicle_journey {:?}",
                                disruption_idx, impact_idx, vehicle_journey_id);
                    }
                    None => {
                        impacts.push(impact_idx);
                    }
                }
            }
            None => {
                linked_disruptions.push((disruption_idx, vec![impact_idx]));
            }
        }
    }

    fn make_stop_times(
        &mut self,
        stop_times: &[disruption::StopTime],
        base_model: &BaseModel,
    ) -> Vec<StopTime> {
        let mut result = Vec::new();
        for stop_time in stop_times {
            let stop_id = stop_time.stop_id.as_str();
            let stop_idx = self.get_or_insert_stop(stop_id, base_model);
            result.push(StopTime {
                stop: stop_idx,
                board_time: stop_time.departure_time,
                debark_time: stop_time.arrival_time,
                flow_direction: stop_time.flow_direction,
            });
        }
        result
    }

    fn get_or_insert_stop(&mut self, stop_id: &str, base_model: &BaseModel) -> StopPointIdx {
        if let Some(idx) = base_model.stop_point_idx(stop_id) {
            StopPointIdx::Base(idx)
        } else if let Some(idx) = self.new_stop_id_to_idx.get(stop_id) {
            StopPointIdx::New(idx.clone())
        } else {
            let idx = NewStopPointIdx {
                idx: self.new_stops.len(),
            };
            self.new_stop_id_to_idx
                .insert(stop_id.to_string(), idx.clone());
            StopPointIdx::New(idx)
        }
    }

    pub fn vehicle_journey_idx(
        &self,
        vehicle_journey_id: &str,
        base_model: &BaseModel,
    ) -> Option<VehicleJourneyIdx> {
        if let Some(transit_model_idx) = base_model.vehicle_journey_idx(vehicle_journey_id) {
            Some(VehicleJourneyIdx::Base(transit_model_idx))
        } else {
            let has_new_vj_idx = self.new_vehicle_journeys_id_to_idx.get(vehicle_journey_id);
            has_new_vj_idx.map(|new_vj_idx| VehicleJourneyIdx::New(new_vj_idx.clone()))
        }
    }

    pub fn stop_times<'a>(
        &'a self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &NaiveDate,
        from_stoptime_idx: StopTimeIdx,
        to_stoptime_idx: StopTimeIdx,
    ) -> Option<RealTimeStopTimes<'a>> {
        let trip_data = self.last_version(vehicle_journey_idx, date)?;

        if let TripData::Present(stop_times) = trip_data {
            let range = from_stoptime_idx.idx..=to_stoptime_idx.idx;
            let inner = stop_times[range].iter();
            Some(RealTimeStopTimes { inner })
        } else {
            None
        }
    }

    pub fn last_version(&self, idx: &VehicleJourneyIdx, date: &NaiveDate) -> Option<&TripData> {
        match idx {
            VehicleJourneyIdx::Base(base_idx) => {
                self.base_vehicle_journey_last_version(base_idx, date)
            }
            VehicleJourneyIdx::New(new_idx) => self.new_vehicle_journey_last_version(new_idx, date),
        }
    }

    pub(super) fn base_vehicle_journey_last_version(
        &self,
        idx: &BaseVehicleJourneyIdx,
        date: &NaiveDate,
    ) -> Option<&TripData> {
        self.base_vehicle_journeys_idx_to_history
            .get(idx)
            .and_then(|vehicle_journey_history| {
                vehicle_journey_history
                    .by_reference_date
                    .get(date)
                    .map(|trip_version| &trip_version.trip_data)
            })
    }

    pub(super) fn new_vehicle_journey_last_version(
        &self,
        idx: &NewVehicleJourneyIdx,
        date: &NaiveDate,
    ) -> Option<&TripData> {
        self.new_vehicle_journeys_history[idx.idx]
            .1
            .by_reference_date
            .get(date)
            .map(|trip_version| &trip_version.trip_data)
    }

    pub fn new_vehicle_journey_is_present(&self, trip: &Trip) -> bool {
        let has_new_vj_idx = self
            .new_vehicle_journeys_id_to_idx
            .get(&trip.vehicle_journey_id);
        if let Some(new_vj_idx) = has_new_vj_idx {
            self.new_vehicle_journey_last_version(new_vj_idx, &trip.reference_date)
                .is_some()
        } else {
            false
        }
    }

    pub fn nb_of_new_vehicle_journeys(&self) -> usize {
        self.new_vehicle_journeys_history.len()
    }

    pub fn new_vehicle_journeys(&self) -> impl Iterator<Item = NewVehicleJourneyIdx> {
        let range = 0..self.nb_of_new_vehicle_journeys();
        range.map(|idx| NewVehicleJourneyIdx { idx })
    }

    pub fn new() -> Self {
        Self {
            new_vehicle_journeys_id_to_idx: HashMap::new(),
            new_vehicle_journeys_history: Vec::new(),
            base_vehicle_journeys_idx_to_history: HashMap::new(),
            new_stop_id_to_idx: HashMap::new(),
            new_stops: Vec::new(),
            disruptions: Vec::new(),
        }
    }
}

impl Default for RealTimeModel {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for VehicleJourneyHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl VehicleJourneyHistory {
    pub fn new() -> Self {
        Self {
            by_reference_date: HashMap::new(),
            linked_disruption: HashMap::new(),
        }
    }
}

impl<'a> Iterator for RealTimeStopTimes<'a> {
    type Item = StopTime;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().cloned()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> ExactSizeIterator for RealTimeStopTimes<'a> {}
