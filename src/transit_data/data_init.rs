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
    loads_data::LoadsData,
    models::{
        base_model::{BaseModel, BaseTransferIdx},
        real_time_model::RealTimeModel,
        ModelRefs, StopPointIdx, TransferIdx, VehicleJourneyIdx,
    },
    time::{days_patterns::DaysPatterns, Calendar},
    timetables::{day_to_timetable::VehicleJourneyToTimetable, FlowDirection::*},
    transit_data::{data_interface::Data as DataInterface, Stop, TransitData},
    RealTimeLevel,
};

use crate::time::PositiveDuration;
use transit_model::objects::VehicleJourney;
use typed_index_collection::Idx;

use tracing::{info, warn};

use super::{handle_insertion_error, Timetables, Transfer, TransferData, TransferDurations};

impl TransitData {
    pub fn _new(base_model: &BaseModel) -> Self {
        let nb_of_stop_points = base_model.nb_of_stop_points();
        let nb_transfers = base_model.nb_of_transfers();

        let (start_date, end_date) = base_model.validity_period();
        let calendar = Calendar::new(start_date, end_date);
        let nb_of_days = calendar.nb_of_days();

        let mut data = Self {
            stop_point_idx_to_stop: std::collections::HashMap::new(),
            stops_data: Vec::with_capacity(nb_of_stop_points),
            timetables: Timetables::new(),
            transfers_data: Vec::with_capacity(nb_transfers),
            vehicle_journey_to_timetable: VehicleJourneyToTimetable::new(),
            calendar,
            days_patterns: DaysPatterns::new(usize::from(nb_of_days)),
        };

        data.init(base_model);

        data
    }

    fn init(&mut self, base_model: &BaseModel) {
        let loads_data = base_model.loads_data();
        info!("Inserting vehicle journeys");
        for vehicle_journey_idx in base_model.vehicle_journeys() {
            let _ = self.insert_base_vehicle_journey(vehicle_journey_idx, base_model, loads_data);
        }
        info!("Inserting transfers");

        for transfer_idx in base_model.transfers() {
            let _ = self.insert_base_transfer(transfer_idx, base_model)
                .map_err(|()| {
                    warn!(
                        "Skipping transfer between {} and {} because at least one of its stops is unknown. ",
                        base_model.from_stop_name(transfer_idx),
                        base_model.to_stop_name(transfer_idx),
                    );
                });
        }
    }

    fn insert_base_transfer(
        &mut self,
        transfer_idx: BaseTransferIdx,
        base_model: &BaseModel,
    ) -> Result<(), ()> {
        let from_base_idx = base_model.from_stop(transfer_idx).ok_or(())?;
        let from_idx = StopPointIdx::Base(from_base_idx);
        let from_stop = self.stop_point_idx_to_stop.get(&from_idx).ok_or(())?;
        let from_stop = *from_stop;

        let to_base_idx = base_model.to_stop(transfer_idx).ok_or(())?;
        let to_idx = StopPointIdx::Base(to_base_idx);
        let to_stop = self.stop_point_idx_to_stop.get(&to_idx).ok_or(())?;
        let to_stop = *to_stop;

        let duration = base_model.transfer_duration(transfer_idx);
        let walking_duration = base_model.transfer_walking_duration(transfer_idx);

        let transfer_idx = TransferIdx::Base(transfer_idx);

        self.insert_transfer_inner(from_stop, to_stop, transfer_idx, duration, walking_duration);

        Ok(())
    }

    fn insert_transfer_inner(
        &mut self,
        from_stop: Stop,
        to_stop: Stop,
        transfer_idx: TransferIdx,
        duration: PositiveDuration,
        walking_duration: PositiveDuration,
    ) {
        let transfer = Transfer {
            idx: self.transfers_data.len(),
        };
        let durations = TransferDurations {
            total_duration: duration,
            walking_duration,
        };
        let transfer_data = TransferData {
            from_stop,
            to_stop,
            durations: durations.clone(),
            transit_model_transfer_idx: transfer_idx,
        };
        self.transfers_data.push(transfer_data);
        let from_stop_data = &mut self.stops_data[from_stop.idx];
        from_stop_data
            .outgoing_transfers
            .push((to_stop, durations.clone(), transfer));
        let to_stop_data = &mut self.stops_data[to_stop.idx];
        to_stop_data
            .incoming_transfers
            .push((from_stop, durations, transfer));
    }

    fn insert_base_vehicle_journey(
        &mut self,
        vehicle_journey_idx: Idx<VehicleJourney>,
        base_model: &BaseModel,
        loads_data: &LoadsData,
    ) -> Result<(), ()> {
        let stop_times =
            base_model
                .stop_times(vehicle_journey_idx)
                .map_err(|(err, stop_time_idx)| {
                    warn!(
                    "Skipping vehicle journey {} because its {}-th stop time is ill formed {:?}.",
                    base_model.vehicle_journey_name(vehicle_journey_idx),
                    stop_time_idx.idx,
                    err
                );
                })?;
        // let rt_model = RealTimeModel::new();
        // let refs = ModelRefs::new(base_model, &rt_model);
        // debug!("Trying to insert vj {} on stops : \n {:#?}",
        //     base_model.vehicle_journey_name(vehicle_journey_idx),
        //     stop_times.clone().map(|stop_time| refs.stop_point_uri(&stop_time.stop)).collect::<Vec<_>>()
        // );
        if stop_times.len() < 2 {
            warn!(
                "Skipping vehicle journey {} because it has less than 2 stop times.",
                base_model.vehicle_journey_name(vehicle_journey_idx),
            );
            return Err(());
        }

        let dates = base_model
            .vehicle_journey_dates(vehicle_journey_idx)
            .ok_or_else(|| {
                warn!(
                    "Skipping vehicle journey {} because it has no dates.",
                    base_model.vehicle_journey_name(vehicle_journey_idx)
                );
            })?;

        let timezone = base_model.timezone(vehicle_journey_idx).ok_or_else(|| {
            warn!(
                "Skipping vehicle journey {} because it has no timezone.",
                base_model.vehicle_journey_name(vehicle_journey_idx)
            );
        })?;

        let stops = stop_times.clone().map(|s| s.stop);
        let flows = stop_times.clone().map(|s| s.flow_direction);
        let board_times = stop_times.clone().map(|s| s.board_time);
        let debark_times = stop_times.clone().map(|s| s.debark_time);

        let vehicle_journey_idx = VehicleJourneyIdx::Base(vehicle_journey_idx);

        let mut local_zones: Vec<_> = stop_times.clone().map(|s| s.local_zone_id).collect();
        local_zones.sort_unstable();
        local_zones.dedup();

        if local_zones.len() == 1 {
            let insert_result = self.insert_inner(
                stops,
                flows,
                board_times,
                debark_times,
                loads_data,
                dates,
                timezone,
                vehicle_journey_idx,
                local_zones[0],
                RealTimeLevel::Base,
            );
            let real_time_model = RealTimeModel::new();
            let model = ModelRefs {
                base: base_model,
                real_time: &real_time_model,
            };

            if let Err(err) = insert_result {
                handle_insertion_error(
                    &model,
                    self.calendar().first_date(),
                    self.calendar().last_date(),
                    &err,
                );
            }
        } else {
            for local_zone in local_zones {
                // we change the flows regarding the `local_zone` so that:
                // - we can only board on stops that belong to `local_zone`
                // - we can only debark on stops that don't belong to `local_zone`
                let local_flows = stop_times.clone().map(|stop_time| {
                    if stop_time.local_zone_id == local_zone {
                        match stop_time.flow_direction {
                            BoardOnly | BoardAndDebark => BoardOnly,
                            DebarkOnly | NoBoardDebark => NoBoardDebark,
                        }
                    } else {
                        match stop_time.flow_direction {
                            BoardOnly | NoBoardDebark => NoBoardDebark,
                            DebarkOnly | BoardAndDebark => DebarkOnly,
                        }
                    }
                });
                let insert_result = self.insert_inner(
                    stops.clone(),
                    local_flows,
                    board_times.clone(),
                    debark_times.clone(),
                    loads_data,
                    dates.clone(),
                    timezone,
                    vehicle_journey_idx.clone(),
                    local_zone,
                    RealTimeLevel::Base,
                );

                let real_time_model = RealTimeModel::new();
                let model = ModelRefs {
                    base: base_model,
                    real_time: &real_time_model,
                };

                if let Err(err) = insert_result {
                    handle_insertion_error(
                        &model,
                        self.calendar().first_date(),
                        self.calendar().last_date(),
                        &err,
                    );
                }
            }
        }

        Ok(())
    }

    fn add_new_stop_point(&mut self, stop_point_idx: StopPointIdx) -> Stop {
        use super::StopData;

        debug_assert!(!self.stop_point_idx_to_stop.contains_key(&stop_point_idx));

        let stop_data = StopData {
            stop_point_idx: stop_point_idx.clone(),
            position_in_timetables: Vec::new(),
            incoming_transfers: Vec::new(),
            outgoing_transfers: Vec::new(),
        };
        let stop = Stop {
            idx: self.stops_data.len(),
        };
        self.stops_data.push(stop_data);
        self.stop_point_idx_to_stop.insert(stop_point_idx, stop);
        stop
    }

    pub(super) fn create_stops<StopPoints: Iterator<Item = StopPointIdx>>(
        &mut self,
        stop_points: StopPoints,
    ) -> Vec<Stop> {
        let mut result = Vec::new();
        for stop_point_idx in stop_points {
            let stop = self
                .stop_point_idx_to_stop
                .get(&stop_point_idx)
                .copied()
                .unwrap_or_else(|| self.add_new_stop_point(stop_point_idx));
            result.push(stop);
        }
        result
    }
}