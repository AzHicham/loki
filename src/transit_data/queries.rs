use crate::transit_data::{Stop, TransitData, };

use crate::time::{SecondsSinceDatasetUTCStart, };

use crate::timetables::{Timetables as TimetablesTrait  };

impl<Timetables : TimetablesTrait> TransitData<Timetables> {
    pub fn is_upstream_in_mission(
        &self,
        upstream: &Timetables::Position,
        downstream: &Timetables::Position,
        mission: &Timetables::Mission,
    ) -> bool {
        assert!(upstream.timetable == downstream.timetable);
        assert!(upstream.timetable == mission.timetable);
        upstream.is_upstream(downstream).unwrap()
    }

    pub fn next_position_in_mission(
        &self,
        position: &Timetables::Position,
        mission: &Timetables::Mission,
    ) -> Option<Timetables::Position> {
        assert!(position.timetable == mission.timetable);
        self.timetables.next_position(position)
    }

    pub fn next_position(&self, position: &Timetables::Position) -> Option<Timetables::Position> {
        self.timetables.next_position(position)
    }

    pub fn stop_at_position_in_mission(&self, position: &Timetables::Position, mission: &Timetables::Mission) -> Stop {
        assert!(position.timetable == mission.timetable);
        self.timetables.stop_at(&mission.timetable, position).clone()
    }

    pub fn stop_at_position_in_trip(&self, position: &Timetables::Position, trip: &Timetables::Trip) -> Stop {
        assert!(position.timetable == trip.vehicle.timetable);
        self.timetables.stop_at(&trip.vehicle.timetable, position).clone()
    }

    pub fn mission_of(&self, trip: &Timetables::Trip) -> Timetables::Mission {
        self.timetables.mission_of(trip)
    }

    pub fn stoptime_idx(&self, position: &Timetables::Position, trip: &Timetables::Trip) -> usize {
        self.timetables.stoptime_idx(position, trip)
    }

    // Panics if `trip` does not go through `position`
    pub fn arrival_time_of(&self, trip: &Timetables::Trip, position: &Timetables::Position) -> SecondsSinceDatasetUTCStart {
        self.timetables.arrival_time_of(trip, position)
    }

    // Panics if `position` is not valid for `trip`
    // None if `trip` does not allows debark at `stop_idx`
    pub fn debark_time_of(
        &self,
        trip: &Timetables::Trip,
        position: &Timetables::Position,
    ) -> Option<SecondsSinceDatasetUTCStart> {
        self.timetables.debark_time_of(trip, position)
    }

    // Panics if `position` is not valid for `trip`
    // None if `trip` does not allows boarding at `stop_idx`
    pub fn board_time_of(
        &self,
        trip: &Timetables::Trip,
        position: &Timetables::Position,
    ) -> Option<SecondsSinceDatasetUTCStart> {
        self.timetables.board_time_of(trip, position)
    }

    pub fn earliest_trip_to_board_at(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Timetables::Mission,
        position: &Timetables::Position,
    ) -> Option<(Timetables::Trip, SecondsSinceDatasetUTCStart)> {
        self.timetables.earliest_trip_to_board_at(waiting_time, mission, position)
    }

    


}
