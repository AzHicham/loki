use crate::{loads_data::LoadsData, transit_data::{Stop, TransitData}};

use crate::time::{PositiveDuration, SecondsSinceTimezonedDayStart};
use crate::timetables::{FlowDirection, Timetables as TimetablesTrait, TimetablesIter};
use transit_model::{
    model::Model,
    objects::{StopPoint, StopTime, Transfer as TransitModelTransfer, VehicleJourney},
};
use typed_index_collection::Idx;

use log::{info, warn};

impl<Timetables> TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a>,
{
    pub fn _new(transit_model: &Model,
        loads_data : & LoadsData,
        default_transfer_duration: PositiveDuration
    ) -> Self {
        let nb_of_stop_points = transit_model.stop_points.len();

        let (start_date, end_date) = transit_model
            .calculate_validity_period()
            .expect("Unable to calculate a validity period.");
        let mut data = Self {
            stop_point_idx_to_stop: std::collections::HashMap::new(),
            stops_data: Vec::with_capacity(nb_of_stop_points),
            timetables: Timetables::new(start_date, end_date),
        };

        data.init(transit_model, loads_data, default_transfer_duration);

        data
    }

    fn init(&mut self, 
        transit_model: &Model, 
        loads_data : & LoadsData,
        default_transfer_duration: PositiveDuration
    ) {
        info!("Inserting vehicle journeys");
        for (vehicle_journey_idx, vehicle_journey) in transit_model.vehicle_journeys.iter() {
            let _ =
                self.insert_vehicle_journey(vehicle_journey_idx, 
                    vehicle_journey, 
                    transit_model,
                    loads_data
                );
        }
        info!("Inserting transfers");

        for (transfer_idx, transfer) in transit_model.transfers.iter() {
            let has_from_stop_point_idx = transit_model.stop_points.get_idx(&transfer.from_stop_id);
            let has_to_stop_point_idx = transit_model.stop_points.get_idx(&transfer.to_stop_id);
            match (has_from_stop_point_idx, has_to_stop_point_idx) {
                (Some(from_stop_point_idx), Some(to_stop_point_idx)) => {
                    let duration = transfer
                        .min_transfer_time
                        .map_or(default_transfer_duration, |seconds| PositiveDuration {
                            seconds,
                        });
                    self.insert_transfer(
                        from_stop_point_idx,
                        to_stop_point_idx,
                        transfer_idx,
                        duration,
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
        from_stop_point_idx: Idx<StopPoint>,
        to_stop_point_idx: Idx<StopPoint>,
        transfer_idx: Idx<TransitModelTransfer>,
        duration: PositiveDuration,
    ) {
        let has_from_stop = self.stop_point_idx_to_stop.get(&from_stop_point_idx);
        let has_to_stop = self.stop_point_idx_to_stop.get(&to_stop_point_idx);

        match (has_from_stop, has_to_stop) {
            (Some(from_stop), Some(to_stop)) => {
                let from_stop_data = &mut self.stops_data[from_stop.idx];
                from_stop_data
                    .transfers
                    .push((*to_stop, duration, transfer_idx));
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

    fn insert_vehicle_journey(
        &mut self,
        vehicle_journey_idx: Idx<VehicleJourney>,
        vehicle_journey: &VehicleJourney,
        transit_model: &Model,
        loads_data : & LoadsData
    ) -> Result<(), ()> {

        let stops = self.create_stops(vehicle_journey);
        let flows = self.create_flows(vehicle_journey)?;

        let model_calendar = transit_model
            .calendars
            .get(&vehicle_journey.service_id)
            .ok_or_else(|| {
                warn!(
                    "Skipping vehicle journey {} because its calendar {} was not found.",
                    vehicle_journey.id, vehicle_journey.service_id,
                );
            })?;
 
        let timezone = timezone_of(vehicle_journey, transit_model)?;

        let board_times = board_timezoned_times(vehicle_journey)?;
        let debark_times = debark_timezoned_times(vehicle_journey)?;

        let missions = self.timetables.insert(
            stops.into_iter(),
            flows.into_iter(),
            board_times.into_iter(),
            debark_times.into_iter(),
            loads_data,
            model_calendar.dates.iter(),
            &timezone,
            vehicle_journey_idx,
            vehicle_journey,
        );

        for mission in missions.iter() {
            for position in self.timetables.positions(&mission) {
                let stop = self.timetables.stop_at(&position, &mission);
                let stop_data = &mut self.stops_data[stop.idx];
                stop_data
                    .position_in_timetables
                    .push((mission.clone(), position));
            }
        }

        Ok(())
    }

    fn add_new_stop_point(&mut self, stop_point_idx: Idx<StopPoint>) -> Stop {
        debug_assert!(!self.stop_point_idx_to_stop.contains_key(&stop_point_idx));

        use super::StopData;
        let stop_data = StopData::<Timetables> {
            stop_point_idx,
            position_in_timetables: Vec::new(),
            transfers: Vec::new(),
        };
        let stop = Stop {
            idx: self.stops_data.len(),
        };
        self.stops_data.push(stop_data);
        self.stop_point_idx_to_stop.insert(stop_point_idx, stop);
        stop
    }


    fn create_stops(& mut self, vehicle_journey: &VehicleJourney) -> Vec<Stop>
    {
        let mut result = Vec::with_capacity(vehicle_journey.stop_times.len());
        for stop_time in vehicle_journey.stop_times.iter() {
            let stop_point_idx = &stop_time.stop_point_idx;
            let stop = self
                    .stop_point_idx_to_stop
                    .get(&stop_point_idx)
                    .cloned()
                    .unwrap_or_else(|| self.add_new_stop_point(*stop_point_idx));
            result.push(stop)
        }
        result
    }


    fn create_flows(& self, vehicle_journey: &VehicleJourney) -> Result<Vec<FlowDirection>, ()>
    {
        let mut result = Vec::with_capacity(vehicle_journey.stop_times.len());
        for (idx, stop_time) in vehicle_journey.stop_times.iter().enumerate() {
            let to_push = match (stop_time.pickup_type, stop_time.drop_off_type) {
                (0, 0) => FlowDirection::BoardAndDebark,
                (1, 0) => FlowDirection::DebarkOnly,
                (0, 1) => FlowDirection::BoardOnly,
                _ => {
                    warn!("Skipping vehicle journey {} that has a bad {}th stop_time : \n {:#?} \n \
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

fn timezone_of(
    vehicle_journey: &VehicleJourney,
    transit_model: &Model,
) -> Result<chrono_tz::Tz, ()> {
    let has_route = transit_model.routes.get(&vehicle_journey.route_id);
    if has_route.is_none() {
        warn!(
            "Skipping vehicle journey {} because its route {} was not found.",
            vehicle_journey.id, vehicle_journey.route_id,
        );
        return Err(());
    };
    let route = has_route.unwrap();
    let has_line = transit_model.lines.get(&route.line_id);

    if has_line.is_none() {
        warn!(
            "Skipping vehicle journey {} because its line {} was not found.",
            vehicle_journey.id, route.line_id,
        );
        return Err(());
    }
    let line = has_line.unwrap();
    let has_network = transit_model.networks.get(&line.network_id);
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
        network.timezone.clone().unwrap()
    };
    Ok(timezone)
}

fn board_timezoned_times(vehicle_journey: &VehicleJourney) -> Result<Vec<SecondsSinceTimezonedDayStart>, ()> {
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

fn debark_timezoned_times(
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
