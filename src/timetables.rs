mod daily;
mod generic_timetables;
mod iters;
mod periodic;

pub use daily::DailyTimetables;
pub use periodic::PeriodicTimetables;

use std::hash::Hash;

pub use crate::transit_data::{Idx, Stop, VehicleJourney};

use crate::time::{Calendar, SecondsSinceDatasetUTCStart, SecondsSinceTimezonedDayStart};

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

pub trait Timetables {
    type Mission: Debug + Clone + Hash + Eq;
    type Position: Debug + Clone;
    type Trip: Debug + Clone;

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

pub trait TimetablesIter<'a>: Timetables {
    type Positions: Iterator<Item = Self::Position>;
    fn positions(&'a self, mission: &Self::Mission) -> Self::Positions;

    type Trips: Iterator<Item = Self::Trip>;
    fn trips_of(&'a self, mission: &Self::Mission) -> Self::Trips;

    type Missions: Iterator<Item = Self::Mission>;
    fn missions(&'a self) -> Self::Missions;
}
