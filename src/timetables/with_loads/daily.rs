use std::{cmp::max, iter::Map};

use super::super::{TimetablesIter, Stop, StopFlows, 
    generic_timetables::{Timetables, Timetable, Position, Vehicle, VehicleTimesError}, 
    iters::{PositionsIter, VehicleIter, TimetableIter}
};

use crate::time::{
    Calendar, DaysSinceDatasetStart, SecondsSinceDatasetUTCStart, SecondsSinceTimezonedDayStart,
};
use crate::transit_data::{Idx, VehicleJourney};
use chrono::NaiveDate;

use crate::timetables::{TimeLoad, TimeLoadtables, Types as TimetablesTypes};

use crate::log::warn;

use crate::loads_data::{Load, LoadsData};

pub struct DailyTimetables {
    timetables: Timetables<TimeLoad, (), VehicleData>,
    calendar: Calendar,
}
#[derive(Clone)]
struct VehicleData {
    vehicle_journey_idx: Idx<VehicleJourney>,
    day: DaysSinceDatasetStart,
}

impl TimetablesTypes for DailyTimetables {
    type Mission = Timetable;

    type Position = Position;

    type Trip = Vehicle;

}

impl TimeLoadtables for DailyTimetables {
    fn new(first_date: NaiveDate, last_date: NaiveDate) -> Self {
        Self {
            timetables: Timetables::new(),
            calendar: Calendar::new(first_date, last_date),
        }
    }

    fn calendar(&self) -> &Calendar {
        &self.calendar
    }

    fn nb_of_missions(&self) -> usize {
        self.timetables.nb_of_timetables()
    }

    fn mission_id(&self, mission: &Self::Mission) -> usize {
        mission.idx
    }

    fn vehicle_journey_idx(&self, trip: &Self::Trip) -> Idx<VehicleJourney> {
        self.timetables.vehicle_data(trip).vehicle_journey_idx
    }

    fn day_of(&self, trip: &Self::Trip) -> NaiveDate {
        let day = self.timetables.vehicle_data(trip).day;
        self.calendar().to_naive_date(&day)
    }

    fn stoptime_idx(&self, position: &Self::Position, _trip: &Self::Trip) -> usize {
        self.timetables.stoptime_idx(position)
    }

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission {
        self.timetables.timetable_of(trip)
    }

    fn stop_at(&self, position: &Self::Position, mission: &Self::Mission) -> Stop {
        *self.timetables.stop_at(position, mission)
    }

    fn is_upstream_in_mission(
        &self,
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool {
        self.timetables.is_upstream(upstream, downstream, mission)
    }

    fn next_position(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.timetables.next_position(position, mission)
    }

    fn arrival_timeload_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> TimeLoad {
        self.timetables.arrival_time(trip, position).clone()
    }

    fn debark_timeload_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<TimeLoad> {
        self.timetables.debark_time(trip, position).cloned()
    }

    fn board_timeload_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<TimeLoad> {
        self.timetables.board_time(trip, position).cloned()
    }

    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, TimeLoad)> {
        let waiting_timeload = TimeLoad::new(*waiting_time, Load::Low);
           
        self.timetables
            .earliest_vehicle_to_board(&waiting_timeload, mission, position)
            .map(|(trip, timeload)| (trip, timeload.clone()))
    }

    fn insert<'date, NaiveDates>(
        &mut self,
        stop_flows: StopFlows,
        board_debark_timezoned_times: &[(
            SecondsSinceTimezonedDayStart,
            SecondsSinceTimezonedDayStart,
        )],
        loads : & [Load],
        valid_dates: NaiveDates,
        timezone: &chrono_tz::Tz,
        vehicle_journey_idx: Idx<VehicleJourney>,
        vehicle_journey: &VehicleJourney,
    ) -> Vec<Self::Mission>
    where
        NaiveDates: Iterator<Item = &'date chrono::NaiveDate>,
    {
        let mut result = Vec::new();
        assert!(board_debark_timezoned_times.len() - 1 == loads.len() );

        for date in valid_dates {
            let has_day = self.calendar.date_to_days_since_start(date);
            match has_day {
                None => {
                    warn!(
                        "Skipping vehicle journey {} on day {} because  \
                        this day is not allowed by the calendar. \
                        Allowed day are between {} and {}",
                        vehicle_journey.id,
                        date,
                        self.calendar.first_date(),
                        self.calendar.last_date(),
                    );
                    continue;
                }
                Some(day) => {
                    let calendar = &self.calendar;
                    let board_debark_utc_times =
                        board_debark_timezoned_times
                            .iter()
                            .map(|(board_time, debark_time)| {
                                let board_time_utc = calendar.compose(&day, board_time, timezone);
                                let debark_time_utc = calendar.compose(&day, debark_time, timezone);
                                (board_time_utc, debark_time_utc)
                            });
                    let board_debark_timeloads = board_debark_utc_times.enumerate()
                            .map(|(idx, (board_time, debark_time))| {
                                let load_after_board = &loads[idx];
                                let board_timeload = TimeLoad::new(board_time, *load_after_board);

                                let load_before_debark = &loads[max(idx - 1, 0)];
                                let debark_timeload = TimeLoad::new(debark_time, *load_before_debark);
                                (board_timeload, debark_timeload)
                            });
                    let vehicle_data = VehicleData {
                        vehicle_journey_idx,
                        day,
                    };
                    let insert_error = self.timetables.insert(
                        stop_flows.clone(),
                        board_debark_timeloads,
                        (),
                        vehicle_data,
                    );
                    match insert_error {
                        Ok(mission) => {
                            result.push(mission);
                        }
                        Err(error) => {
                            handle_vehicletimes_error(
                                vehicle_journey,
                                date,
                                &error,
                                board_debark_timezoned_times,
                            );
                        }
                    }
                }
            }
        }
        result
    }
}

impl<'a> TimetablesIter<'a> for DailyTimetables {
    type Positions = PositionsIter;

    fn positions(&'a self, mission: &Self::Mission) -> Self::Positions {
        self.timetables.positions(mission)
    }

    type Trips = VehicleIter;

    fn trips_of(&'a self, mission: &Self::Mission) -> Self::Trips {
        self.timetables.vehicles(mission)
    }

    type Missions = TimetableIter;

    fn missions(&'a self) -> Self::Missions {
        self.timetables.timetables()
    }
}



fn handle_vehicletimes_error(
    vehicle_journey: &VehicleJourney,
    date: &NaiveDate,
    error: &VehicleTimesError,
    board_debark_times: &[(SecondsSinceTimezonedDayStart, SecondsSinceTimezonedDayStart)],
) {
    match error {
        VehicleTimesError::DebarkBeforeUpstreamBoard(position_pair) => {
            let upstream_stop_time = &vehicle_journey.stop_times[position_pair.upstream];
            let downstream_stop_time = &vehicle_journey.stop_times[position_pair.downstream];
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
            let downstream_stop_time = &vehicle_journey.stop_times[position_pair.downstream];
            let upstream_board = &board_debark_times[position_pair.upstream].0;
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
            let downstream_stop_time = &vehicle_journey.stop_times[position_pair.downstream];
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
