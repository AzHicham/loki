use super::data::{Mission, Stop, StopPattern, TransitData, Trip};

use super::time::{DaysSinceDatasetStart, SecondsSinceDatasetStart, SecondsSinceDayStart};

use super::ordered_timetable::{Position, StopPatternData, Timetable, Vehicle};

impl TransitData {
    pub fn is_upstream_in_mission(
        &self,
        upstream: &Position,
        downstream: &Position,
        mission: &Mission,
    ) -> bool {
        debug_assert!({
            let pattern = self.pattern(&mission.stop_pattern);
            pattern.is_valid(upstream) && pattern.is_valid(downstream)
        });
        upstream < downstream
    }

    pub fn next_position_in_mission(
        &self,
        position: &Position,
        mission: &Mission,
    ) -> Option<Position> {
        let pattern = self.pattern(&mission.stop_pattern);
        pattern.next_position(position)
    }

    pub fn stop_at_position_in_mission(&self, position: &Position, mission: &Mission) -> Stop {
        let pattern = self.pattern(&mission.stop_pattern);
        *pattern.stop_at(position)
    }

    pub fn stop_at_position_in_trip(&self, position: &Position, trip: &Trip) -> Stop {
        let pattern = self.pattern(&trip.mission.stop_pattern);
        *pattern.stop_at(position)
    }

    pub fn mission_of(&self, trip: &Trip) -> Mission {
        trip.mission.clone()
    }

    pub fn stoptime_idx(&self, position: &Position, _trip: &Trip) -> usize {
        position.idx
    }

    // Panics if `trip` does not go through `stop_idx`
    pub fn arrival_time_of(&self, trip: &Trip, position: &Position) -> SecondsSinceDatasetStart {
        let pattern = &trip.mission.stop_pattern;
        let timetable = &trip.mission.timetable;
        let vehicle = &trip.vehicle;
        let seconds_in_day = self
            .pattern(pattern)
            .arrival_time_at(timetable, vehicle, position);
        SecondsSinceDatasetStart::compose(&trip.day, seconds_in_day)
    }

    // Panics if `position` is not valid for `trip`
    // None if `trip` does not allows debark at `stop_idx`
    pub fn debark_time_of(
        &self,
        trip: &Trip,
        position: &Position,
    ) -> Option<SecondsSinceDatasetStart> {
        let pattern = &trip.mission.stop_pattern;
        let timetable = &trip.mission.timetable;
        let vehicle = &trip.vehicle;
        let has_seconds_in_day = self
            .pattern(pattern)
            .debark_time_at(timetable, vehicle, position);
        has_seconds_in_day.as_ref().map(|seconds_in_day| {
            let days = &trip.day;
            SecondsSinceDatasetStart::compose(days, &seconds_in_day)
        })
    }

    // Panics if `position` is not valid for `trip`
    // None if `trip` does not allows boarding at `stop_idx`
    pub fn board_time_of(
        &self,
        trip: &Trip,
        position: &Position,
    ) -> Option<SecondsSinceDatasetStart> {
        let pattern = &trip.mission.stop_pattern;
        let timetable = &trip.mission.timetable;
        let vehicle = &trip.vehicle;
        let has_seconds_in_day = self
            .pattern(pattern)
            .board_time_at(timetable, vehicle, position);
        has_seconds_in_day.as_ref().map(|seconds_in_day| {
            let days = &trip.day;
            SecondsSinceDatasetStart::compose(days, &seconds_in_day)
        })
    }

    pub fn earliest_trip_to_board_at(
        &self,
        waiting_time: &SecondsSinceDatasetStart,
        mission: &Mission,
        position: &Position,
    ) -> Option<(Trip, SecondsSinceDatasetStart)> {
        let stop_pattern = &mission.stop_pattern;
        let timetable = &mission.timetable;
        self.earliest_vehicle_to_board(waiting_time, stop_pattern, timetable, position)
            .map(|(vehicle, day, arrival_time)| {
                let trip = Trip {
                    mission: mission.clone(),
                    day,
                    vehicle,
                };
                (trip, arrival_time)
            })
    }

    fn earliest_vehicle_to_board(
        &self,
        waiting_time: &SecondsSinceDatasetStart,
        stop_pattern: &StopPattern,
        timetable: &Timetable,
        position: &Position,
    ) -> Option<(Vehicle, DaysSinceDatasetStart, SecondsSinceDatasetStart)> {
        //TODO : reread this and look for optimization

        let pattern_data = self.pattern(stop_pattern);

        debug_assert!(!pattern_data.is_last_position(position));

        let has_next_position = pattern_data.next_position(position);
        // if there is no vehicle to board at this position, we return None
        let next_position = has_next_position?;

        let has_latest_board_time = pattern_data.latest_board_time_at(timetable, position);

        // if there is no latest board time, it means that this position cannot be boarded
        // and we return None
        let latest_board_time_in_day = has_latest_board_time?;

        let mut nb_of_days_to_offset = 0u16;
        let (mut waiting_day, mut waiting_time_in_day) = waiting_time.decompose();
        let mut best_vehicle_day_and_its_arrival_time_at_next_position: Option<(
            Vehicle,
            DaysSinceDatasetStart,
            SecondsSinceDatasetStart,
        )> = None;

        while waiting_time_in_day <= *latest_board_time_in_day {
            let has_vehicle = self.earliest_vehicle_to_board_in_day(
                &waiting_day,
                &waiting_time_in_day,
                timetable,
                pattern_data,
                position,
            );
            if let Some(vehicle) = has_vehicle {
                let vehicle_arrival_time_in_day_at_next_stop =
                    pattern_data.arrival_time_at(timetable, &vehicle, &next_position);
                let vehicle_arrival_time_at_next_stop = SecondsSinceDatasetStart::compose(
                    &waiting_day,
                    vehicle_arrival_time_in_day_at_next_stop,
                );
                if let Some((_, _, best_arrival_time)) =
                    &best_vehicle_day_and_its_arrival_time_at_next_position
                {
                    if vehicle_arrival_time_at_next_stop < *best_arrival_time {
                        best_vehicle_day_and_its_arrival_time_at_next_position =
                            Some((vehicle, waiting_day, vehicle_arrival_time_at_next_stop));
                    }
                } else {
                    best_vehicle_day_and_its_arrival_time_at_next_position =
                        Some((vehicle, waiting_day, vehicle_arrival_time_at_next_stop));
                }
            }
            nb_of_days_to_offset += 1;
            let has_prev_day = waiting_time.decompose_with_days_offset(nb_of_days_to_offset);
            if let Some((day, time_in_day)) = has_prev_day {
                waiting_day = day;
                waiting_time_in_day = time_in_day;
            } else {
                break;
            }
        }

        best_vehicle_day_and_its_arrival_time_at_next_position
    }

    fn earliest_vehicle_to_board_in_day(
        &self,
        day: &DaysSinceDatasetStart,
        time_in_day: &SecondsSinceDayStart,
        timetable: &Timetable,
        pattern_data: &StopPatternData,
        position: &Position,
    ) -> Option<Vehicle> {
        pattern_data.earliest_filtered_vehicle_to_board_at(
            time_in_day,
            timetable,
            position,
            |vehicle_data| {
                let days_pattern = vehicle_data.days_pattern;
                self.calendar.is_allowed(&days_pattern, day)
            },
        )
    }
}
