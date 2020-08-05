use super::calendar::Calendar;
use super::transit_data::{
    FlowDirection, Stop, StopData, StopPattern, StopPoints, TransitData, TransitModelTime,
    VehicleData,
};
use super::ordered_timetable::{StopPatternData, VehicleTimesError};
use super::time::{PositiveDuration, SecondsSinceDayStart};
use std::collections::BTreeMap;
use transit_model::{
    model::Model,
    objects::{StopPoint, StopTime, Transfer as TransitModelTransfer, VehicleJourney},
};
use typed_index_collection::Idx;

use chrono_tz::Tz as TimeZone;

use log::{info, warn, debug};

impl TransitData {
    pub fn new(transit_model: &Model, default_transfer_duration: PositiveDuration) -> Self {
        let nb_of_stop_points = transit_model.stop_points.len();

        let (start_date, end_date) = transit_model
            .calculate_validity_period()
            .expect("Unable to calculate a validity period.");

        let mut engine_data = Self {
            stop_points_to_pattern: BTreeMap::new(),
            stop_point_idx_to_stop: std::collections::HashMap::new(),
            stops_data: Vec::with_capacity(nb_of_stop_points),
            patterns: Vec::new(),
            calendar: Calendar::new(start_date, end_date),
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

        let mut stop_points = {
            let mut result = Vec::with_capacity(vehicle_journey.stop_times.len());
            for (idx, stop_time) in vehicle_journey.stop_times.iter().enumerate() {
                let stop_point_idx = stop_time.stop_point_idx;
                let to_push = match (stop_time.pickup_type, stop_time.drop_off_type) {
                    (0, 0) => (stop_point_idx, FlowDirection::BoardAndDebark),
                    (1, 0) => (stop_point_idx, FlowDirection::DebarkOnly),
                    (0, 1) => (stop_point_idx, FlowDirection::BoardOnly),
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

        if stop_points.len() < 2 {
            warn!(
                "Skipping vehicle journey {} that has less than 2 stop times.",
                vehicle_journey.id
            );
            return;
        }
        if stop_points[0].1 != FlowDirection::BoardOnly {
            debug!(
                "First stop time of vehicle journey {} has debarked allowed. I ignore it.",
                vehicle_journey.id
            );
            stop_points[0].1 = FlowDirection::BoardOnly;
        }
        if stop_points.last().unwrap().1 != FlowDirection::DebarkOnly {
            debug!(
                "Last stop time of vehicle journey {} has boarding allowed. I ignore it.",
                vehicle_journey.id
            );
            stop_points.last_mut().unwrap().1 = FlowDirection::DebarkOnly;
        }

        let has_pattern = self.stop_points_to_pattern.get(&stop_points);
        let pattern = if let Some(pattern_) = has_pattern {
            *pattern_
        } else {
            self.create_new_pattern(stop_points)
        };

        let pattern_data = &mut self.patterns[pattern.idx];

        let board_debark_times = vehicle_journey
            .stop_times
            .iter()
            .map(|stop_time| (board_time(stop_time), debark_time(stop_time)));

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
            .calendar
            .get_or_insert(transit_model_calendar.dates.iter());


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
            let timezone_string = network.timezone.as_ref().unwrap();
            let has_timezone : Result<TimeZone, _> = timezone_string.parse();
            match has_timezone {
                Result::Err(err) => {
                    warn!(
                        "Skipping vehicle journey {} because I can't parse its timezone {}. \n {}",
                        vehicle_journey.id, timezone_string, err, 
                    );
                    return;
                }
                Result::Ok(timezone) => {
                    timezone
                }
            }
        };


        let daily_trip_data = VehicleData {
            vehicle_journey_idx,
            days_pattern,
        };

        let insert_error = pattern_data.insert(board_debark_times, &timezone, daily_trip_data);
        if let Err(err) = insert_error {
            match err {
                VehicleTimesError::DebarkBeforeUpstreamBoard(position_pair) => {
                    let upstream_stop_time = &vehicle_journey.stop_times[position_pair.upstream];
                    let downstream_stop_time =
                        &vehicle_journey.stop_times[position_pair.downstream];
                    let board = board_time(upstream_stop_time);
                    let debark = debark_time(downstream_stop_time);
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
                    let upstream_board = board_time(upstream_stop_time);
                    let downstream_board = board_time(downstream_stop_time);
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
                    let upstream_debark = debark_time(upstream_stop_time);
                    let downstream_debark = debark_time(downstream_stop_time);
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

    fn create_new_pattern(&mut self, stop_points: StopPoints) -> StopPattern {
        debug_assert!(!self.stop_points_to_pattern.contains_key(&stop_points));

        let nb_of_positions = stop_points.len();
        let pattern = StopPattern {
            idx: self.patterns.len(),
        };

        let mut stops = Vec::with_capacity(nb_of_positions);
        let mut flow_directions = Vec::with_capacity(nb_of_positions);
        for (stop_point_idx, flow_direction) in stop_points.iter() {
            let has_stop = self.stop_point_idx_to_stop.get(stop_point_idx);
            let stop = match has_stop {
                None => {
                    self.add_new_stop_point(*stop_point_idx)
                }
                Some(&stop) => stop,
            };
            stops.push(stop);
            flow_directions.push(*flow_direction);
        }

        let pattern_data = StopPatternData::new(stops, flow_directions);

        self.stop_points_to_pattern.insert(stop_points, pattern);

        for (stop, position) in pattern_data.stops_and_positions() {
            let stop_data = &mut self.stops_data[stop.idx];
            stop_data.position_in_patterns.push((pattern, position));
        }

        self.patterns.push(pattern_data);

        pattern
    }

    fn add_new_stop_point(&mut self, stop_point_idx: Idx<StopPoint>) -> Stop {
        debug_assert!(!self.stop_point_idx_to_stop.contains_key(&stop_point_idx));
        let stop_data = StopData {
            stop_point_idx,
            position_in_patterns: Vec::new(),
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

fn board_time(stop_time: &StopTime) -> SecondsSinceDayStart {
    let transit_model_time =
        stop_time.departure_time - TransitModelTime::new(0, 0, stop_time.boarding_duration.into());
    let seconds = transit_model_time.total_seconds();
    SecondsSinceDayStart { seconds }
}

fn debark_time(stop_time: &StopTime) -> SecondsSinceDayStart {
    let transit_model_time =
        stop_time.arrival_time + TransitModelTime::new(0, 0, stop_time.alighting_duration.into());
    let seconds = transit_model_time.total_seconds();
    SecondsSinceDayStart { seconds }
}
