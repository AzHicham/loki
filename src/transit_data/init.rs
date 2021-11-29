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
        base_model::BaseModel, real_time_model::RealTimeModel, ModelRefs, StopPointIdx,
        TransferIdx, VehicleJourneyIdx,
    },
    transit_data::{Stop, TransitData},
    RealTimeLevel,
};

use crate::{
    time::{PositiveDuration, SecondsSinceTimezonedDayStart},
    timetables::{FlowDirection, Timetables as TimetablesTrait, TimetablesIter},
};
use transit_model::objects::{StopTime, VehicleJourney};
use typed_index_collection::Idx;

use tracing::{info, warn};

use super::{handle_insertion_error, Transfer, TransferData, TransferDurations};

impl<Timetables> TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a>,
{
    pub fn _new(
        base_model: &BaseModel,
        loads_data: &LoadsData,
        default_transfer_duration: PositiveDuration,
    ) -> Self {
        let nb_of_stop_points = base_model.stop_points.len();
        let nb_transfers = base_model.transfers.len();

        let (start_date, end_date) = base_model
            .calculate_validity_period()
            .expect("Unable to calculate a validity period.");

        let mut data = Self {
            stop_point_idx_to_stop: std::collections::HashMap::new(),
            stops_data: Vec::with_capacity(nb_of_stop_points),
            timetables: Timetables::new(start_date, end_date),
            transfers_data: Vec::with_capacity(nb_transfers),
        };

        data.init(base_model, loads_data, default_transfer_duration);

        data
    }

    fn init(
        &mut self,
        base_model: &BaseModel,
        loads_data: &LoadsData,
        default_transfer_duration: PositiveDuration,
    ) {
        info!("Inserting vehicle journeys");
        for (vehicle_journey_idx, vehicle_journey) in base_model.vehicle_journeys.iter() {
            let _ = self.insert_base_vehicle_journey(
                vehicle_journey_idx,
                vehicle_journey,
                base_model,
                loads_data,
            );
        }
        info!("Inserting transfers");

        for (transfer_idx, transfer) in base_model.transfers.iter() {
            let has_from_stop_point_idx = base_model.stop_points.get_idx(&transfer.from_stop_id);
            let has_to_stop_point_idx = base_model.stop_points.get_idx(&transfer.to_stop_id);
            match (has_from_stop_point_idx, has_to_stop_point_idx) {
                (Some(from_stop_point_idx), Some(to_stop_point_idx)) => {
                    let duration = transfer
                        .real_min_transfer_time
                        .map_or(default_transfer_duration, |seconds| PositiveDuration {
                            seconds,
                        });
                    let walking_duration = transfer
                        .min_transfer_time
                        .map_or(PositiveDuration::zero(), |seconds| PositiveDuration {
                            seconds,
                        });
                    let from_stop_point_idx = StopPointIdx::Base(from_stop_point_idx);
                    let to_stop_point_idx = StopPointIdx::Base(to_stop_point_idx);
                    let transfer_idx = TransferIdx::Base(transfer_idx);
                    self.insert_transfer(
                        from_stop_point_idx,
                        to_stop_point_idx,
                        transfer_idx,
                        duration,
                        walking_duration,
                    )
                }
                _ => {
                    warn!("Skipping transfer between {} and {} because at least one of this stop is not used by \
                           vehicles.", transfer.from_stop_id, transfer.to_stop_id);
                }
            }
        }
    }

    fn insert_transfer(
        &mut self,
        from_stop_point_idx: StopPointIdx,
        to_stop_point_idx: StopPointIdx,
        transfer_idx: TransferIdx,
        duration: PositiveDuration,
        walking_duration: PositiveDuration,
    ) {
        let has_from_stop = self.stop_point_idx_to_stop.get(&from_stop_point_idx);
        let has_to_stop = self.stop_point_idx_to_stop.get(&to_stop_point_idx);

        match (has_from_stop, has_to_stop) {
            (Some(from_stop), Some(to_stop)) => {
                let transfer = Transfer {
                    idx: self.transfers_data.len(),
                };
                let durations = TransferDurations {
                    total_duration: duration,
                    walking_duration,
                };
                let transfer_data = TransferData {
                    from_stop: *from_stop,
                    to_stop: *to_stop,
                    durations: durations.clone(),
                    transit_model_transfer_idx: transfer_idx,
                };
                self.transfers_data.push(transfer_data);
                let from_stop_data = &mut self.stops_data[from_stop.idx];
                from_stop_data.outgoing_transfers.push((
                    *to_stop,
                    durations.clone(),
                    transfer.clone(),
                ));
                let to_stop_data = &mut self.stops_data[to_stop.idx];
                to_stop_data
                    .incoming_transfers
                    .push((*from_stop, durations, transfer));
            }
            _ => {
                warn!(
                    "Transfer {:?} is between stops which does not appears in the data. \
                    I ignore it.",
                    transfer_idx
                );
            }
        }
    }

    fn insert_base_vehicle_journey(
        &mut self,
        vehicle_journey_idx: Idx<VehicleJourney>,
        vehicle_journey: &VehicleJourney,
        base_model: &BaseModel,
        loads_data: &LoadsData,
    ) -> Result<(), ()> {
        let stop_points = vehicle_journey
            .stop_times
            .iter()
            .map(|stop_time| StopPointIdx::Base(stop_time.stop_point_idx));

        let flows = create_flows_for_base_vehicle_journey(vehicle_journey)?;

        let model_calendar = base_model
            .calendars
            .get(&vehicle_journey.service_id)
            .ok_or_else(|| {
                warn!(
                    "Skipping vehicle journey {} because its calendar {} was not found.",
                    vehicle_journey.id, vehicle_journey.service_id,
                );
            })?;

        let dates = model_calendar.dates.iter();

        let timezone = timezone_of(vehicle_journey, base_model)?;

        let board_times = board_timezoned_times(vehicle_journey)?;
        let debark_times = debark_timezoned_times(vehicle_journey)?;

        let vehicle_journey_idx = VehicleJourneyIdx::Base(vehicle_journey_idx);

        let insert_result = self.insert_inner(
            stop_points,
            flows.into_iter(),
            board_times.into_iter(),
            debark_times.into_iter(),
            loads_data,
            dates,
            &timezone,
            vehicle_journey_idx,
            RealTimeLevel::Base,
        );

        let real_time_model = RealTimeModel::new();
        let model = ModelRefs {
            base: base_model,
            real_time: &real_time_model,
        };

        use crate::transit_data::data_interface::Data;
        if let Err(err) = insert_result {
            handle_insertion_error(
                &model,
                self.calendar().first_date(),
                self.calendar().last_date(),
                &err,
            );
        }

        Ok(())
    }

    fn add_new_stop_point(&mut self, stop_point_idx: StopPointIdx) -> Stop {
        debug_assert!(!self.stop_point_idx_to_stop.contains_key(&stop_point_idx));

        use super::StopData;
        let stop_data = StopData::<Timetables> {
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
                .cloned()
                .unwrap_or_else(|| self.add_new_stop_point(stop_point_idx));
            result.push(stop)
        }
        result
    }
}

pub fn create_flows_for_base_vehicle_journey(
    vehicle_journey: &VehicleJourney,
) -> Result<Vec<FlowDirection>, ()> {
    let mut result = Vec::with_capacity(vehicle_journey.stop_times.len());
    for (idx, stop_time) in vehicle_journey.stop_times.iter().enumerate() {
        let to_push = match (stop_time.pickup_type, stop_time.drop_off_type) {
            (0, 0) => FlowDirection::BoardAndDebark,
            (1, 0) => FlowDirection::DebarkOnly,
            (0, 1) => FlowDirection::BoardOnly,
            (1, 1) => FlowDirection::NoBoardDebark,
            _ => {
                warn!(
                    "Skipping vehicle journey {} that has a bad {}th stop_time : \n {:#?} \n \
                because of unhandled pickup type {} or dropoff type {}. ",
                    vehicle_journey.id,
                    idx,
                    stop_time,
                    stop_time.pickup_type,
                    stop_time.drop_off_type
                );
                return Err(());
            }
        };
        result.push(to_push);
    }

    if result.len() < 2 {
        warn!(
            "Skipping vehicle journey {} that has less than 2 stop times.",
            vehicle_journey.id
        );
        return Err(());
    }
    Ok(result)
}

fn board_time(stop_time: &StopTime) -> Option<SecondsSinceTimezonedDayStart> {
    use std::convert::TryFrom;
    let departure_seconds = i32::try_from(stop_time.departure_time.total_seconds()).ok()?;
    let boarding_duration = i32::try_from(stop_time.boarding_duration).ok()?;
    let seconds = departure_seconds.checked_sub(boarding_duration)?;
    SecondsSinceTimezonedDayStart::from_seconds(seconds)
}

fn debark_time(stop_time: &StopTime) -> Option<SecondsSinceTimezonedDayStart> {
    use std::convert::TryFrom;
    let arrival_seconds = i32::try_from(stop_time.arrival_time.total_seconds()).ok()?;
    let alighting_duration = i32::try_from(stop_time.alighting_duration).ok()?;
    let seconds = arrival_seconds.checked_add(alighting_duration)?;
    SecondsSinceTimezonedDayStart::from_seconds(seconds)
}

pub fn timezone_of(
    vehicle_journey: &VehicleJourney,
    base_model: &BaseModel,
) -> Result<chrono_tz::Tz, ()> {
    let has_route = base_model.routes.get(&vehicle_journey.route_id);
    if has_route.is_none() {
        warn!(
            "Skipping vehicle journey {} because its route {} was not found.",
            vehicle_journey.id, vehicle_journey.route_id,
        );
        return Err(());
    };
    let route = has_route.unwrap();
    let has_line = base_model.lines.get(&route.line_id);

    if has_line.is_none() {
        warn!(
            "Skipping vehicle journey {} because its line {} was not found.",
            vehicle_journey.id, route.line_id,
        );
        return Err(());
    }
    let line = has_line.unwrap();
    let has_network = base_model.networks.get(&line.network_id);
    if has_network.is_none() {
        warn!(
            "Skipping vehicle journey {} because its network {} was not found.",
            vehicle_journey.id, line.network_id,
        );
        return Err(());
    }
    let network = has_network.unwrap();

    let timezone = {
        if network.timezone.is_none() {
            warn!(
                "Skipping vehicle journey {} because its network {} has no timezone.",
                vehicle_journey.id, line.network_id,
            );
            return Err(());
        };
        network.timezone.unwrap()
    };
    Ok(timezone)
}

pub fn board_timezoned_times(
    vehicle_journey: &VehicleJourney,
) -> Result<Vec<SecondsSinceTimezonedDayStart>, ()> {
    let mut result = Vec::with_capacity(vehicle_journey.stop_times.len());
    for (idx, stop_time) in vehicle_journey.stop_times.iter().enumerate() {
        let board_time = board_time(stop_time).ok_or_else(|| {
            warn!(
                "Skipping vehicle journey {} because I can't compute \
                   board time for its {}th stop_time. \n {:#?}",
                vehicle_journey.id, idx, stop_time
            );
        })?;
        result.push(board_time);
    }

    Ok(result)
}

pub fn debark_timezoned_times(
    vehicle_journey: &VehicleJourney,
) -> Result<Vec<SecondsSinceTimezonedDayStart>, ()> {
    let mut result = Vec::with_capacity(vehicle_journey.stop_times.len());
    for (idx, stop_time) in vehicle_journey.stop_times.iter().enumerate() {
        let debark_time = debark_time(stop_time).ok_or_else(|| {
            warn!(
                "Skipping vehicle journey {} because I can't compute \
                   debark time for its {}th stop_time. \n {:#?}",
                vehicle_journey.id, idx, stop_time
            );
        })?;

        result.push(debark_time);
    }

    Ok(result)
}
