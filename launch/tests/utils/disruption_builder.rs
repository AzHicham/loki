// Copyright (C) 2017 Kisio Digital and/or its affiliates.
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License as published by the
// Free Software Foundation, version 3.

// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.

// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>

use loki::{
    models,
    models::{base_model::BaseModel, real_time_disruption::kirin_disruption, RealTimeModel},
    time::SecondsSinceTimezonedDayStart,
};

use super::model_builder::IntoTime;

pub struct StopTimesBuilder {
    pub stop_times: Vec<kirin_disruption::StopTime>,
}

impl StopTimesBuilder {
    pub fn new() -> Self {
        Self {
            stop_times: Vec::new(),
        }
    }

    pub fn st(mut self, stop_id: &str, time: impl IntoTime) -> Self {
        let time = SecondsSinceTimezonedDayStart::from_seconds_i64(i64::from(
            time.into_time().total_seconds(),
        ))
        .unwrap();
        let stop_time = kirin_disruption::StopTime {
            stop_id: stop_id.to_string(),
            arrival_time: time,
            departure_time: time,
            flow_direction: loki::timetables::FlowDirection::BoardAndDebark,
        };
        self.stop_times.push(stop_time);
        self
    }

    pub fn finalize(
        self,
        real_time_model: &mut RealTimeModel,
        base_model: &BaseModel,
    ) -> Vec<models::StopTime> {
        real_time_model.make_stop_times(&self.stop_times, base_model)
    }
}
