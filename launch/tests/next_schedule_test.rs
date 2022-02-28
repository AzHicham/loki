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
use loki::chrono::Duration;
use loki::schedule::{ScheduleOn, ScheduleRequestInput, ScheduleResponse};
use loki::tracing::info;
use loki::{
    models::{base_model::BaseModel, real_time_model::RealTimeModel, ModelRefs},
    schedule, NaiveDateTime, PositiveDuration, RealTimeLevel, TransitData,
};
use rstest::{fixture, rstest};
use utils::model_builder::ModelBuilder;

#[fixture]
pub fn fixture_model() -> BaseModel {
    let model = ModelBuilder::new("2020-01-01", "2020-01-02")
        .network("N1", |n| n.name = "N1".into())
        .route("R1", |r| r.name = "R1".into())
        .route("R2", |r| r.name = "R2".into())
        .route("R3", |r| r.name = "R3".into())
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
                .st("E", "10:05:00")
                .st("F", "10:20:00")
                .st("G", "10:30:00");
        })
        .vj("tyty", |vj_builder| {
            vj_builder
                .route("R3")
                .st_detailed("X", "10:00:00", "10:00:00", 1, 1, None) // no_pickup & no drop_off
                .st_detailed("Y", "10:10:00", "10:10:00", 0, 0, None);
        })
        .build();

    let loads_data = loki::LoadsData::empty();
    BaseModel::from_transit_model(model, loads_data, PositiveDuration::zero()).unwrap()
}

#[rstest]
fn test_no_pickup_dropoff(fixture_model: BaseModel) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    let config = ScheduleConfig::new(
        ScheduleOn::BoardTimes,
        "stop_area:sa:X",
        "2020-01-01T09:59:00",
    );

    let result = build_and_solve_schedule(&config, &fixture_model)?;

    assert_eq!(result.len(), 0);

    Ok(())
}

#[rstest]
fn test_range_datetime(fixture_model: BaseModel) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    let mut config = ScheduleConfig::new(
        ScheduleOn::BoardTimes,
        "stop_area:sa:A",
        "2020-01-01T10:00:00",
    );
    config.duration = 5 * 60; // 5 min

    let result = build_and_solve_schedule(&config, &fixture_model)?;
    assert_eq!(result.len(), 1);
    let stop_time = &result[0];
    assert_eq!(stop_time.time, "2020-01-01T10:00:00".as_datetime());

    config.duration = 60 * 60; // 1h
    let result = build_and_solve_schedule(&config, &fixture_model)?;
    assert_eq!(result.len(), 2);
    let stop_time = &result[1];
    assert_eq!(stop_time.time, "2020-01-01T11:00:00".as_datetime());

    Ok(())
}

#[rstest]
fn test_range_datetime_calendar(fixture_model: BaseModel) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    let mut config = ScheduleConfig::new(
        ScheduleOn::BoardTimes,
        "stop_area:sa:A",
        "2019-12-25T10:00:00",
    );
    config.duration = 60 * 60; // 1h

    // from_datetime < calendar.start
    // until_datetime < calendar.end
    let result = build_and_solve_schedule(&config, &fixture_model);
    let error = format!("{:?}", result.as_ref().err().unwrap());
    assert_eq!(error, "BadFromDatetime");

    // from_datetime > calendar.start
    // until_datetime > calendar.end
    config.from_datetime = "2020-01-01T10:00:00".as_datetime();
    config.duration = 3600 * 24 * 30; // 1 month

    let result = build_and_solve_schedule(&config, &fixture_model)?;
    // 2 vj per day and calendar is composed of 2 days 2020-01-01 & 2020-01-02
    assert_eq!(result.len(), 4);

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
