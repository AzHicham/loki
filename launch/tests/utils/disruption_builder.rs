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
    chrono::NaiveDate,
    models::real_time_disruption::{Disruption, StopTime, Trip, Update},
    time::SecondsSinceTimezonedDayStart,
};

use super::model_builder::{AsDate, IntoTime};

pub fn delete(vehicle_journey_id: String, date: NaiveDate) -> Disruption {
    let trip = Trip {
        vehicle_journey_id: vehicle_journey_id.clone(),
        reference_date: date,
    };
    let update = Update::Delete(trip);
    Disruption {
        id: format!("Delete {} {}", vehicle_journey_id, date),
        updates: vec![update],
    }
}

pub fn add(
    vehicle_journey_id: String,
    date: NaiveDate,
    stop_times_builder: StopTimesBuilder,
) -> Disruption {
    let trip = Trip {
        vehicle_journey_id: vehicle_journey_id.clone(),
        reference_date: date,
    };
    let update = Update::Add(trip, stop_times_builder.stop_times);
    Disruption {
        id: format!("Add {} {}", vehicle_journey_id, date),
        updates: vec![update],
    }
}

pub fn modify(
    vehicle_journey_id: &str,
    date: impl AsDate,
    stop_times_builder: StopTimesBuilder,
) -> Disruption {
    let trip = Trip {
        vehicle_journey_id: vehicle_journey_id.to_string(),
        reference_date: date.as_date(),
    };
    let update = Update::Modify(trip, stop_times_builder.stop_times);
    Disruption {
        id: format!("Modify {} {}", vehicle_journey_id, date.as_date()),
        updates: vec![update],
    }
}

pub struct DisruptionBuilder {}

pub struct StopTimesBuilder {
    stop_times: Vec<StopTime>,
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
        let stop_time = StopTime {
            stop_id: stop_id.to_string(),
            arrival_time: time,
            departure_time: time,
            flow_direction: loki::timetables::FlowDirection::BoardAndDebark,
        };
        self.stop_times.push(stop_time);
        self
    }
}
