

use super::generic_timetables::Timetables;

use chrono::NaiveDate;
use crate::time::{Calendar, SecondsSinceDatasetUTCStart, SecondsSinceTimezonedDayStart};
use crate::transit_data::{Idx, VehicleJourney};

use crate::timetables::Timetables as TimetablesTrait;

use crate::log::warn;

struct DailyTimetables {
    timetables : Timetables<SecondsSinceDatasetUTCStart, (), Idx<VehicleJourney>>,
    calendar : Calendar,
}

impl TimetablesTrait for DailyTimetables {
    type Mission = super::generic_timetables::Timetable;

    type Position = super::generic_timetables::Position;

    type Trip = super::generic_timetables::Vehicle;

    fn new(first_date: NaiveDate, last_date: NaiveDate) -> Self {
        Self {
            timetables : Timetables::new(),
            calendar : Calendar::new(first_date, last_date)
        }
    }

    fn nb_of_missions(&self) -> usize {
        self.timetables.nb_of_timetables()
    }

    fn vehicle_journey_idx(&self, trip : & Self::Trip) -> Idx<VehicleJourney> {
        self.timetables.vehicle_data(trip).clone()
    }

    fn stoptime_idx(&self, position : &Self::Position, trip : & Self::Trip) -> usize {
        self.timetables.stoptime_idx(position)
    }

    fn mission_of(&self, trip : & Self::Trip) -> Self::Mission {
        self.timetables.timetable_of(trip)
    }

    fn stop_at(&self, position : & Self::Position, mission : & Self::Mission) -> super::Stop {
        self.timetables.stop_at(position, mission).clone()
    }

    fn is_upstream_in_mission(
        &self,
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool {
        self.timetables.is_upstream(upstream, downstream, mission)
    }

    fn next_position_in_mission(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.timetables.next_position(position, mission)
    }

    fn arrival_time_of(&self, trip : & Self::Trip, position : & Self::Position) -> SecondsSinceDatasetUTCStart {
        self.timetables.arrival_time(trip, position).clone()
    }

    fn debark_time_of(&self, trip : & Self::Trip, position : & Self::Position) -> Option<SecondsSinceDatasetUTCStart>  {
        self.timetables.debark_time(trip, position).cloned()
    }

    fn board_time_of(&self, trip : & Self::Trip, position : & Self::Position) -> Option<SecondsSinceDatasetUTCStart>  {
        self.timetables.board_time(trip, position).cloned()
    }

    fn earliest_trip_to_board_at(&self, waiting_time : & SecondsSinceDatasetUTCStart, mission : &Self::Mission, position : & Self::Position) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart)> {
        self.timetables.earliest_vehicle_to_board(waiting_time, mission, position)
            .map(|(trip, time)| (trip, time.clone()))
    }

    fn insert<'date, NaiveDates>(
        &mut self, 
        stop_flows : super::StopFlows, 
        board_debark_timezoned_times : & [(SecondsSinceTimezonedDayStart, SecondsSinceTimezonedDayStart)], 
        valid_dates : NaiveDates,
        timezone : & chrono_tz::Tz, 
        vehicle_journey_idx : Idx<VehicleJourney>,
        vehicle_journey : &VehicleJourney
    ) -> Vec<Self::Mission>
    where
        NaiveDates : Iterator<Item = & 'date chrono::NaiveDate>
    {
        let mut result = Vec::new();
        
        for date in valid_dates {
            let has_day = self.calendar.date_to_days_since_start(date);
            match has_day {
                None =>  {

                    warn!("Skipping vehicle journey {} on day {} because  \
                        this day is not allowed by the calendar. \
                        Allowed day are between {} and {}",
                        vehicle_journey.id,
                        date,
                        self.calendar.first_date(),
                        self.calendar.last_date(),
                    );
                    continue;
                },
                Some(day) => {
                    let calendar = &self.calendar;
                    let board_debark_utc_times = board_debark_timezoned_times.iter()
                    .map(|(board_time, debark_time)| {
                        let board_time_utc = calendar.compose(&day, board_time, timezone).clone() ;
                        let debark_time_utc =  calendar.compose(&day, debark_time, timezone).clone() ;
                        (board_time_utc, debark_time_utc)
                    });
                    let insert_error = self.timetables.insert(
                        stop_flows.clone(), 
                        board_debark_utc_times, 
                        (),
                        vehicle_journey_idx,
                    );
                    match insert_error {
                        Ok(mission) => {
                            result.push(mission);
                        },
                        Err(error) =>  {
                            handle_vehicletimes_error(vehicle_journey, date, &error, board_debark_timezoned_times);
                        }
                    }
                }
            }



            
             
        }
        result
    }
}

use super::generic_timetables::{VehicleTimesError};

fn handle_vehicletimes_error(vehicle_journey : & VehicleJourney, 
    date : & NaiveDate, 
    error : & VehicleTimesError, 
    board_debark_times : & [(SecondsSinceTimezonedDayStart, SecondsSinceTimezonedDayStart)],
)
 {
    match error {
        VehicleTimesError::DebarkBeforeUpstreamBoard(position_pair) => {
            let upstream_stop_time = &vehicle_journey.stop_times[position_pair.upstream];
            let downstream_stop_time =
                &vehicle_journey.stop_times[position_pair.downstream];
            let board = &board_debark_times[position_pair.upstream].0;
            let debark = &board_debark_times[position_pair.downstream].1;
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
            let upstream_board =  &board_debark_times[position_pair.upstream].0;
            let downstream_board = &board_debark_times[position_pair.downstream].0;
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
            let upstream_debark = &board_debark_times[position_pair.upstream].1;
            let downstream_debark = &board_debark_times[position_pair.downstream].1;
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
