mod daily;
mod generic_timetables;
mod iters;
mod periodic;
mod with_loads;

pub use daily::DailyTimetables;
pub use periodic::PeriodicTimetables;

use std::hash::Hash;

pub use crate::transit_data::{Idx, Stop, VehicleJourney};

use crate::{loads_data::Load, time::{Calendar, SecondsSinceDatasetUTCStart, SecondsSinceTimezonedDayStart}};

use chrono::NaiveDate;
use chrono_tz::Tz as TimeZone;

use std::fmt::Debug;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Ord, PartialOrd)]
pub enum FlowDirection {
    BoardOnly,
    DebarkOnly,
    BoardAndDebark,
}
pub type StopFlows = Vec<(Stop, FlowDirection)>;


pub trait Types {
    type Mission: Debug + Clone + Hash + Eq;
    type Position: Debug + Clone;
    type Trip: Debug + Clone;
}

pub trait Timetables : Types {


    fn new(first_date: NaiveDate, last_date: NaiveDate) -> Self;

    fn calendar(&self) -> &Calendar;

    fn nb_of_missions(&self) -> usize;
    fn mission_id(&self, mission: &Self::Mission) -> usize;

    fn vehicle_journey_idx(&self, trip: &Self::Trip) -> Idx<VehicleJourney>;
    fn stoptime_idx(&self, position: &Self::Position, trip: &Self::Trip) -> usize;
    fn day_of(&self, trip: &Self::Trip) -> NaiveDate;

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission;
    fn stop_at(&self, position: &Self::Position, mission: &Self::Mission) -> Stop;

    fn is_upstream_in_mission(
        &self,
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool;

    fn next_position(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position>;

    fn arrival_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> SecondsSinceDatasetUTCStart;
    fn debark_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<SecondsSinceDatasetUTCStart>;
    fn board_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<SecondsSinceDatasetUTCStart>;
    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart)>;

    fn insert<'date, NaiveDates>(
        &mut self,
        stop_flows: StopFlows,
        board_debark_timezoned_times: &[(
            SecondsSinceTimezonedDayStart,
            SecondsSinceTimezonedDayStart,
        )],
        valid_dates: NaiveDates,
        timezone: &TimeZone,
        vehicle_journey_idx: Idx<VehicleJourney>,
        vehicle_journey: &VehicleJourney,
    ) -> Vec<Self::Mission>
    where
        NaiveDates: Iterator<Item = &'date NaiveDate>;
}

pub trait TimetablesIter<'a>: Types {
    type Positions: Iterator<Item = Self::Position>;
    fn positions(&'a self, mission: &Self::Mission) -> Self::Positions;

    type Trips: Iterator<Item = Self::Trip>;
    fn trips_of(&'a self, mission: &Self::Mission) -> Self::Trips;

    type Missions: Iterator<Item = Self::Mission>;
    fn missions(&'a self) -> Self::Missions;
}



#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeLoad {
    time : SecondsSinceDatasetUTCStart,
    load : Load,
}

impl TimeLoad {
    fn new(time : SecondsSinceDatasetUTCStart, load : Load) -> Self {
        Self{
            time,
            load
        }
    }
}

use std::cmp::Ordering;

impl Ord for TimeLoad {
    fn cmp(&self, other: &Self) -> Ordering {
        use Ordering::{Less, Equal, Greater};
        match Ord::cmp(&self.time, &other.time) {
            Less => Less, 
            Greater => Greater,
            Equal => self.load.cmp(&other.load)
        }
    }
}

impl PartialOrd for TimeLoad {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}


pub trait TimeLoadtables : Types {

    fn new(first_date: NaiveDate, last_date: NaiveDate) -> Self;

    fn calendar(&self) -> &Calendar;

    fn nb_of_missions(&self) -> usize;
    fn mission_id(&self, mission: &Self::Mission) -> usize;

    fn vehicle_journey_idx(&self, trip: &Self::Trip) -> Idx<VehicleJourney>;
    fn stoptime_idx(&self, position: &Self::Position, trip: &Self::Trip) -> usize;
    fn day_of(&self, trip: &Self::Trip) -> NaiveDate;

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission;
    fn stop_at(&self, position: &Self::Position, mission: &Self::Mission) -> Stop;

    fn is_upstream_in_mission(
        &self,
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool;

    fn next_position(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position>;

    fn arrival_timeload_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> TimeLoad;
    fn debark_timeload_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<TimeLoad>;
    fn board_timeload_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<TimeLoad>;
    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, TimeLoad)>;

    fn insert<'date, NaiveDates>(
        &mut self,
        stop_flows: StopFlows,
        board_debark_timezoned_times: &[(
            SecondsSinceTimezonedDayStart,
            SecondsSinceTimezonedDayStart,
        )],
        loads : & [Load],
        valid_dates: NaiveDates,
        timezone: &TimeZone,
        vehicle_journey_idx: Idx<VehicleJourney>,
        vehicle_journey: &VehicleJourney,
    ) -> Vec<Self::Mission>
    where
        NaiveDates: Iterator<Item = &'date NaiveDate>;
}

