use super::transit_data::{Mission, Stop, TransitData, Trip};

use super::time::{DaysSinceDatasetStart, SecondsSinceDatasetUTCStart, SecondsSinceTimezonedDayStart};

use super::timetables::timetables_data::{Position, Timetable, Vehicle};

impl TransitData {
    pub fn is_upstream_in_mission(
        &self,
        upstream: &Position,
        downstream: &Position,
        mission: &Mission,
    ) -> bool {
        assert!(upstream.timetable == downstream.timetable);
        upstream.is_upstream(downstream).unwrap()
    }

    pub fn next_position_in_mission(
        &self,
        position: &Position,
        mission: &Mission,
    ) -> Option<Position> {
        assert!(position.timetable == mission.timetable);
        self.timetables.next_position(position)
    }

    pub fn next_position(&self, position: &Position) -> Option<Position> {
        self.timetables.next_position(position)
    }

    pub fn stop_at_position_in_mission(&self, position: &Position, mission: &Mission) -> Stop {
        assert!(position.timetable == mission.timetable);
        self.timetables.stop_at(&mission.timetable, position).clone()
    }

    pub fn stop_at_position_in_trip(&self, position: &Position, trip: &Trip) -> Stop {
        assert!(position.timetable == trip.vehicle.timetable);
        self.timetables.stop_at(&trip.vehicle.timetable, position).clone()
    }

    pub fn mission_of(&self, trip: &Trip) -> Mission {
        Mission{ timetable : trip.vehicle.timetable.clone() }
    }

    pub fn stoptime_idx(&self, position: &Position, _trip: &Trip) -> usize {
        position.idx_in_timetable()
    }

    // Panics if `trip` does not go through `position`
    pub fn arrival_time_of(&self, trip: &Trip, position: &Position) -> SecondsSinceDatasetUTCStart {
        let vehicle = &trip.vehicle;
        let seconds_in_day = self.timetables.arrival_time_at(vehicle, position);
        SecondsSinceDatasetUTCStart::compose(&trip.day, seconds_in_day)
    }

    // Panics if `position` is not valid for `trip`
    // None if `trip` does not allows debark at `stop_idx`
    pub fn debark_time_of(
        &self,
        trip: &Trip,
        position: &Position,
    ) -> Option<SecondsSinceDatasetUTCStart> {
        let vehicle = &trip.vehicle;
        let has_seconds_in_day = self.timetables.debark_time_at(vehicle, position);
        has_seconds_in_day.as_ref().map(|seconds_in_day| {
            let days = &trip.day;
            SecondsSinceDatasetUTCStart::compose(days, &seconds_in_day)
        })
    }

    // Panics if `position` is not valid for `trip`
    // None if `trip` does not allows boarding at `stop_idx`
    pub fn board_time_of(
        &self,
        trip: &Trip,
        position: &Position,
    ) -> Option<SecondsSinceDatasetUTCStart> {
        let vehicle = &trip.vehicle;
        let has_seconds_in_day = self.timetables.board_time_at(vehicle, position);
        has_seconds_in_day.as_ref().map(|seconds_in_day| {
            let days = &trip.day;
            SecondsSinceDatasetUTCStart::compose(days, &seconds_in_day)
        })
    }

    pub fn earliest_trip_to_board_at(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Mission,
        position: &Position,
    ) -> Option<(Trip, SecondsSinceDatasetUTCStart)> {
        let timetable = &mission.timetable;
        self.timetables.best_vehicle_to_board(waiting_time, timetable, position);

    }

    


}
