use super::transit_data::{
    Stop, StopData,  TransitData, 
};
use super::timetables::{
    timetables_data::{Timetables, VehicleData, FlowDirection},
    insert::{VehicleTimesError}
    };
use super::time::{Calendar, PositiveDuration, SecondsSinceTimezonedDayStart, SecondsSinceDatasetUTCStart};
use transit_model::{
    model::Model,
    objects::{StopPoint, StopTime, Transfer as TransitModelTransfer, VehicleJourney},
};
use typed_index_collection::Idx;

use log::{info, warn, debug};

use chrono::NaiveDate;

impl TransitData {
    pub fn new(transit_model: &Model, default_transfer_duration: PositiveDuration) -> Self {
        let nb_of_stop_points = transit_model.stop_points.len();

        let (start_date, end_date) = transit_model
            .calculate_validity_period()
            .expect("Unable to calculate a validity period.");
        let calendar = Calendar::new(start_date, end_date);
        let mut engine_data = Self {
            stop_point_idx_to_stop: std::collections::HashMap::new(),
            stops_data: Vec::with_capacity(nb_of_stop_points),
            timetables : Timetables::new(), 
            calendar
        };

        engine_data.init(transit_model, default_transfer_duration);

        engine_data
    }

    fn init(&mut self, transit_model: &Model, default_transfer_duration: PositiveDuration) {
        info!("Inserting vehicle journeys");
        for (vehicle_journey_idx, vehicle_journey) in transit_model.vehicle_journeys.iter() {
            let _ = self.insert_vehicle_journey(vehicle_journey_idx, vehicle_journey, transit_model);
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
    ) -> Result<(), ()>
    {

        let stop_flows = self.create_stop_flows(vehicle_journey)?;



        let transit_model_calendar = transit_model
            .calendars
            .get(&vehicle_journey.service_id).ok_or_else( || {
                warn!(
                    "Skipping vehicle journey {} because its calendar {} was not found.",
                    vehicle_journey.id, vehicle_journey.service_id, 
                );
            })?;

        let timezone = timezone_of(vehicle_journey, transit_model)?;

        let board_debark_timezoned_times = board_debark_timezoned_times_in_day(vehicle_journey)?;



        for date in transit_model_calendar.dates.iter() {

            let vehicle_data = VehicleData {
                vehicle_journey_idx,
                date : date.clone(),
            };

            let has_board_debark_utc_times = board_debark_utc_times(&board_debark_timezoned_times, date, &timezone, &self.calendar, vehicle_journey);
            if let Ok(board_debark_utc_times) = has_board_debark_utc_times {
                let insert_error = self.timetables.insert(
                    stop_flows.clone(), 
                    board_debark_utc_times.iter().map(|pair| pair.clone()), 
                    &timezone, 
                    vehicle_data.clone()
                );
                match insert_error {
                    Ok(timetable) => {
                        for position in self.timetables.positions(&timetable) {
                            let stop = self.timetables.stop_at(&timetable, &position);
                            let stop_data = & mut self.stops_data[stop.idx];
                            stop_data.position_in_timetables.push(position);
                        }
                    },
                    Err(error) =>  {
                        handle_vehicletimes_error(vehicle_journey, date, &error);
                    }
                }
            }
             
        }

        Ok(())
        
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


    fn create_stop_flows(& mut self, vehicle_journey : & VehicleJourney) -> Result<Vec<(Stop,FlowDirection)>, ()> {
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
                        return Err(());
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
            return Err(());
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

        Ok(stop_flows)
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

fn timezone_of(vehicle_journey : & VehicleJourney, transit_model : & Model) -> Result<chrono_tz::Tz, ()> {
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


fn handle_vehicletimes_error(vehicle_journey : & VehicleJourney, date : & NaiveDate, error : & VehicleTimesError) {
    match error {
        VehicleTimesError::DebarkBeforeUpstreamBoard(position_pair) => {
            let upstream_stop_time = &vehicle_journey.stop_times[position_pair.upstream];
            let downstream_stop_time =
                &vehicle_journey.stop_times[position_pair.downstream];
            let board = board_time(upstream_stop_time).unwrap();
            let debark = debark_time(downstream_stop_time).unwrap();
            warn!(
                "Skipping vehicle journey {} on day {} because its \
                    debark time {} at sequence {}\
                    is earlier than its \
                    board time {} upstream at sequence {}. ",
                vehicle_journey.id,
                date,
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
                "Skipping vehicle journey {} on day {} because its \
                    board time {} at sequence {} \
                    is earlier than its \
                    board time {} upstream at sequence {}. ",
                vehicle_journey.id,
                date,
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
                "Skipping vehicle journey {} on day {} because its \
                    debark time {} at sequence {} \
                    is earlier than its \
                    debark time {} upstream at sequence {}. ",
                vehicle_journey.id,
                date,
                downstream_debark,
                downstream_stop_time.sequence,
                upstream_debark,
                upstream_stop_time.sequence
            );
        }
    }
}

fn board_debark_timezoned_times_in_day(vehicle_journey : & VehicleJourney) -> Result<  Vec<(SecondsSinceTimezonedDayStart, SecondsSinceTimezonedDayStart)> , ()>
{
    let mut result = Vec::with_capacity(vehicle_journey.stop_times.len());
    for (idx, stop_time) in vehicle_journey.stop_times.iter().enumerate() {
        let board_time = board_time(stop_time).ok_or_else( || {
            warn!("Skipping vehicle journey {} because I can't compute \
                   board time for its {}th stop_time. \n {:#?}",
                  vehicle_journey.id,
                  idx,
                  stop_time
            );
        })?;
        let debark_time = debark_time(stop_time).ok_or_else( || {
            warn!("Skipping vehicle journey {} because I can't compute \
                   debark time for its {}th stop_time. \n {:#?}",
                  vehicle_journey.id,
                  idx,
                  stop_time
            );
        })?;

        result.push((board_time, debark_time));
    }

    Ok(result)
}

fn board_debark_utc_times(board_debark_timezoned_times_in_day : &[(SecondsSinceTimezonedDayStart, SecondsSinceTimezonedDayStart)], 
    date : & NaiveDate,
    timezone : & chrono_tz::Tz,
    calendar : & Calendar,
    vehicle_journey : & VehicleJourney
) -> Result<  Vec<(SecondsSinceDatasetUTCStart, SecondsSinceDatasetUTCStart)> , ()>
{
    let day = calendar.date_to_days_since_start(date).ok_or_else(|| {
        warn!("Skipping vehicle journey {} on day {} because  \
                this day is not allowed by the calendar. \
                Allowed day are between {} and {}",
                vehicle_journey.id,
                date,
                calendar.first_date(),
                calendar.last_date(),
        );
    })?;
    let mut result = Vec::with_capacity(board_debark_timezoned_times_in_day.len());
    for (board_time, debark_time) in board_debark_timezoned_times_in_day.iter() {

        let board_time_utc = calendar.compose(&day, board_time, timezone);
        let debark_time_utc = calendar.compose(&day, debark_time, timezone);

        result.push((board_time_utc, debark_time_utc));
    }

    Ok(result)

    
}