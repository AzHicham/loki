use super::transit_data::{
    Stop, StopData,  TransitData, 
};
use super::timetables::{
    timetables_data::{Timetables, VehicleData, FlowDirection},
    insert::{VehicleTimesError}
    };
use super::time::{Calendar, PositiveDuration, SecondsSinceTimezonedDayStart};
use super::days_patterns::{DaysPatterns};
use transit_model::{
    model::Model,
    objects::{StopPoint, StopTime, Transfer as TransitModelTransfer, VehicleJourney},
};
use typed_index_collection::Idx;

use log::{info, warn, debug};

impl TransitData {
    pub fn new(transit_model: &Model, default_transfer_duration: PositiveDuration) -> Self {
        let nb_of_stop_points = transit_model.stop_points.len();

        let (start_date, end_date) = transit_model
            .calculate_validity_period()
            .expect("Unable to calculate a validity period.");
        let calendar = Calendar::new(start_date, end_date);
        let nb_of_days : usize = calendar.nb_of_days().into();
        let mut engine_data = Self {
            stop_point_idx_to_stop: std::collections::HashMap::new(),
            stops_data: Vec::with_capacity(nb_of_stop_points),
            timetables : Timetables::new(), 
            calendar,
            days_patterns : DaysPatterns::new(nb_of_days)
        };

        engine_data.init(transit_model, default_transfer_duration);

        engine_data
    }

    fn init(&mut self, transit_model: &Model, default_transfer_duration: PositiveDuration) {
        info!("Inserting vehicle journeys");
        for (vehicle_journey_idx, vehicle_journey) in transit_model.vehicle_journeys.iter() {
            self.insert_vehicle_journey(vehicle_journey_idx, vehicle_journey, transit_model);
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
                    warn!("Skipping transfer between {} and {} because at least one of this stop is not used by 
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
                    "Transfer {:?} is between stops which does not appears in the data. 
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
    ) {

        let mut stop_flows = {
            let mut result = Vec::with_capacity(vehicle_journey.stop_times.len());
            for (idx, stop_time) in vehicle_journey.stop_times.iter().enumerate() {
                let stop_point_idx = stop_time.stop_point_idx;
                let stop = self.stop_point_idx_to_stop.get(&stop_point_idx)
                    .cloned()
                    .unwrap_or_else( ||
                        self.add_new_stop_point(stop_point_idx)
                    );
                let to_push = match (stop_time.pickup_type, stop_time.drop_off_type) {
                    (0, 0) => (stop, FlowDirection::BoardAndDebark),
                    (1, 0) => (stop, FlowDirection::DebarkOnly),
                    (0, 1) => (stop, FlowDirection::BoardOnly),
                    _ => {
                        warn!("Skipping vehicle journey {} that has a bad {}th stop_time : \n {:#?} \n \
                        because of unhandled pickup type {} or dropoff type {}. ",
                        vehicle_journey.id,
                        idx,
                        stop_time,
                        stop_time.pickup_type,
                        stop_time.drop_off_type
                        );
                        return;
                    }
                };
                result.push(to_push);
            }
            result 
        };

        if stop_flows.len() < 2 {
            warn!(
                "Skipping vehicle journey {} that has less than 2 stop times.",
                vehicle_journey.id
            );
            return;
        }
        if stop_flows[0].1 != FlowDirection::BoardOnly {
            debug!(
                "First stop time of vehicle journey {} has debarked allowed. I ignore it.",
                vehicle_journey.id
            );
            stop_flows[0].1 = FlowDirection::BoardOnly;
        }
        if stop_flows.last().unwrap().1 != FlowDirection::DebarkOnly {
            debug!(
                "Last stop time of vehicle journey {} has boarding allowed. I ignore it.",
                vehicle_journey.id
            );
            stop_flows.last_mut().unwrap().1 = FlowDirection::DebarkOnly;
        }

        let has_transit_model_calendar = transit_model
            .calendars
            .get(&vehicle_journey.service_id);
        if has_transit_model_calendar.is_none() {
            warn!(
                "Skipping vehicle journey {} because its calendar {} was not found.",
                vehicle_journey.id, vehicle_journey.service_id, 
            );
            return;
        }
        let transit_model_calendar = has_transit_model_calendar.unwrap();

        let days_pattern = self
            .days_patterns
            .get_or_insert(transit_model_calendar.dates.iter(), &self.calendar);


        let has_route = transit_model.routes.get(&vehicle_journey.route_id);
        if has_route.is_none() {
            warn!(
                "Skipping vehicle journey {} because its route {} was not found.",
                vehicle_journey.id, vehicle_journey.route_id, 
            );
            return;
        };
        let route = has_route.unwrap();
        let has_line = transit_model.lines.get(&route.line_id);

        if has_line.is_none() {
            warn!(
                "Skipping vehicle journey {} because its line {} was not found.",
                vehicle_journey.id, route.line_id, 
            );
            return;
        }
        let line = has_line.unwrap();
        let has_network = transit_model.networks.get(&line.network_id);
        if has_network.is_none() {
            warn!(
                "Skipping vehicle journey {} because its network {} was not found.",
                vehicle_journey.id, line.network_id, 
            );
            return;
        }
        let network = has_network.unwrap();

        let timezone = {
            if network.timezone.is_none() {
                warn!(
                    "Skipping vehicle journey {} because its network {} has no timezone.",
                    vehicle_journey.id, line.network_id, 
                );
                return;
            };
            network.timezone.as_ref().unwrap()
        };

        for (idx, stop_time) in vehicle_journey.stop_times.iter().enumerate() {
            let has_board_time = board_time(stop_time);
            let has_debark_time = debark_time(stop_time);
            if has_board_time.is_none() || has_debark_time.is_none() {
                warn!("Skipping vehicle journey {} because I can't compute \
                       board and debark times for its {}th stop_time. \n {:#?}",
                      vehicle_journey.id,
                      idx,
                      stop_time
                );
                return 
            }
        }

        let board_debark_times = vehicle_journey
            .stop_times
            .iter()
            .map(|stop_time|  {
                // we can unwrap() here because we rejected the vehicle journey
                // above if some stop_time returns None on board_time or debark_time
                let board_time = board_time(stop_time).unwrap();
                let debark_time = debark_time(stop_time).unwrap();
                (board_time, debark_time)
            });


        let vehicle_data = VehicleData {
            vehicle_journey_idx,
            days_pattern,
        };

        let insert_error = self.timetables.insert(stop_flows.clone(), board_debark_times, &timezone, vehicle_data);
        match insert_error {
            Ok(timetable) => {
                for position in self.timetables.positions(&timetable) {
                    let stop = self.timetables.stop_at(&timetable, &position);
                    let stop_data = & mut self.stops_data[stop.idx];
                    stop_data.position_in_timetables.push(position);
                }
            },
            Err(err) =>  {
                match err {
                    VehicleTimesError::DebarkBeforeUpstreamBoard(position_pair) => {
                        let upstream_stop_time = &vehicle_journey.stop_times[position_pair.upstream];
                        let downstream_stop_time =
                            &vehicle_journey.stop_times[position_pair.downstream];
                        let board = board_time(upstream_stop_time).unwrap();
                        let debark = debark_time(downstream_stop_time).unwrap();
                        warn!(
                            "Skipping vehicle journey {} because its 
                                debark time {} at sequence {}
                                is earlier than its 
                                board time {} upstream at sequence {}. ",
                            vehicle_journey.id,
                            debark,
                            downstream_stop_time.sequence,
                            board,
                            upstream_stop_time.sequence
                        );
                    }
                    VehicleTimesError::DecreasingBoardTime(position_pair) => {
                        let upstream_stop_time = &vehicle_journey.stop_times[position_pair.upstream];
                        let downstream_stop_time =
                            &vehicle_journey.stop_times[position_pair.downstream];
                        let upstream_board = board_time(upstream_stop_time).unwrap();
                        let downstream_board = board_time(downstream_stop_time).unwrap();
                        warn!(
                            "Skipping vehicle journey {} because its 
                                board time {} at sequence {}
                                is earlier than its
                                board time {} upstream at sequence {}. ",
                            vehicle_journey.id,
                            downstream_board,
                            downstream_stop_time.sequence,
                            upstream_board,
                            upstream_stop_time.sequence
                        );
                    }
                    VehicleTimesError::DecreasingDebarkTime(position_pair) => {
                        let upstream_stop_time = &vehicle_journey.stop_times[position_pair.upstream];
                        let downstream_stop_time =
                            &vehicle_journey.stop_times[position_pair.downstream];
                        let upstream_debark = debark_time(upstream_stop_time).unwrap();
                        let downstream_debark = debark_time(downstream_stop_time).unwrap();
                        warn!(
                            "Skipping vehicle journey {} because its 
                                debark time {} at sequence {}
                                is earlier than its
                                debark time {} upstream at sequence {}. ",
                            vehicle_journey.id,
                            downstream_debark,
                            downstream_stop_time.sequence,
                            upstream_debark,
                            upstream_stop_time.sequence
                        );
                    }
                }
            }
        }
    }

    

    fn add_new_stop_point(&mut self, stop_point_idx: Idx<StopPoint>) -> Stop {
        debug_assert!(!self.stop_point_idx_to_stop.contains_key(&stop_point_idx));
        let stop_data = StopData {
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
