use crate::time::days_patterns::{DaysInPatternIter, DaysPattern, DaysPatterns};

use super::{
    generic_timetables::{Timetables, Vehicle},
    TimetablesIter,
};

use crate::time::{
    Calendar, DaysSinceDatasetStart, SecondsSinceDatasetUTCStart, SecondsSinceTimezonedDayStart,
};
use crate::transit_data::{Idx, VehicleJourney};
use chrono::NaiveDate;
use chrono_tz::Tz as TimeZone;

use crate::timetables::Timetables as TimetablesTrait;

use crate::log::warn;

pub struct PeriodicTimetables {
    timetables: Timetables<SecondsSinceTimezonedDayStart, TimeZone, VehicleData>,
    calendar: Calendar,
    days_patterns: DaysPatterns,
}

#[derive(Debug, Clone)]
struct VehicleData {
    days_pattern: DaysPattern,
    vehicle_journey_idx: Idx<VehicleJourney>,
}

#[derive(Debug, Clone)]
pub struct Trip {
    vehicle: Vehicle,
    day: DaysSinceDatasetStart,
}

impl TimetablesTrait for PeriodicTimetables {
    type Mission = super::generic_timetables::Timetable;

    type Position = super::generic_timetables::Position;

    type Trip = Trip;

    fn new(first_date: NaiveDate, last_date: NaiveDate) -> Self {
        let calendar = Calendar::new(first_date, last_date);
        let nb_of_days: usize = calendar.nb_of_days().into();
        Self {
            timetables: Timetables::new(),
            calendar,
            days_patterns: DaysPatterns::new(nb_of_days),
        }
    }

    fn calendar(&self) -> &Calendar {
        &self.calendar
    }

    fn nb_of_missions(&self) -> usize {
        self.timetables.nb_of_timetables()
    }

    fn mission_id(&self, mission : &Self::Mission) -> usize {
        mission.idx
    }

    fn vehicle_journey_idx(&self, trip: &Self::Trip) -> Idx<VehicleJourney> {
        self.timetables
            .vehicle_data(&trip.vehicle)
            .vehicle_journey_idx
    }

    fn stoptime_idx(&self, position: &Self::Position, _trip: &Self::Trip) -> usize {
        self.timetables.stoptime_idx(position)
    }

    fn day_of(&self, trip: &Self::Trip) -> NaiveDate {
        self.calendar().to_naive_date(&trip.day)
    }

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission {
        self.timetables.timetable_of(&trip.vehicle)
    }

    fn stop_at(&self, position: &Self::Position, mission: &Self::Mission) -> super::Stop {
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

    fn arrival_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> SecondsSinceDatasetUTCStart {
        let time_in_day = self.timetables.arrival_time(&trip.vehicle, position);
        let timetable = self.timetables.timetable_of(&trip.vehicle);
        let timezone = self.timetables.timezone_data(&timetable);
        let day = &trip.day;
        self.calendar.compose(day, time_in_day, timezone)
    }

    fn debark_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<SecondsSinceDatasetUTCStart> {
        let time_in_day = self.timetables.debark_time(&trip.vehicle, position);
        time_in_day.map(|time| {
            let timetable = self.timetables.timetable_of(&trip.vehicle);
            let timezone = self.timetables.timezone_data(&timetable);
            let day = &trip.day;
            self.calendar.compose(day, time, timezone)
        })
    }

    fn board_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<SecondsSinceDatasetUTCStart> {
        let time_in_day = self.timetables.board_time(&trip.vehicle, position);
        time_in_day.map(|time| {
            let timetable = self.timetables.timetable_of(&trip.vehicle);
            let timezone = self.timetables.timezone_data(&timetable);
            let day = &trip.day;
            self.calendar.compose(day, time, timezone)
        })
    }

    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart)> {
        let has_earliest_and_latest_board_time =
            self.timetables.earliest_and_latest_board_time(position);

        // if there is no earliest/latest board time, it means that this position cannot be boarded
        // and we return None
        let (earliest_board_time_in_day, latest_board_time_in_day) =
            has_earliest_and_latest_board_time?;

        let timezone = self.timetables.timezone_data(&mission);

        let decompositions = self.calendar.decompositions(
            waiting_time,
            timezone,
            *latest_board_time_in_day,
            *earliest_board_time_in_day,
        );
        let mut best_vehicle_day_and_its_arrival_time_at_next_position: Option<(
            Vehicle,
            DaysSinceDatasetStart,
            SecondsSinceDatasetUTCStart,
        )> = None;
        for (waiting_day, waiting_time_in_day) in decompositions {
            let has_vehicle = self.timetables.earliest_filtered_vehicle_to_board(
                &waiting_time_in_day,
                &mission,
                &position,
                |vehicle_data| {
                    let days_pattern = vehicle_data.days_pattern;
                    self.days_patterns.is_allowed(&days_pattern, &waiting_day)
                },
            );
            if let Some((vehicle, arrival_time_in_day_at_next_stop)) = has_vehicle {
                let arrival_time_at_next_stop = self.calendar.compose(
                    &waiting_day,
                    arrival_time_in_day_at_next_stop,
                    &timezone,
                );
                if let Some((_, _, best_arrival_time)) =
                    &best_vehicle_day_and_its_arrival_time_at_next_position
                {
                    if arrival_time_at_next_stop < *best_arrival_time {
                        best_vehicle_day_and_its_arrival_time_at_next_position =
                            Some((vehicle, waiting_day, arrival_time_at_next_stop));
                    }
                } else {
                    best_vehicle_day_and_its_arrival_time_at_next_position =
                        Some((vehicle, waiting_day, arrival_time_at_next_stop));
                }
            }
        }

        best_vehicle_day_and_its_arrival_time_at_next_position.map(
            |(vehicle, day, arrival_time_at_next_stop)| {
                let trip = Trip { vehicle, day };
                (trip, arrival_time_at_next_stop)
            },
        )
    }

    fn insert<'date, NaiveDates>(
        &mut self,
        stop_flows: super::StopFlows,
        board_debark_timezoned_times: &[(
            SecondsSinceTimezonedDayStart,
            SecondsSinceTimezonedDayStart,
        )],
        valid_dates: NaiveDates,
        timezone: &chrono_tz::Tz,
        vehicle_journey_idx: Idx<VehicleJourney>,
        vehicle_journey: &VehicleJourney,
    ) -> Vec<Self::Mission>
    where
        NaiveDates: Iterator<Item = &'date NaiveDate>,
    {
        let days_pattern = self
            .days_patterns
            .get_or_insert(valid_dates, &self.calendar);
        let vehicle_data = VehicleData {
            days_pattern,
            vehicle_journey_idx,
        };
        let insert_error = self.timetables.insert(
            stop_flows,
            board_debark_timezoned_times
                .iter()
                .copied(),
            *timezone,
            vehicle_data,
        );
        let mut result = Vec::new();
        match insert_error {
            Ok(mission) => {
                result.push(mission);
            }
            Err(error) => {
                handle_vehicletimes_error(vehicle_journey, &error, board_debark_timezoned_times);
            }
        }
        result
    }
}

use super::generic_timetables::VehicleTimesError;

fn handle_vehicletimes_error(
    vehicle_journey: &VehicleJourney,
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
                "Skipping vehicle journey {}  because its \
                    debark time {} at sequence {}\
                    is earlier than its \
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
            let downstream_stop_time = &vehicle_journey.stop_times[position_pair.downstream];
            let upstream_board = &board_debark_times[position_pair.upstream].0;
            let downstream_board = &board_debark_times[position_pair.downstream].0;
            warn!(
                "Skipping vehicle journey {}  because its \
                    board time {} at sequence {} \
                    is earlier than its \
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
            let downstream_stop_time = &vehicle_journey.stop_times[position_pair.downstream];
            let upstream_debark = &board_debark_times[position_pair.upstream].1;
            let downstream_debark = &board_debark_times[position_pair.downstream].1;
            warn!(
                "Skipping vehicle journey {}  because its \
                    debark time {} at sequence {} \
                    is earlier than its \
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

impl<'a> TimetablesIter<'a> for PeriodicTimetables {
    type Positions = super::iters::PositionsIter;

    fn positions(&'a self, mission: &Self::Mission) -> Self::Positions {
        self.timetables.positions(mission)
    }

    type Trips = TripsIter<'a>;

    fn trips_of(&'a self, mission: &Self::Mission) -> Self::Trips {
        TripsIter::new(&self, mission)
    }

    type Missions = super::iters::TimetableIter;

    fn missions(&'a self) -> Self::Missions {
        self.timetables.timetables()
    }
}

pub struct TripsIter<'a> {
    periodic: &'a PeriodicTimetables,
    current_vehicle_days: Option<(Vehicle, DaysInPatternIter<'a>)>,
    vehicles_iter: super::iters::VehicleIter,
}

impl<'a> TripsIter<'a> {
    fn new(
        periodic: &'a PeriodicTimetables,
        timetable: &super::generic_timetables::Timetable,
    ) -> Self {
        let mut vehicles_iter = periodic.timetables.vehicles(&timetable);
        let has_current_vehicle = vehicles_iter.next();
        let current_vehicle_days = has_current_vehicle.map(|vehicle| {
            let days_pattern = periodic.timetables.vehicle_data(&vehicle).days_pattern;
            let days_iter = periodic.days_patterns.days_in_pattern(&days_pattern);
            (vehicle, days_iter)
        });

        Self {
            periodic,
            current_vehicle_days,
            vehicles_iter,
        }
    }
}

impl<'a> Iterator for TripsIter<'a> {
    type Item = Trip;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((vehicle, days_iter)) = &mut self.current_vehicle_days {
                match days_iter.next() {
                    Some(day) => {
                        let trip = Trip {
                            vehicle: vehicle.clone(),
                            day,
                        };
                        return Some(trip);
                    }
                    None => {
                        let has_current_vehicle = self.vehicles_iter.next();
                        self.current_vehicle_days= has_current_vehicle.map(|vehicle| {
                            let days_pattern =
                                self.periodic.timetables.vehicle_data(&vehicle).days_pattern;
                            let days_iter =
                                self.periodic.days_patterns.days_in_pattern(&days_pattern);
                            (vehicle, days_iter)
                        });
                    }
                }
            } else {
                return None;
            }
        }
    }
}
