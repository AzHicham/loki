// Copyright  (C) 2020, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Hove (www.kisio.com).
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
    real_time_disruption::{
        chaos_disruption::{ChaosDisruption, ChaosImpact},
        kirin_disruption::{self, KirinDisruption},
    },
    StopPointIdx, StopTime, StopTimeIdx, VehicleJourneyIdx,
};

pub struct RealTimeModel {
    pub(super) new_vehicle_journeys_id_to_idx: HashMap<String, NewVehicleJourneyIdx>,
    // indexed by NewVehicleJourney.idx
    pub(super) new_vehicle_journeys_history: Vec<(String, VehicleJourneyHistory)>,

    pub(super) base_vehicle_journeys_idx_to_history:
        HashMap<BaseVehicleJourneyIdx, VehicleJourneyHistory>,

    pub(super) new_stop_id_to_idx: HashMap<String, NewStopPointIdx>,
    pub(super) new_stops: Vec<StopData>,

    pub(super) chaos_disruptions: Vec<ChaosDisruption>,

    pub(super) kirin_disruptions: Vec<KirinDisruption>,
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct NewVehicleJourneyIdx {
    pub idx: usize, // position in new_vehicle_journeys_history
}

pub type LinkedChaosImpacts = Vec<(ChaosImpactIdx, ChaosImpactObjectIdx)>;

#[derive(Debug, Clone)]
pub struct VehicleJourneyHistory {
    by_reference_date: HashMap<NaiveDate, TripVersion>,

    // provides all chaos impacts that affect this (vehicle_journey, date)
    linked_chaos_impacts: HashMap<NaiveDate, LinkedChaosImpacts>,
    // provides the kirin disruption (if any) that affect this (vehicle_journey, date)
    linked_kirin_disruption: HashMap<NaiveDate, KirinDisruptionIdx>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChaosImpactIdx {
    pub(super) disruption_idx: usize, // position in RealTimeModel.chaos_disruptions
    pub(super) impact_idx: usize,     // position in ChaosDisruption.impacts
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChaosImpactObjectIdx {
    Impacted(usize), // position in ChaosImpact.impacted
    Informed(usize), // position in ChaosImpact.informed
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct KirinDisruptionIdx {
    pub(super) idx: usize, // position in RealTimeModel.kirin_disruptions
}

#[derive(Debug, Clone)]
pub enum TripVersion {
    Deleted(),              // the trip is currently disabled
    Present(Vec<StopTime>), // list of all stop times of this trip
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
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

impl RealTimeModel {
    pub fn set_base_trip_version(
        &mut self,
        vehicle_journey_idx: BaseVehicleJourneyIdx,
        date: &NaiveDate,
        trip_version: TripVersion,
    ) -> Option<TripVersion> {
        let history = self
            .base_vehicle_journeys_idx_to_history
            .entry(vehicle_journey_idx)
            .or_insert_with(VehicleJourneyHistory::new);

        history.by_reference_date.insert(*date, trip_version)
    }

    pub fn set_new_trip_version(
        &mut self,
        vehicle_journey_idx: NewVehicleJourneyIdx,
        date: &NaiveDate,
        trip_version: TripVersion,
    ) -> Option<TripVersion> {
        let history = &mut self.new_vehicle_journeys_history[vehicle_journey_idx.idx].1;

        history.by_reference_date.insert(*date, trip_version)
    }

    pub fn insert_new_vehicle_journey(&mut self, vehicle_journey_id: &str) -> NewVehicleJourneyIdx {
        let histories = &mut self.new_vehicle_journeys_history;
        let idx = self
            .new_vehicle_journeys_id_to_idx
            .entry(vehicle_journey_id.to_string())
            .or_insert_with(|| {
                let idx = histories.len();
                histories.push((vehicle_journey_id.to_string(), VehicleJourneyHistory::new()));
                NewVehicleJourneyIdx { idx }
            });

        *idx
    }

    pub fn set_linked_kirin_disruption(
        &mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: NaiveDate,
        kirin_disruption_idx: KirinDisruptionIdx,
    ) -> Option<KirinDisruptionIdx> {
        let history = match vehicle_journey_idx {
            VehicleJourneyIdx::Base(base_idx) => self
                .base_vehicle_journeys_idx_to_history
                .entry(*base_idx)
                .or_insert_with(VehicleJourneyHistory::new),
            VehicleJourneyIdx::New(new_idx) => {
                &mut self.new_vehicle_journeys_history[new_idx.idx].1
            }
        };
        history
            .linked_kirin_disruption
            .insert(date, kirin_disruption_idx)
    }

    pub fn unset_linked_kirin_disruption(
        &mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: NaiveDate,
    ) {
        let history = match vehicle_journey_idx {
            VehicleJourneyIdx::Base(base_idx) => self
                .base_vehicle_journeys_idx_to_history
                .entry(*base_idx)
                .or_insert_with(VehicleJourneyHistory::new),
            VehicleJourneyIdx::New(new_idx) => {
                &mut self.new_vehicle_journeys_history[new_idx.idx].1
            }
        };
        history.linked_kirin_disruption.remove(&date);
    }

    pub fn get_linked_kirin_disruption(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: NaiveDate,
    ) -> Option<&KirinDisruptionIdx> {
        let history = match vehicle_journey_idx {
            VehicleJourneyIdx::Base(base_idx) => {
                self.base_vehicle_journeys_idx_to_history.get(base_idx)?
            }
            VehicleJourneyIdx::New(new_idx) => &self.new_vehicle_journeys_history[new_idx.idx].1,
        };
        history.linked_kirin_disruption.get(&date)
    }

    pub fn link_chaos_impact(
        &mut self,
        base_vehicle_journey_idx: BaseVehicleJourneyIdx,
        date: NaiveDate,
        base_model: &BaseModel,
        chaos_impact_idx: &ChaosImpactIdx,
        impact_object_idx: &ChaosImpactObjectIdx,
    ) {
        let history = self
            .base_vehicle_journeys_idx_to_history
            .entry(base_vehicle_journey_idx)
            .or_insert_with(VehicleJourneyHistory::new);

        let linked_impacts = history
            .linked_chaos_impacts
            .entry(date)
            .or_insert_with(LinkedChaosImpacts::new);

        let find_disruption_impact = linked_impacts.iter().find(|(impact_idx, object_idx)| {
            *impact_idx == *chaos_impact_idx && *object_idx == *impact_object_idx
        });

        match find_disruption_impact {
            Some(_) => {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                warn!(
                    "Chaos impact : {:?} already linked to vehicle_journey {} on date {}",
                    chaos_impact_idx, vehicle_journey_id, date
                );
            }
            None => {
                linked_impacts.push((chaos_impact_idx.clone(), impact_object_idx.clone()));
            }
        }
    }

    pub fn unlink_chaos_impact(
        &mut self,
        base_vehicle_journey_idx: BaseVehicleJourneyIdx,
        date: NaiveDate,
        base_model: &BaseModel,
        chaos_impact_idx: &ChaosImpactIdx,
        impact_object_idx: &ChaosImpactObjectIdx,
    ) {
        let history = self
            .base_vehicle_journeys_idx_to_history
            .entry(base_vehicle_journey_idx)
            .or_insert_with(VehicleJourneyHistory::new);

        let linked_impacts = history
            .linked_chaos_impacts
            .entry(date)
            .or_insert_with(LinkedChaosImpacts::new);

        let find_disruption_impact = linked_impacts.iter().find(|(impact_idx, object_idx)| {
            *impact_idx == *chaos_impact_idx && *object_idx == *impact_object_idx
        });

        match find_disruption_impact {
            Some(_) => {
                linked_impacts.retain(|(impact_idx, object_idx)| {
                    !(*impact_idx == *chaos_impact_idx && *object_idx == *impact_object_idx)
                });
            }
            None => {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                warn!(
                    "Cannot unlink absent chaos impact {:?} on vehicle_journey {} on date {}",
                    chaos_impact_idx, vehicle_journey_id, date
                );
            }
        }
    }

    pub fn get_linked_chaos_impacts(
        &self,
        base_vehicle_journey_idx: BaseVehicleJourneyIdx,
        date: NaiveDate,
    ) -> Option<&[(ChaosImpactIdx, ChaosImpactObjectIdx)]> {
        let history = self
            .base_vehicle_journeys_idx_to_history
            .get(&base_vehicle_journey_idx)?;
        let impacts = history.linked_chaos_impacts.get(&date)?;
        Some(impacts.as_slice())
    }

    pub fn get_chaos_disruption(
        &self,
        chaos_impact_idx: &ChaosImpactIdx,
    ) -> Option<(&ChaosDisruption, &ChaosImpact)> {
        let disruption = self
            .chaos_disruptions
            .get(chaos_impact_idx.disruption_idx)?;
        let impact = disruption.impacts.get(chaos_impact_idx.impact_idx)?;
        Some((disruption, impact))
    }

    pub fn get_kirin_disruption(
        &self,
        kirin_disruption_idx: KirinDisruptionIdx,
    ) -> &KirinDisruption {
        &self.kirin_disruptions[kirin_disruption_idx.idx]
    }

    pub fn make_stop_times(
        &mut self,
        stop_times: &[kirin_disruption::StopTime],
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
                local_zone_id: None,
            });
        }
        result
    }

    fn get_or_insert_stop(&mut self, stop_id: &str, base_model: &BaseModel) -> StopPointIdx {
        self.stop_point_idx(stop_id, base_model).unwrap_or_else(|| {
            let idx = NewStopPointIdx {
                idx: self.new_stops.len(),
            };
            self.new_stop_id_to_idx.insert(stop_id.to_string(), idx);
            StopPointIdx::New(idx)
        })
    }

    pub fn stop_point_idx(&self, stop_id: &str, base_model: &BaseModel) -> Option<StopPointIdx> {
        if let Some(idx) = base_model.stop_point_idx(stop_id) {
            Some(StopPointIdx::Base(idx))
        } else {
            self.new_stop_id_to_idx
                .get(stop_id)
                .map(|idx| StopPointIdx::New(*idx))
        }
    }

    pub fn new_vehicle_journey_idx(
        &self,
        vehicle_journey_id: &str,
    ) -> Option<NewVehicleJourneyIdx> {
        let has_new_vj_idx = self.new_vehicle_journeys_id_to_idx.get(vehicle_journey_id);
        has_new_vj_idx.copied()
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
            has_new_vj_idx.map(|new_vj_idx| VehicleJourneyIdx::New(*new_vj_idx))
        }
    }

    pub fn stop_times<'a>(
        &'a self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: NaiveDate,
        from_stoptime_idx: StopTimeIdx,
        to_stoptime_idx: StopTimeIdx,
    ) -> Option<RealTimeStopTimes<'a>> {
        let trip_data = self.last_version(vehicle_journey_idx, date)?;

        if let TripVersion::Present(stop_times) = trip_data {
            let range = from_stoptime_idx.idx..=to_stoptime_idx.idx;
            let inner = stop_times[range].iter();
            Some(RealTimeStopTimes { inner })
        } else {
            None
        }
    }

    pub fn last_version(&self, idx: &VehicleJourneyIdx, date: NaiveDate) -> Option<&TripVersion> {
        match idx {
            VehicleJourneyIdx::Base(base_idx) => {
                self.base_vehicle_journey_last_version(*base_idx, date)
            }
            VehicleJourneyIdx::New(new_idx) => {
                self.new_vehicle_journey_last_version(*new_idx, date)
            }
        }
    }

    pub(super) fn base_vehicle_journey_last_version(
        &self,
        idx: BaseVehicleJourneyIdx,
        date: NaiveDate,
    ) -> Option<&TripVersion> {
        self.base_vehicle_journeys_idx_to_history
            .get(&idx)
            .and_then(|vehicle_journey_history| {
                vehicle_journey_history.by_reference_date.get(&date)
            })
    }

    pub fn base_vehicle_journey_is_present(
        &self,
        idx: BaseVehicleJourneyIdx,
        date: NaiveDate,
        base_model: &BaseModel,
    ) -> bool {
        let last_version = self.base_vehicle_journey_last_version(idx, date);
        match last_version {
            Some(&TripVersion::Deleted()) => false,
            Some(&TripVersion::Present(_)) => true,
            None => base_model.trip_exists(idx, date),
        }
    }

    pub(super) fn new_vehicle_journey_last_version(
        &self,
        idx: NewVehicleJourneyIdx,
        date: NaiveDate,
    ) -> Option<&TripVersion> {
        self.new_vehicle_journeys_history[idx.idx]
            .1
            .by_reference_date
            .get(&date)
    }

    pub fn new_vehicle_journey_is_present(
        &self,
        idx: NewVehicleJourneyIdx,
        date: NaiveDate,
    ) -> bool {
        match self.new_vehicle_journey_last_version(idx, date) {
            Some(TripVersion::Present(_)) => true,
            _ => false,
        }
    }

    pub fn nb_of_new_vehicle_journeys(&self) -> usize {
        self.new_vehicle_journeys_history.len()
    }

    pub fn new_vehicle_journeys(&self) -> impl Iterator<Item = NewVehicleJourneyIdx> {
        let range = 0..self.nb_of_new_vehicle_journeys();
        range.map(|idx| NewVehicleJourneyIdx { idx })
    }

    pub fn get_chaos_disruption_and_impact(
        &self,
        chaos_impact_idx: &ChaosImpactIdx,
    ) -> (&ChaosDisruption, &ChaosImpact) {
        let disruption = &self.chaos_disruptions[chaos_impact_idx.disruption_idx];
        let impact = &disruption.impacts[chaos_impact_idx.impact_idx];
        (disruption, impact)
    }

    pub fn new() -> Self {
        Self {
            new_vehicle_journeys_id_to_idx: HashMap::new(),
            new_vehicle_journeys_history: Vec::new(),
            base_vehicle_journeys_idx_to_history: HashMap::new(),
            new_stop_id_to_idx: HashMap::new(),
            new_stops: Vec::new(),
            chaos_disruptions: Vec::new(),
            kirin_disruptions: Vec::new(),
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
            linked_chaos_impacts: HashMap::new(),
            linked_kirin_disruption: HashMap::new(),
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
