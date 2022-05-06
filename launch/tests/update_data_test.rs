// Copyright  (C) 2021, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Hove (www.kisio.com).
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

use anyhow::Error;
use launch::solver::Solver;

use loki::{
    chrono::NaiveDate,
    chrono_tz::UTC,
    models::{
        self, base_model::BaseModel, real_time_model::RealTimeModel, ModelRefs, StopTime,
        VehicleJourneyIdx,
    },
    timetables::InsertionError,
    DataTrait, RealTimeLevel,
};
use utils::{
    disruption_builder::StopTimesBuilder,
    model_builder::{AsDate, ModelBuilder},
    Config,
};

#[test]
fn remove_vj() -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    let model = ModelBuilder::new("2020-01-01", "2020-01-02")
        .vj("first", |vj_builder| {
            vj_builder
                .st("A", "10:00:00")
                .st("B", "10:05:00")
                .st("C", "10:10:00");
        })
        .vj("second", |vj_builder| {
            vj_builder
                .st("E", "10:20:00")
                .st("F", "10:30:00")
                .st("G", "10:40:00");
        })
        .add_transfer("C", "E", "00:02:00")
        .build();

    let config = Config::new("2020-01-01T08:00:00", "A", "G");

    let base_model = BaseModel::from_transit_model(
        model,
        loki::LoadsData::empty(),
        config.default_transfer_duration,
    )
    .unwrap();

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&base_model, &real_time_model);

    let mut data = launch::read::build_transit_data(&base_model);

    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    {
        let request_input = utils::make_request_from_config(&config);
        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        assert_eq!(
            model_refs.vehicle_journey_name(&journey.first_vehicle.vehicle_journey),
            "first"
        );
        assert_eq!(journey.connections.len(), 1);
        let second_vehicle = &journey.connections[0].2;
        assert_eq!(
            model_refs.vehicle_journey_name(&second_vehicle.vehicle_journey),
            "second"
        );
    }

    let vehicle_journey_idx = base_model.vehicle_journey_idx("first").unwrap();
    let vj_idx = VehicleJourneyIdx::Base(vehicle_journey_idx);

    data.remove_real_time_vehicle(&vj_idx, "2020-01-01".as_date())
        .unwrap();

    {
        let mut request_input = utils::make_request_from_config(&config);
        request_input.real_time_level = RealTimeLevel::RealTime;
        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        assert_eq!(responses.len(), 0);
    }

    {
        let config = Config::new("2020-01-02T08:00:00", "A", "G");
        let request_input = utils::make_request_from_config(&config);
        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        assert_eq!(
            model_refs.vehicle_journey_name(&journey.first_vehicle.vehicle_journey),
            "first"
        );
        assert_eq!(journey.connections.len(), 1);
        let second_vehicle = &journey.connections[0].2;
        assert_eq!(
            model_refs.vehicle_journey_name(&second_vehicle.vehicle_journey),
            "second"
        );
    }

    Ok(())
}

#[test]
fn remove_successive_vj() -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    let model = ModelBuilder::new("2020-01-01", "2020-01-02")
        .vj("first", |vj_builder| {
            vj_builder
                .st("A", "10:00:00")
                .st("B", "10:05:00")
                .st("C", "10:10:00");
        })
        .vj("second", |vj_builder| {
            vj_builder
                .st("A", "11:00:00")
                .st("B", "11:05:00")
                .st("C", "11:10:00");
        })
        .vj("third", |vj_builder| {
            vj_builder
                .st("A", "12:00:00")
                .st("B", "12:05:00")
                .st("C", "12:10:00");
        })
        .build();

    let config = Config::new("2020-01-01T08:00:00", "A", "C");

    let base_model = BaseModel::from_transit_model(
        model,
        loki::LoadsData::empty(),
        config.default_transfer_duration,
    )
    .unwrap();

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&base_model, &real_time_model);

    let mut data = launch::read::build_transit_data(&base_model);

    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    {
        let request_input = utils::make_request_from_config(&config);
        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        assert_eq!(
            model_refs.vehicle_journey_name(&journey.first_vehicle.vehicle_journey),
            "first"
        );
    }

    {
        let vehicle_journey_idx = base_model.vehicle_journey_idx("first").unwrap();
        let vj_idx = VehicleJourneyIdx::Base(vehicle_journey_idx);
        data.remove_real_time_vehicle(&vj_idx, "2020-01-01".as_date())
            .unwrap();
    }

    {
        let mut request_input = utils::make_request_from_config(&config);
        request_input.real_time_level = RealTimeLevel::RealTime;
        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        assert_eq!(
            model_refs.vehicle_journey_name(&journey.first_vehicle.vehicle_journey),
            "second"
        );
    }

    {
        let vehicle_journey_idx = base_model.vehicle_journey_idx("second").unwrap();
        let vj_idx = VehicleJourneyIdx::Base(vehicle_journey_idx);
        data.remove_real_time_vehicle(&vj_idx, "2020-01-01".as_date())
            .unwrap();
    }

    {
        let mut request_input = utils::make_request_from_config(&config);
        request_input.real_time_level = RealTimeLevel::RealTime;
        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        assert_eq!(
            model_refs.vehicle_journey_name(&journey.first_vehicle.vehicle_journey),
            "third"
        );
    }

    {
        let vehicle_journey_idx = base_model.vehicle_journey_idx("third").unwrap();
        let vj_idx = VehicleJourneyIdx::Base(vehicle_journey_idx);
        data.remove_real_time_vehicle(&vj_idx, "2020-01-01".as_date())
            .unwrap();
    }

    {
        let mut request_input = utils::make_request_from_config(&config);
        request_input.real_time_level = RealTimeLevel::RealTime;
        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        assert_eq!(responses.len(), 0);
    }

    Ok(())
}

#[test]
fn remove_middle_vj() -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    let model = ModelBuilder::new("2020-01-01", "2020-01-02")
        .vj("first", |vj_builder| {
            vj_builder
                .st("A", "10:00:00")
                .st("B", "10:05:00")
                .st("C", "10:10:00");
        })
        .vj("second", |vj_builder| {
            vj_builder
                .st("A", "11:00:00")
                .st("B", "11:05:00")
                .st("C", "11:10:00");
        })
        .vj("third", |vj_builder| {
            vj_builder
                .st("A", "12:00:00")
                .st("B", "12:05:00")
                .st("C", "12:10:00");
        })
        .build();

    let config = Config::new("2020-01-01T10:50:00", "A", "C");
    let base_model = BaseModel::from_transit_model(
        model,
        loki::LoadsData::empty(),
        config.default_transfer_duration,
    )
    .unwrap();

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&base_model, &real_time_model);

    let mut data = launch::read::build_transit_data(&base_model);

    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    {
        let request_input = utils::make_request_from_config(&config);
        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        assert_eq!(
            model_refs.vehicle_journey_name(&journey.first_vehicle.vehicle_journey),
            "second"
        );
    }

    {
        let vehicle_journey_idx = base_model.vehicle_journey_idx("first").unwrap();
        let vj_idx = VehicleJourneyIdx::Base(vehicle_journey_idx);
        data.remove_real_time_vehicle(&vj_idx, "2020-01-01".as_date())
            .unwrap();
    }

    {
        let request_input = utils::make_request_from_config(&config);
        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        assert_eq!(
            model_refs.vehicle_journey_name(&journey.first_vehicle.vehicle_journey),
            "second"
        );
    }

    {
        let vehicle_journey_idx = base_model.vehicle_journey_idx("third").unwrap();
        let vj_idx = VehicleJourneyIdx::Base(vehicle_journey_idx);
        data.remove_real_time_vehicle(&vj_idx, "2020-01-01".as_date())
            .unwrap();
    }

    {
        let request_input = utils::make_request_from_config(&config);
        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        assert_eq!(
            model_refs.vehicle_journey_name(&journey.first_vehicle.vehicle_journey),
            "second"
        );
    }

    Ok(())
}

#[test]
fn modify_vj() -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    let model = ModelBuilder::new("2020-01-01", "2020-01-02")
        .vj("first", |vj_builder| {
            vj_builder
                .st("A", "10:00:00")
                .st("B", "10:05:00")
                .st("C", "10:10:00");
        })
        .build();

    let config = Config::new("2020-01-01T09:50:00", "A", "C");
    let base_model = BaseModel::from_transit_model(
        model,
        loki::LoadsData::empty(),
        config.default_transfer_duration,
    )
    .unwrap();

    let mut real_time_model = RealTimeModel::new();

    let config = Config::new("2020-01-01T09:50:00", "A", "C");
    let request_input = utils::make_request_from_config(&config);

    let mut data = launch::read::build_transit_data(&base_model);

    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    {
        let model_refs = ModelRefs::new(&base_model, &real_time_model);
        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        assert_eq!(
            model_refs.vehicle_journey_name(&journey.first_vehicle.vehicle_journey),
            "first"
        );
    }

    {
        let stop_times = StopTimesBuilder::new()
            .st("A", "09:45:00")
            .st("B", "10:05:00")
            .st("C", "10:10:00")
            .finalize(&mut real_time_model, &base_model);

        let date = "2020-01-01".as_date();

        let dates = std::iter::once(date);
        let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
        let flows = stop_times.iter().map(|stop_time| stop_time.flow_direction);
        let board_times = stop_times.iter().map(|stop_time| stop_time.board_time);
        let debark_times = stop_times.iter().map(|stop_time| stop_time.debark_time);

        let base_vj_idx = base_model.vehicle_journey_idx("first").unwrap();
        let vj_idx = VehicleJourneyIdx::Base(base_vj_idx);

        let result = data.modify_real_time_vehicle(
            stops,
            flows,
            board_times,
            debark_times,
            base_model.loads_data(),
            dates,
            UTC,
            &vj_idx,
        );
        assert!(result.is_ok());
    }

    {
        let mut request_input = request_input.clone();
        request_input.real_time_level = RealTimeLevel::RealTime;

        let model_refs = ModelRefs::new(&base_model, &real_time_model);

        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        // the request is to depart at 9:50, but the vehicle depart at 9:45 at the real time level
        // so we should not obtain any result
        assert_eq!(responses.len(), 0);
    }

    {
        let mut request_input = request_input.clone();
        request_input.real_time_level = RealTimeLevel::Base;
        let model_refs = ModelRefs::new(&base_model, &real_time_model);

        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        // the request is to depart at 9:50,
        // the vehicle depart at 9:45 at the real time level, but its departure is still at 10:00 at the base level
        // so we should obtain a result
        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        assert_eq!(
            model_refs.vehicle_journey_name(&journey.first_vehicle.vehicle_journey),
            "first"
        );
    }

    Ok(())
}

#[test]
fn modify_vj_with_local_zone() -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    let model = ModelBuilder::new("2020-01-01", "2020-01-02")
        .vj("first", |vj_builder| {
            vj_builder
                .st_detailed("A", "10:00:00", "10:00:00", 0u8, 0u8, None)
                .st_detailed("B", "10:05:00", "10:05:00", 0u8, 0u8, None)
                .st_detailed("C", "10:10:00", "10:10:00", 0u8, 0u8, Some(1u16))
                .st_detailed("D", "10:15:00", "10:15:00", 0u8, 0u8, Some(1u16))
                .st_detailed("E", "10:20:00", "10:20:00", 0u8, 0u8, Some(2u16))
                .st_detailed("F", "10:25:00", "10:25:00", 0u8, 0u8, Some(2u16))
                .st_detailed("G", "10:30:00", "10:30:00", 0u8, 0u8, Some(3u16));
        })
        .build();

    let config = Config::new("2020-01-01T09:50:00", "A", "G");
    let base_model = BaseModel::from_transit_model(
        model,
        loki::LoadsData::empty(),
        config.default_transfer_duration,
    )
    .unwrap();

    let mut real_time_model = RealTimeModel::new();

    let request_input = utils::make_request_from_config(&config);

    let mut data = launch::read::build_transit_data(&base_model);

    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    {
        let model_refs = ModelRefs::new(&base_model, &real_time_model);
        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        assert_eq!(
            model_refs.vehicle_journey_name(&journey.first_vehicle.vehicle_journey),
            "first"
        );
    }

    // let's modify the vehicle
    {
        let stop_times = StopTimesBuilder::new()
            .st("A", "10:00:00")
            .st("B", "10:05:00")
            .st("C", "10:10:00")
            .st("D", "10:15:00")
            .st("E", "10:20:00")
            .st("F", "10:25:00")
            // the stop_time of the last stop is changed
            .st("G", "10:45:00")
            .finalize(&mut real_time_model, &base_model);

        let date = "2020-01-01".as_date();

        let dates = std::iter::once(date);
        let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
        let flows = stop_times.iter().map(|stop_time| stop_time.flow_direction);
        let board_times = stop_times.iter().map(|stop_time| stop_time.board_time);
        let debark_times = stop_times.iter().map(|stop_time| stop_time.debark_time);

        let base_vj_idx = base_model.vehicle_journey_idx("first").unwrap();
        let vj_idx = VehicleJourneyIdx::Base(base_vj_idx);

        let result = data.modify_real_time_vehicle(
            stops,
            flows,
            board_times,
            debark_times,
            base_model.loads_data(),
            dates,
            UTC,
            &vj_idx,
        );
        assert!(result.is_ok());
    }

    {
        let mut request_input = request_input.clone();
        request_input.real_time_level = RealTimeLevel::RealTime;

        let model_refs = ModelRefs::new(&base_model, &real_time_model);

        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        // All base vehicle journeys are deactivated
        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        assert_eq!(
            model_refs.vehicle_journey_name(&journey.first_vehicle.vehicle_journey),
            "first"
        );
        // the arrival time now is 10:45:00 instead of 10:30:00
        assert_eq!(
            journey.arrival.to_datetime,
            NaiveDate::from_ymd(2020, 1, 1).and_hms(10, 45, 0)
        );
    }

    // we run the request again on Base level
    {
        let mut request_input = request_input.clone();
        request_input.real_time_level = RealTimeLevel::Base;
        let model_refs = ModelRefs::new(&base_model, &real_time_model);

        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        assert_eq!(
            model_refs.vehicle_journey_name(&journey.first_vehicle.vehicle_journey),
            "first"
        );
        // the arrival time is still 10:30:00 on Base level
        assert_eq!(
            journey.arrival.to_datetime,
            NaiveDate::from_ymd(2020, 1, 1).and_hms(10, 30, 0)
        );
    }

    // let's now try another request, from C to D
    let config = Config::new("2020-01-01T09:50:00", "C", "D");
    let request_input = utils::make_request_from_config(&config);
    // we run the request on Base level
    {
        let model_refs = ModelRefs::new(&base_model, &real_time_model);

        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        // we should not get any response, since A and C are in the same local zone
        // on the base level
        assert_eq!(responses.len(), 0);
    }
    // we run the request on real time level
    {
        let mut request_input = request_input.clone();
        request_input.real_time_level = RealTimeLevel::RealTime;
        let model_refs = ModelRefs::new(&base_model, &real_time_model);

        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        // we should not get a response, since there is no local zone on
        // the real time vehicle
        assert_eq!(responses.len(), 1);
    }

    Ok(())
}

#[test]
fn remove_vj_with_local_zone() -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    let model = ModelBuilder::new("2020-01-01", "2020-01-03")
        .vj("first", |vj_builder| {
            vj_builder
                .st_detailed("A", "10:00:00", "10:00:00", 0u8, 0u8, None)
                .st_detailed("B", "10:05:00", "10:05:00", 0u8, 0u8, None)
                .st_detailed("C", "10:10:00", "10:10:00", 0u8, 0u8, Some(1u16))
                .st_detailed("D", "10:15:00", "10:15:00", 0u8, 0u8, Some(1u16))
                .st_detailed("E", "10:20:00", "10:20:00", 0u8, 0u8, Some(2u16))
                .st_detailed("F", "10:25:00", "10:25:00", 0u8, 0u8, Some(2u16))
                .st_detailed("G", "10:30:00", "10:30:00", 0u8, 0u8, Some(3u16));
        })
        .vj("second", |vj_builder| {
            vj_builder
                .st_detailed("A", "10:00:00", "10:00:00", 0u8, 0u8, None)
                .st_detailed("G", "10:35:00", "10:35:00", 0u8, 0u8, None);
        })
        .build();

    let config = Config::new("2020-01-02T09:50:00", "A", "G");
    let base_model = BaseModel::from_transit_model(
        model,
        loki::LoadsData::empty(),
        config.default_transfer_duration,
    )
    .unwrap();

    let real_time_model = RealTimeModel::new();

    let request_input = utils::make_request_from_config(&config);

    let mut data = launch::read::build_transit_data(&base_model);

    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    // we test, before removing first vj, if we can find a solution with first vj in both Base and
    // RealTime
    {
        let model_refs = ModelRefs::new(&base_model, &real_time_model);
        let mut request_input = request_input.clone();

        request_input.real_time_level = RealTimeLevel::Base;
        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        assert_eq!(
            model_refs.vehicle_journey_name(&journey.first_vehicle.vehicle_journey),
            "first"
        );

        request_input.real_time_level = RealTimeLevel::RealTime;
        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        assert_eq!(
            model_refs.vehicle_journey_name(&journey.first_vehicle.vehicle_journey),
            "first"
        );
        // the arrival time is still 10:30:00 on Base level
        assert_eq!(
            journey.arrival.to_datetime,
            NaiveDate::from_ymd(2020, 1, 2).and_hms(10, 30, 0)
        );
    }

    // Now we are going to remove the vj on 2020-01-02T09:50:00
    {
        let vehicle_journey_idx = base_model.vehicle_journey_idx("first").unwrap();
        let vj_idx = VehicleJourneyIdx::Base(vehicle_journey_idx);

        // remove the vj
        data.remove_real_time_vehicle(&vj_idx, "2020-01-02".as_date())
            .unwrap();

        let mut request_input = request_input.clone();
        request_input.real_time_level = RealTimeLevel::RealTime;

        let model_refs = ModelRefs::new(&base_model, &real_time_model);

        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        // the first vehicle_journey is totally removed for RealTime, the only solution is to take
        // the second vehicle_journey
        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        assert_eq!(
            model_refs.vehicle_journey_name(&journey.first_vehicle.vehicle_journey),
            "second"
        );
        // the arrival time is still 10:30:00 on Base level
        assert_eq!(
            journey.arrival.to_datetime,
            NaiveDate::from_ymd(2020, 1, 2).and_hms(10, 35, 0)
        );
    }

    {
        // we retest with the base schedule after removing the first from timetable and we should be
        // able to take the first vehicle_journey
        let mut request_input = request_input;
        request_input.real_time_level = RealTimeLevel::Base;
        let model_refs = ModelRefs::new(&base_model, &real_time_model);

        let responses = solver.solve_journey_request(
            &data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            &config.datetime_represent,
        )?;

        // the request is to depart at 9:50,
        // the vehicle depart at 9:45 at the real time level, but its departure is still at 10:00 at the base level
        // so we should obtain a result
        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        assert_eq!(
            model_refs.vehicle_journey_name(&journey.first_vehicle.vehicle_journey),
            "first"
        );
        // the arrival time is still 10:30:00 on Base level
        assert_eq!(
            journey.arrival.to_datetime,
            NaiveDate::from_ymd(2020, 1, 2).and_hms(10, 30, 0)
        );
    }
    Ok(())
}

#[test]
fn insert_invalid_vj() -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    let model = ModelBuilder::new("2020-01-01", "2020-01-02")
        .vj("first", |vj_builder| {
            vj_builder
                .st("A", "10:00:00")
                .st("B", "10:05:00")
                .st("C", "10:10:00");
        })
        .build();

    let config = Config::new("2020-01-01T08:00:00", "A", "C");

    let base_model = BaseModel::from_transit_model(
        model,
        loki::LoadsData::empty(),
        config.default_transfer_duration,
    )
    .unwrap();

    let mut real_time_model = RealTimeModel::new();
    let _model_refs = ModelRefs::new(&base_model, &real_time_model);

    let mut data = launch::read::build_transit_data(&base_model);

    // insert a vehicle with a date outside of the calendar of the data
    {
        let vehicle_journey_id = "invalid_date_vj".to_string();
        let date = "1999-01-01".as_date();
        let new_vj_idx = real_time_model.insert_new_vehicle_journey(&vehicle_journey_id);
        let vj_idx = VehicleJourneyIdx::New(new_vj_idx);
        let stop_times: Vec<models::StopTime> = Vec::new();
        let dates = std::iter::once(date);
        let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
        let flows = stop_times.iter().map(|stop_time| stop_time.flow_direction);
        let board_times = stop_times.iter().map(|stop_time| stop_time.board_time);
        let debark_times = stop_times.iter().map(|stop_time| stop_time.debark_time);
        let insert_result = data.insert_real_time_vehicle(
            stops,
            flows,
            board_times,
            debark_times,
            base_model.loads_data(),
            dates,
            UTC,
            vj_idx,
        );
        match insert_result {
            Err(InsertionError::InvalidDate(_, _)) => {
                assert!(true)
            }
            _ => {
                assert!(
                    false,
                    "Expected Err(InvalidDate), found {:?}",
                    insert_result
                );
            }
        }
    }

    // insert a vehicle without date
    {
        let vehicle_journey_id = "no_dates_vj".to_string();

        let new_vj_idx = real_time_model.insert_new_vehicle_journey(&vehicle_journey_id);
        let vj_idx = VehicleJourneyIdx::New(new_vj_idx);
        let stop_times: Vec<models::StopTime> = Vec::new();
        let dates = std::iter::empty();
        let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
        let flows = stop_times.iter().map(|stop_time| stop_time.flow_direction);
        let board_times = stop_times.iter().map(|stop_time| stop_time.board_time);
        let debark_times = stop_times.iter().map(|stop_time| stop_time.debark_time);
        let insert_result = data.insert_real_time_vehicle(
            stops,
            flows,
            board_times,
            debark_times,
            base_model.loads_data(),
            dates,
            UTC,
            vj_idx,
        );
        match insert_result {
            Err(InsertionError::NoValidDates(_)) => {
                assert!(true)
            }
            _ => {
                assert!(
                    false,
                    "Expected Err(NoValidDates), found {:?}",
                    insert_result
                );
            }
        }
    }

    // insert a vehicle that time-travels
    {
        let vehicle_journey_id = "time_travel_vj".to_string();
        let date = "2020-01-01".as_date();
        let stop_times = StopTimesBuilder::new()
            .st("A", "09:45:00")
            .st("B", "08:05:00")
            .st("C", "10:10:00")
            .finalize(&mut real_time_model, &base_model);

        let new_vj_idx = real_time_model.insert_new_vehicle_journey(&vehicle_journey_id);
        let vj_idx = VehicleJourneyIdx::New(new_vj_idx);

        let dates = std::iter::once(date);
        let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
        let flows = stop_times.iter().map(|stop_time| stop_time.flow_direction);
        let board_times = stop_times.iter().map(|stop_time| stop_time.board_time);
        let debark_times = stop_times.iter().map(|stop_time| stop_time.debark_time);
        let insert_result = data.insert_real_time_vehicle(
            stops,
            flows,
            board_times,
            debark_times,
            base_model.loads_data(),
            dates,
            UTC,
            vj_idx,
        );
        match insert_result {
            Err(InsertionError::Times(_, _, _, _)) => {
                assert!(true)
            }
            _ => {
                assert!(false, "Expected Err(Times), found {:?}", insert_result);
            }
        }
    }

    // insert a vehicle that already exists in base schedule
    {
        let vj_idx = VehicleJourneyIdx::Base(base_model.vehicle_journey_idx("first").unwrap());
        let stop_times: Vec<StopTime> = Vec::new();
        let dates = std::iter::once("2020-01-01".as_date());
        let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
        let flows = stop_times.iter().map(|stop_time| stop_time.flow_direction);
        let board_times = stop_times.iter().map(|stop_time| stop_time.board_time);
        let debark_times = stop_times.iter().map(|stop_time| stop_time.debark_time);
        let insert_result = data.insert_real_time_vehicle(
            stops,
            flows,
            board_times,
            debark_times,
            base_model.loads_data(),
            dates,
            UTC,
            vj_idx,
        );
        match insert_result {
            Err(InsertionError::RealTimeVehicleJourneyAlreadyExistsOnDate(_, _)) => {
                assert!(true)
            }
            _ => {
                assert!(
                    false,
                    "Expected Err(RealTimeVehicleJourneyAlreadyExistsOnDate), found {:?}",
                    insert_result
                );
            }
        }
    }

    // insert a new vehicle twice
    {
        let vehicle_journey_id = "inserted_twice_vj".to_string();
        let date = "2020-01-01".as_date();
        let stop_times = StopTimesBuilder::new()
            .st("A", "09:45:00")
            .st("B", "10:05:00")
            .st("C", "10:10:00")
            .finalize(&mut real_time_model, &base_model);

        let new_vj_idx = real_time_model.insert_new_vehicle_journey(&vehicle_journey_id);
        let vj_idx = VehicleJourneyIdx::New(new_vj_idx);

        let dates = std::iter::once(date);
        let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
        let flows = stop_times.iter().map(|stop_time| stop_time.flow_direction);
        let board_times = stop_times.iter().map(|stop_time| stop_time.board_time);
        let debark_times = stop_times.iter().map(|stop_time| stop_time.debark_time);
        let insert_result = data.insert_real_time_vehicle(
            stops,
            flows,
            board_times,
            debark_times,
            base_model.loads_data(),
            dates,
            UTC,
            vj_idx.clone(),
        );

        assert!(insert_result.is_ok());

        let dates = std::iter::once(date);
        let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
        let flows = stop_times.iter().map(|stop_time| stop_time.flow_direction);
        let board_times = stop_times.iter().map(|stop_time| stop_time.board_time);
        let debark_times = stop_times.iter().map(|stop_time| stop_time.debark_time);
        let insert_result = data.insert_real_time_vehicle(
            stops,
            flows,
            board_times,
            debark_times,
            base_model.loads_data(),
            dates,
            UTC,
            vj_idx,
        );

        match insert_result {
            Err(InsertionError::RealTimeVehicleJourneyAlreadyExistsOnDate(_, _)) => {
                assert!(true)
            }
            _ => {
                assert!(
                    false,
                    "Expected Err(RealTimeVehicleJourneyAlreadyExistsOnDate), found {:?}",
                    insert_result
                );
            }
        }
    }

    Ok(())
}
