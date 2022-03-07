// Copyright  (C) 2020, Kisio Digital and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Kisio Digital (www.kisio.com).
// Help us simplify mobility and open public transport:
// a non ending quest to the responsive locomotion way of traveling!
//
// This contribution is a part of the research and development work of the
// IVA Project which aims to enhance traveler information and is carried out
// under the leadership of the Technological Research Institute SystemX,
// with the partnership and support of the transport organization authority
// Ile-De-France Mobilités (IDFM), SNCF, and public funds
// under the scope of the French Program "Investissements d’Avenir".
//
// LICENCE: This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// Stay tuned using
// twitter @navitia
// channel `#navitia` on riot https://riot.im/app/#/room/#navitia:matrix.org
// https://groups.google.com/d/forum/navitia
// www.navitia.io

mod utils;
use anyhow::{format_err, Error};

use crate::utils::model_builder::AsDateTime;
use launch::solver::Solver;
use loki::{
    chrono::Duration,
    models::{base_model::BaseModel, real_time_model::RealTimeModel, ModelRefs},
    schedule,
    schedule::{ScheduleOn, ScheduleRequestInput, ScheduleResponse},
    NaiveDateTime, PositiveDuration, RealTimeLevel, TransitData,
};
use rstest::{fixture, rstest};
use utils::model_builder::ModelBuilder;

#[fixture]
pub fn fixture_model() -> BaseModel {
    let model = ModelBuilder::new("2020-01-01", "2020-01-02")
        .route("R1", |r| r.name = "R1".into())
        .route("R2", |r| r.name = "R2".into())
        .route("R3", |r| r.name = "R3".into())
        .route("R4", |r| r.name = "R4".into())
        .vj("toto_1", |vj_builder| {
            vj_builder
                .route("R1")
                .st("A", "10:00:00")
                .st("B", "10:05:00")
                .st("C", "10:10:00");
        })
        .vj("toto_2", |vj_builder| {
            vj_builder
                .route("R1")
                .st("A", "11:00:00")
                .st("B", "11:05:00")
                .st("C", "11:10:00");
        })
        .vj("tata_1", |vj_builder| {
            vj_builder
                .route("R2")
                .st("E", "10:05:00")
                .st("F", "10:20:00")
                .st("G", "10:30:00");
        })
        .vj("tata_2", |vj_builder| {
            vj_builder
                .route("R2")
                .st("E", "10:15:00")
                .st("F", "10:30:00")
                .st("G", "10:40:00");
        })
        .vj("tyty", |vj_builder| {
            vj_builder
                .route("R3")
                .st_detailed("X", "10:00:00", "10:00:00", 1, 1, None) // no_pickup & no drop_off
                .st_detailed("Y", "10:10:00", "10:10:00", 0, 0, None);
        })
        .vj("past_midnight_1", |vj_builder| {
            vj_builder
                .route("R4")
                .st("I", "23:50:00")
                .st("J", "24:10:00")
                .st("K", "24:20:00");
        })
        .vj("past_midnight_2", |vj_builder| {
            vj_builder
                .route("R4")
                .st("I", "00:10:00")
                .st("J", "00:30:00")
                .st("K", "00:40:00");
        })
        .build();

    let loads_data = loki::LoadsData::empty();
    BaseModel::from_transit_model(model, loads_data, PositiveDuration::zero()).unwrap()
}

#[rstest]
fn test_no_pickup_dropoff(fixture_model: BaseModel) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // This part checks that the pickup / drop_off fields of a StopTime are taken into account by
    // next_schedule functions

    // Departure Test
    {
        let config = ScheduleConfig::new(
            ScheduleOn::BoardTimes,
            "stop_area:sa:X",
            "2020-01-01T09:59:00",
        );

        let result = build_and_solve_schedule(&config, &fixture_model)?;
        // "tyty" is the only vehicle that goes throught the "stop_area:sa:X"
        // but is cannot be boarded at this stop_point. So we should get no results
        assert_eq!(result.len(), 0);
    }

    // Arrival Test
    {
        let config = ScheduleConfig::new(
            ScheduleOn::DebarkTimes,
            "stop_area:sa:X",
            "2020-01-01T09:59:00",
        );

        let result = build_and_solve_schedule(&config, &fixture_model)?;
        // "tyty" is the only vehicle that goes throught the "stop_area:sa:X"
        // but is cannot be debarked at this stop_point. So we should get no results
        assert_eq!(result.len(), 0);
    }

    Ok(())
}

#[rstest]
fn test_at_first_and_last_stops(fixture_model: BaseModel) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // This test ensure that no departure is possible on terminus stop_point
    // and no arrival is possible on departure stop_point

    // Test Departure at Terminus
    {
        let mut config = ScheduleConfig::new(
            ScheduleOn::BoardTimes,
            "stop_area:sa:C",
            "2020-01-01T10:00:00",
        );
        config.duration = 30 * 24 * 60 * 60; // 1 month

        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 0);
    }

    // Test Arrival at Departure
    {
        let mut config = ScheduleConfig::new(
            ScheduleOn::DebarkTimes,
            "stop_area:sa:A",
            "2020-01-01T10:00:00",
        );
        config.duration = 30 * 24 * 60 * 60; // 1 month

        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 0);
    }

    Ok(())
}

#[rstest]
fn test_range_datetime(fixture_model: BaseModel) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // This part test if until_datetime (ie from_datetime + duration)
    // is working properly

    // Departure tests
    {
        let mut config = ScheduleConfig::new(
            ScheduleOn::BoardTimes,
            "stop_area:sa:B",
            "2020-01-01T10:00:00",
        );

        config.duration = 5 * 60; // 5 min
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 1);
        let stop_time = &result[0];
        assert_eq!(stop_time.time, "2020-01-01T10:05:00".as_datetime());

        config.duration = 2 * 60 * 60; // 2h
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 2);
        let stop_time = &result[1];
        assert_eq!(stop_time.time, "2020-01-01T11:05:00".as_datetime());
    }

    // Arrival tests
    {
        let mut config = ScheduleConfig::new(
            ScheduleOn::DebarkTimes,
            "stop_area:sa:B",
            "2020-01-01T10:00:00",
        );

        config.duration = 5 * 60; // 5 min
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 1);
        let stop_time = &result[0];
        assert_eq!(stop_time.time, "2020-01-01T10:05:00".as_datetime());

        config.duration = 2 * 60 * 60; // 2h
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 2);
        let stop_time = &result[1];
        assert_eq!(stop_time.time, "2020-01-01T11:05:00".as_datetime());
    }

    Ok(())
}

#[rstest]
fn test_invalid_range_datetime(fixture_model: BaseModel) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // This code test multiple from/until_datetime combination
    // some datetime could be outside calendar range
    // We should handle all cases correctly

    // Departure tests
    {
        let mut config = ScheduleConfig::new(
            ScheduleOn::BoardTimes,
            "stop_area:sa:B",
            "2019-12-25T10:00:00",
        );

        // from_datetime < until_datetime < calendar.start < calendar.end
        config.duration = 2 * 60 * 60; // 2h
        let result = build_and_solve_schedule(&config, &fixture_model);
        let error = format!("{:?}", result.as_ref().err().unwrap());
        assert_eq!(error, "BadFromDatetime");

        // from_datetime < calendar.start < until_datetime < calendar.end
        config.duration = 8 * 24 * 60 * 60; // 7 days ie : until_datetime = 2020-02-01 < calendar.end
        let result = build_and_solve_schedule(&config, &fixture_model);
        let error = format!("{:?}", result.as_ref().err().unwrap());
        assert_eq!(error, "BadFromDatetime");

        // calendar.start < from_datetime < calendar.end < until_datetime
        config.from_datetime = "2020-01-01T10:00:00".as_datetime();
        config.duration = 30 * 24 * 60 * 60; // 1 month
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        // 2 vj per day and calendar is composed of 2 days 2020-01-01 & 2020-01-02
        assert_eq!(result.len(), 4);

        // calendar.start  < calendar.end < from_datetime < until_datetime
        config.from_datetime = "2020-01-10T10:00:00".as_datetime();
        config.duration = 30 * 24 * 60 * 60; // 1 month
        let result = build_and_solve_schedule(&config, &fixture_model);
        let error = format!("{:?}", result.as_ref().err().unwrap());
        assert_eq!(error, "BadFromDatetime");
    }

    // Arrival tests
    {
        let mut config = ScheduleConfig::new(
            ScheduleOn::DebarkTimes,
            "stop_area:sa:B",
            "2019-12-25T10:00:00",
        );

        // from_datetime < until_datetime < calendar.start < calendar.end
        config.duration = 2 * 60 * 60; // 2h
        let result = build_and_solve_schedule(&config, &fixture_model);
        let error = format!("{:?}", result.as_ref().err().unwrap());
        assert_eq!(error, "BadFromDatetime");

        // from_datetime < calendar.start < until_datetime < calendar.end
        config.duration = 8 * 24 * 60 * 60; // 7 days ie : until_datetime = 2020-02-01 < calendar.end
        let result = build_and_solve_schedule(&config, &fixture_model);
        let error = format!("{:?}", result.as_ref().err().unwrap());
        assert_eq!(error, "BadFromDatetime");

        // calendar.start < from_datetime < calendar.end < until_datetime
        config.from_datetime = "2020-01-01T10:00:00".as_datetime();
        config.duration = 30 * 24 * 60 * 60; // 1 month
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        // 2 vj per day and calendar is composed of 2 days 2020-01-01 & 2020-01-02
        assert_eq!(result.len(), 4);

        // calendar.start  < calendar.end < from_datetime < until_datetime
        config.from_datetime = "2020-01-10T10:00:00".as_datetime();
        config.duration = 30 * 24 * 60 * 60; // 1 month
        let result = build_and_solve_schedule(&config, &fixture_model);
        let error = format!("{:?}", result.as_ref().err().unwrap());
        assert_eq!(error, "BadFromDatetime");
    }

    Ok(())
}

#[rstest]
fn test_past_midnight_vehicle(fixture_model: BaseModel) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // This code checks that vehicle valid on day : from_date - 2 & from_date -1
    // and with a next_schedule (departure/arrival) after from_datetime are returned as expected

    // Departure Tests
    {
        let mut config = ScheduleConfig::new(
            ScheduleOn::BoardTimes,
            "stop_area:sa:J",
            "2020-01-02T00:00:00",
        );
        config.duration = 10 * 60; // 10m
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 1);
        let stop_time = &result[0];
        assert_eq!(stop_time.time, "2020-01-02T00:10:00".as_datetime());

        config.duration = 30 * 60; // 30m
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 2);
        let stop_time = &result[1];
        assert_eq!(stop_time.time, "2020-01-02T00:30:00".as_datetime());
    }

    // Arrival Tests
    {
        let mut config = ScheduleConfig::new(
            ScheduleOn::DebarkTimes,
            "stop_area:sa:J",
            "2020-01-02T00:00:00",
        );
        config.duration = 10 * 60; // 10m
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 1);
        let stop_time = &result[0];
        assert_eq!(stop_time.time, "2020-01-02T00:10:00".as_datetime());

        config.duration = 30 * 60; // 30m
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 2);
        let stop_time = &result[1];
        assert_eq!(stop_time.time, "2020-01-02T00:30:00".as_datetime());
    }

    Ok(())
}

#[rstest]
fn test_on_route(fixture_model: BaseModel) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // In this test we expect to receive all departure of all stop_points of route R1
    // after a certain datetime (we test multiple datetime)

    // Departure Tests
    {
        let mut config =
            ScheduleConfig::new(ScheduleOn::BoardTimes, "route:R1", "2020-01-01T10:00:00");
        config.duration = 10 * 60 * 60; // 10h

        // on route R1 we have 3 stop_points (A,B,C) and 2 valid vj per day
        // we expect 3 Next Departure with from_datetime = 2020-01-02T10:00:00
        // At stop_point A (10:00:00 & 11:00:00) & B (10:05:00 & 11:05:00)
        // Note: Last stop_point of a VJ is a terminus and considered as not bordable
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 4);

        // We update from_datetime to retrieve departures after 2020-01-01T11:00:00
        // We expect only two departures after 11:00:00
        // At stop_point A (11:00:00) & B (11:05:00)
        config.from_datetime = "2020-01-01T11:00:00".as_datetime();
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 2);
    }

    // Arrival Tests
    {
        let mut config =
            ScheduleConfig::new(ScheduleOn::DebarkTimes, "route:R1", "2020-01-01T10:00:00");
        config.duration = 10 * 60 * 60; // 10h

        // on route R1 we have 3 stop_points and 2 valid vj per day
        // we expect 4 Next Arrival with from_datetime = 2020-01-02T10:00:00
        // At stop_point C (10:10:00 & 11:10:00) & B (10:05:00 & 11:05:00)
        // Note: Last stop_point of a VJ is a terminus and considered as not bordable
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 4);

        // We update from_datetime to retrieve departures after 2020-01-01T11:00:00
        // We expect only two arrival after 11:00:00
        // At stop_point C (11:10:00) & B (11:05:00)
        config.from_datetime = "2020-01-01T11:00:00".as_datetime();
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 2);
    }

    Ok(())
}

#[rstest]
fn test_on_network(fixture_model: BaseModel) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // In this test we expect to receive all departure of all stop_points of network N1
    // after a certain datetime

    // Departure Tests
    {
        let mut config = ScheduleConfig::new(
            ScheduleOn::BoardTimes,
            "network:default_network",
            "2020-01-02T10:00:00",
        );
        // fetch next_schedule between 2020-01-02T10:00:00 - 14:00:00
        config.duration = 4 * 60 * 60; // 4h

        // on network "default_network" we have 11 stop_points
        // we expect 8 Next Departure in range 2020-01-01T10:00:00 - 14:00:00
        // At stop_point A (10:00:00 & 11:00:00) & B (10:05:00 & 11:05:00), no departure on C
        // At stop_point E (10:05:00 & 10:15:00) & F (10:20:00 & 10:30:00), no departure on G
        // no departure on I & J & K on this time range
        // no departure on X (no pickup) & Y (terminus)
        // Note: Last stop_point of a VJ is a terminus and considered as not bordable
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 8);
    }

    // Arrival Tests
    {
        let mut config = ScheduleConfig::new(
            ScheduleOn::DebarkTimes,
            "network:default_network",
            "2020-01-02T10:00:00",
        );
        // fetch next_schedule between 2020-01-02T10:00:00 - 14:00:00
        config.duration = 4 * 60 * 60; // 4h

        // on network "default_network" we have 11 stop_points
        // we expect 9 Next Arrivals in range 2020-01-01T10:00:00 - 14:00:00
        // At stop_point C (10:10:00 & 11:10:00) & B (10:05:00 & 11:05:00), no arrivals on A
        // At stop_point G (10:30:00 & 10:40:00) & F (10:20:00 & 10:30:00), no arrivals on E
        // At stop_point Y (10:10:00) no departure on X (no pickup)
        // no departure on I & J & K on this time range
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 9);
    }

    Ok(())
}

#[rstest]
fn test_forbidden_filter(fixture_model: BaseModel) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // In this test we call next_departures/next_arrivals with a forbidden filter

    // Departure Tests
    {
        let mut config =
            ScheduleConfig::new(ScheduleOn::BoardTimes, "route:R1", "2020-01-02T10:00:00");
        // we don't want next_departure of sa:A, sa:B
        // And there is not next_departure at stop:C (terminus)
        config.forbidden_uris = vec!["stop_area:sa:A", "stop_area:sa:B"];
        config.duration = 4 * 60 * 60; // 4h

        // on route R1 we have 3 stop_points A, B, C
        // we expect only 2 results :
        //  - 0 on stop_point A because of the forbidden_uris
        //  - 0 on stop_point B because of the forbidden_uris
        //  - 0 on stop_point C because there is no departure on terminus
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 0);

        config.forbidden_uris = vec!["stop_area:sa:B"];
        config.duration = 4 * 60 * 60; // 4h

        // on route R1 we have 3 stop_points A, B, C
        // we expect only 2 results :
        //  - 2 on stop_point A
        //  - 0 on stop_point B because of the forbidden_uris
        //  - 0 on stop_point C because there is no departure on terminus
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 2);
    }

    // Arrival Tests
    {
        let mut config =
            ScheduleConfig::new(ScheduleOn::DebarkTimes, "route:R1", "2020-01-02T10:00:00");
        // we don't want next_departure of sa:A, sa:B
        // And there is not next_departure at stop:C (terminus)
        config.forbidden_uris = vec!["stop_area:sa:B", "stop_area:sa:C"];
        config.duration = 4 * 60 * 60; // 4h

        // on route R1 we have 3 stop_points A, B, C
        // we expect only 2 results :
        //  - 0 on stop_point A because there is no arrivals on departure stop_point
        //  - 0 on stop_point B because of the forbidden_uris
        //  - 0 on stop_point C because of the forbidden_uris
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 0);

        config.forbidden_uris = vec!["stop_area:sa:B"];
        config.duration = 4 * 60 * 60; // 4h

        // on route R1 we have 3 stop_points A, B, C
        // we expect only 2 results :
        //  - 0 on stop_point A because there is no arrivals on departure stop_point
        //  - 2 on stop_point B
        //  - 0 on stop_point C because of the forbidden_uris
        let result = build_and_solve_schedule(&config, &fixture_model)?;
        assert_eq!(result.len(), 2);
    }

    Ok(())
}

struct ScheduleConfig<'a> {
    pub schedule_on: ScheduleOn,
    pub input_filter: String,
    pub from_datetime: NaiveDateTime,
    pub duration: i64,
    pub nb_max_responses: usize,
    pub real_time_level: RealTimeLevel,
    pub forbidden_uris: Vec<&'a str>,
}

impl<'a> ScheduleConfig<'a> {
    pub fn new(schedule_on: ScheduleOn, input_filter: &str, datetime: impl AsDateTime) -> Self {
        ScheduleConfig {
            schedule_on,
            input_filter: input_filter.into(),
            from_datetime: datetime.as_datetime(),
            duration: 3600,
            nb_max_responses: 10,
            real_time_level: RealTimeLevel::Base,
            forbidden_uris: vec![],
        }
    }
}

fn build_and_solve_schedule(
    config: &ScheduleConfig,
    base_model: &BaseModel,
) -> Result<Vec<ScheduleResponse>, Error> {
    use loki::DataTrait;

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(base_model, &real_time_model);
    let data: TransitData = launch::read::build_transit_data(model_refs.base);

    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    let request = make_schedule_request(
        config.schedule_on.clone(),
        &config.input_filter,
        config.from_datetime,
        config.duration,
        config.nb_max_responses,
        config.real_time_level,
        &*config.forbidden_uris,
        &model_refs,
        &data,
    )?;

    let has_filters = schedule::generate_vehicle_filters_for_schedule_request(
        &*config.forbidden_uris,
        &model_refs,
    );

    solver
        .solve_schedule(&data, &model_refs, &request, has_filters)
        .map_err(|err| format_err!("{:?}", err))
}

fn make_schedule_request(
    schedule_on: ScheduleOn,
    input_filter: &str,
    from_datetime: NaiveDateTime,
    duration: i64,
    nb_max_responses: usize,
    real_time_level: RealTimeLevel,
    forbidden_uri: &[&str],
    model: &ModelRefs,
    data: &TransitData,
) -> Result<ScheduleRequestInput, Error> {
    use loki::DataTrait;
    let until_datetime = from_datetime + Duration::seconds(duration);
    let until_datetime = std::cmp::min(until_datetime, data.calendar().last_datetime());

    let input_stop_points =
        schedule::generate_stops_for_schedule_request(input_filter, forbidden_uri, model);

    Ok(ScheduleRequestInput {
        input_stop_points,
        from_datetime,
        until_datetime,
        nb_max_responses,
        real_time_level,
        schedule_on,
    })
}
