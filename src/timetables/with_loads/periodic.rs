use std::{cmp::max, collections::BTreeMap};

use super::super::{TimetablesIter, Stop, StopFlows, 
    generic_timetables::{Timetables, Timetable, Position, Vehicle, VehicleTimesError}, 
    iters::{PositionsIter, VehicleIter, TimetableIter}
};

use crate::{loads_data::LoadsData, time::{Calendar, DaysSinceDatasetStart, SecondsSinceDatasetUTCStart, SecondsSinceTimezonedDayStart, days_patterns::{DaysInPatternIter, DaysPattern, DaysPatterns}}};
use crate::transit_data::{Idx, VehicleJourney};
use chrono::NaiveDate;
use chrono_tz::Tz as TimeZone;
use crate::timetables::{TimeLoad, TimeLoadtables, Types as TimetablesTypes};

use crate::log::warn;

use crate::loads_data::{Load};



#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalTimeLoad {
    time : SecondsSinceTimezonedDayStart,
    load : Load,
}

impl LocalTimeLoad {
    fn new(time : SecondsSinceTimezonedDayStart, load : Load) -> Self {
        Self{
            time,
            load
        }
    }
}

use std::cmp::Ordering;

impl Ord for LocalTimeLoad {
    fn cmp(&self, other: &Self) -> Ordering {
        use Ordering::{Less, Equal, Greater};
        match Ord::cmp(&self.time, &other.time) {
            Less => Less, 
            Greater => Greater,
            Equal => self.load.cmp(&other.load)
        }
    }
}

impl PartialOrd for LocalTimeLoad {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}


pub struct PeriodicTimetables {
    timetables: Timetables<LocalTimeLoad, TimeZone, VehicleData>,
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

impl TimetablesTypes for PeriodicTimetables {
    type Mission = Timetable;

    type Position = Position;

    type Trip = Trip;

}

impl TimeLoadtables for PeriodicTimetables {
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

    fn mission_id(&self, mission: &Self::Mission) -> usize {
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
        let localtime_load = self.timetables.arrival_time(&trip.vehicle, position);
        let timetable = self.timetables.timetable_of(&trip.vehicle);
        let timezone = self.timetables.timezone_data(&timetable);
        let day = &trip.day;
        let time = self.calendar.compose(day, &localtime_load.time, timezone);
        TimeLoad::new(time, localtime_load.load)
    }

    fn debark_timeload_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<TimeLoad> {
        let has_localtime_load = self.timetables.debark_time(&trip.vehicle, position);
        has_localtime_load.map(|localtime_load| {
            let timetable = self.timetables.timetable_of(&trip.vehicle);
            let timezone = self.timetables.timezone_data(&timetable);
            let day = &trip.day;
            let time = self.calendar.compose(day, &localtime_load.time, timezone);
            TimeLoad::new(time, localtime_load.load)
        })
    }

    fn board_timeload_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<TimeLoad> {
        let has_localtime_load = self.timetables.board_time(&trip.vehicle, position);
        has_localtime_load.map(|localtime_load| {
            let timetable = self.timetables.timetable_of(&trip.vehicle);
            let timezone = self.timetables.timezone_data(&timetable);
            let day = &trip.day;
            let time = self.calendar.compose(day, &localtime_load.time, timezone);
            TimeLoad::new(time, localtime_load.load)
        })
    }

    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, TimeLoad)> {
        let has_earliest_and_latest_board_time =
            self.timetables.earliest_and_latest_board_time(position);

        // if there is no earliest/latest board time, it means that this position cannot be boarded
        // and we return None
        let (earliest_board_time_in_day, latest_board_time_in_day) =
            has_earliest_and_latest_board_time
            .map(|(earliest_timeload, latest_timeload)| (earliest_timeload.time, latest_timeload.time))?;

        let timezone = self.timetables.timezone_data(&mission);

        let decompositions = self.calendar.decompositions(
            waiting_time,
            timezone,
            latest_board_time_in_day,
            earliest_board_time_in_day,
        );
        let mut best_vehicle_day_and_its_arrival_timeload_at_next_position: Option<(
            Vehicle,
            DaysSinceDatasetStart,
            TimeLoad,
        )> = None;
        for (waiting_day, waiting_time_in_day) in decompositions {
            let waiting_localtimeload = LocalTimeLoad::new(waiting_time_in_day, Load::Low);
            let has_vehicle = self.timetables.earliest_filtered_vehicle_to_board(
                &waiting_localtimeload,
                &mission,
                &position,
                |vehicle_data| {
                    let days_pattern = vehicle_data.days_pattern;
                    self.days_patterns.is_allowed(&days_pattern, &waiting_day)
                },
            );
            if let Some((vehicle, arrival_localtimeload_at_next_stop)) = has_vehicle {
                let arrival_time_at_next_stop = self.calendar.compose(
                    &waiting_day,
                    &arrival_localtimeload_at_next_stop.time,
                    &timezone,
                );
                let arrival_timeload_at_next_stop = TimeLoad::new(arrival_time_at_next_stop, arrival_localtimeload_at_next_stop.load);
                if let Some((_, _, best_arrival_timeload)) =
                    &best_vehicle_day_and_its_arrival_timeload_at_next_position
                {
                    if arrival_timeload_at_next_stop < *best_arrival_timeload {
                        best_vehicle_day_and_its_arrival_timeload_at_next_position =
                            Some((vehicle, waiting_day, arrival_timeload_at_next_stop));
                    }
                } else {
                    best_vehicle_day_and_its_arrival_timeload_at_next_position =
                        Some((vehicle, waiting_day, arrival_timeload_at_next_stop));
                }
            }
        }

        best_vehicle_day_and_its_arrival_timeload_at_next_position.map(
            |(vehicle, day, arrival_timeload_at_next_stop)| {
                let trip = Trip { vehicle, day };
                (trip, arrival_timeload_at_next_stop)
            },
        )
    }

    fn insert<'date, NaiveDates>(
        &mut self,
        stop_flows: StopFlows,
        board_debark_timezoned_times: &[(
            SecondsSinceTimezonedDayStart,
            SecondsSinceTimezonedDayStart,
        )],
        loads_data : & LoadsData,
        valid_dates: NaiveDates,
        timezone: &chrono_tz::Tz,
        vehicle_journey_idx: Idx<VehicleJourney>,
        vehicle_journey: &VehicleJourney,
    ) -> Vec<Self::Mission>
    where
        NaiveDates: Iterator<Item = &'date NaiveDate>,
    {

        let mut load_patterns_dates : BTreeMap<& [Load], Vec<NaiveDate>>= BTreeMap::new();
        let mut default_load_dates : Vec<NaiveDate> = Vec::new();

        for date in valid_dates {
            let has_loads = loads_data.loads(&vehicle_journey_idx, date);
            assert!(if let Some(loads) = has_loads {
                board_debark_timezoned_times.len()  == loads.len() 
                } 
                else {
                    true
                }                      
            );

            if let Some(loads) = has_loads {
                load_patterns_dates.entry(loads).or_insert_with(Vec::new)
                    .push(*date);
            }
            else {
                default_load_dates.push(*date)
            }

        }

        let mut result = Vec::new();

        for (loads, dates) in load_patterns_dates.into_iter() {

            let board_debark_localtimeloads = board_debark_timezoned_times.iter().enumerate()
            .map(|(idx, (board_time, debark_time))| {
                let load_after_board = loads[idx].clone();
                let board_timeload = LocalTimeLoad::new(*board_time, load_after_board);
    
                let load_before_debark = loads[max(idx - 1, 0)].clone();
                let debark_timeload = LocalTimeLoad::new(*debark_time, load_before_debark);
                (board_timeload, debark_timeload)
            });

            let insert_error = self._insert(
                stop_flows.clone(), 
                board_debark_localtimeloads, 
                &dates,
                 timezone, 
                 vehicle_journey_idx
                );
            match insert_error {
                Ok(mission) => {
                    result.push(mission);
                }
                Err(error) => {
                    handle_vehicletimes_error(vehicle_journey, &error, board_debark_timezoned_times);
                }
            }
        }

        {
            let board_debark_localtimeloads = board_debark_timezoned_times.iter()
                .map(|(board_time, debark_time)| {
                    let board_timeload = LocalTimeLoad::new(*board_time, Load::Medium);
                    let debark_timeload = LocalTimeLoad::new(*debark_time, Load::Medium);
                    (board_timeload, debark_timeload)
                } );
            let insert_error = self._insert(
                stop_flows.clone(), 
                board_debark_localtimeloads, 
                &default_load_dates,
                timezone, 
                vehicle_journey_idx
            );
            match insert_error {
                Ok(mission) => {
                    result.push(mission);
                }
                Err(error) => {
                    handle_vehicletimes_error(vehicle_journey, &error, board_debark_timezoned_times);
                }
            }

        }
        
        result
    }
}

impl PeriodicTimetables {
    fn _insert<BoardDebarkLocalTimeLoads>(& mut self, 
        stop_flows : StopFlows,
        board_debark_localtimeloads : BoardDebarkLocalTimeLoads,
        dates : & [NaiveDate], 
        timezone: &chrono_tz::Tz,
        vehicle_journey_idx: Idx<VehicleJourney>,
    ) -> Result<Timetable, VehicleTimesError>
    where BoardDebarkLocalTimeLoads : Iterator<Item = (LocalTimeLoad, LocalTimeLoad)> + ExactSizeIterator + Clone
    
    {
        let days_pattern = self
                .days_patterns
                .get_or_insert(dates.iter(), &self.calendar);
        let vehicle_data = VehicleData {
            days_pattern,
            vehicle_journey_idx,
        };
        self.timetables.insert(
            stop_flows.clone(),
            board_debark_localtimeloads,
            *timezone,
            vehicle_data,
        )
        
    }
}


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
    type Positions = PositionsIter;

    fn positions(&'a self, mission: &Self::Mission) -> Self::Positions {
        self.timetables.positions(mission)
    }

    type Trips = TripsIter<'a>;

    fn trips_of(&'a self, mission: &Self::Mission) -> Self::Trips {
        TripsIter::new(&self, mission)
    }

    type Missions = TimetableIter;

    fn missions(&'a self) -> Self::Missions {
        self.timetables.timetables()
    }
}

pub struct TripsIter<'a> {
    periodic: &'a PeriodicTimetables,
    current_vehicle_days: Option<(Vehicle, DaysInPatternIter<'a>)>,
    vehicles_iter: VehicleIter,
}

impl<'a> TripsIter<'a> {
    fn new(
        periodic: &'a PeriodicTimetables,
        timetable: &Timetable,
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
                        self.current_vehicle_days = has_current_vehicle.map(|vehicle| {
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
