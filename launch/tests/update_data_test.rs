// Copyright  (C) 2021, Kisio Digital and/or its affiliates. All rights reserved.
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

use anyhow::Error;
use launch::{config::DataImplem, solver::Solver};

use loki::{
    chrono_tz::UTC,
    models::{
        base_model::BaseModel,
        real_time_model::{DisruptionIdx, RealTimeModel},
        ModelRefs, StopTime, VehicleJourneyIdx,
    },
    request::generic_request,
    timetables::{InsertionError, Timetables, TimetablesIter},
    DailyData, DataTrait, DataUpdate, PeriodicData, PeriodicSplitVjData, RealTimeLevel,
};
use utils::{
    disruption_builder::StopTimesBuilder,
    model_builder::{AsDate, ModelBuilder},
    Config,
};

use loki::models::real_time_model::ImpactIdx;
use rstest::rstest;

#[rstest]
#[case(DataImplem::Periodic)]
#[case(DataImplem::Daily)]
#[case(DataImplem::PeriodicSplitVj)]
fn remove_vj(#[case] data_implem: DataImplem) -> Result<(), Error> {
    match data_implem {
        DataImplem::Periodic => remove_vj_inner::<PeriodicData>(),
        DataImplem::PeriodicSplitVj => remove_vj_inner::<PeriodicSplitVjData>(),
        DataImplem::Daily => remove_vj_inner::<DailyData>(),
    }
}

fn remove_vj_inner<T>() -> Result<(), Error>
where
    T: Timetables<
        Mission = generic_request::Mission,
        Position = generic_request::Position,
        Trip = generic_request::Trip,
    >,
    T: for<'a> TimetablesIter<'a>,
    T::Mission: 'static,
    T::Position: 'static,
{
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

    let mut data = launch::read::build_transit_data::<T>(&base_model);

    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    {
        let request_input = utils::make_request_from_config(&config)?;
        let responses = solver.solve_request(
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

    data.remove_real_time_vehicle(&vj_idx, &"2020-01-01".as_date())
        .unwrap();

    {
        let mut request_input = utils::make_request_from_config(&config)?;
        request_input.real_time_level = RealTimeLevel::RealTime;
        let responses = solver.solve_request(
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
        let request_input = utils::make_request_from_config(&config)?;
        let responses = solver.solve_request(
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

#[rstest]
#[case(DataImplem::Periodic)]
#[case(DataImplem::Daily)]
#[case(DataImplem::PeriodicSplitVj)]
fn remove_successive_vj(#[case] data_implem: DataImplem) -> Result<(), Error> {
    match data_implem {
        DataImplem::Periodic => remove_successive_vj_inner::<PeriodicData>(),
        DataImplem::PeriodicSplitVj => remove_successive_vj_inner::<PeriodicSplitVjData>(),
        DataImplem::Daily => remove_successive_vj_inner::<DailyData>(),
    }
}

fn remove_successive_vj_inner<T>() -> Result<(), Error>
where
    T: Timetables<
        Mission = generic_request::Mission,
        Position = generic_request::Position,
        Trip = generic_request::Trip,
    >,
    T: for<'a> TimetablesIter<'a>,
    T::Mission: 'static,
    T::Position: 'static,
{
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

    let mut data = launch::read::build_transit_data::<T>(&base_model);

    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    {
        let request_input = utils::make_request_from_config(&config)?;
        let responses = solver.solve_request(
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
        data.remove_real_time_vehicle(&vj_idx, &"2020-01-01".as_date())
            .unwrap();
    }

    {
        let mut request_input = utils::make_request_from_config(&config)?;
        request_input.real_time_level = RealTimeLevel::RealTime;
        let responses = solver.solve_request(
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
        data.remove_real_time_vehicle(&vj_idx, &"2020-01-01".as_date())
            .unwrap();
    }

    {
        let mut request_input = utils::make_request_from_config(&config)?;
        request_input.real_time_level = RealTimeLevel::RealTime;
        let responses = solver.solve_request(
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
        data.remove_real_time_vehicle(&vj_idx, &"2020-01-01".as_date())
            .unwrap();
    }

    {
        let mut request_input = utils::make_request_from_config(&config)?;
        request_input.real_time_level = RealTimeLevel::RealTime;
        let responses = solver.solve_request(
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

#[rstest]
#[case(DataImplem::Periodic)]
#[case(DataImplem::Daily)]
#[case(DataImplem::PeriodicSplitVj)]
fn remove_middle_vj(#[case] data_implem: DataImplem) -> Result<(), Error> {
    match data_implem {
        DataImplem::Periodic => remove_middle_vj_inner::<PeriodicData>(),
        DataImplem::PeriodicSplitVj => remove_middle_vj_inner::<PeriodicSplitVjData>(),
        DataImplem::Daily => remove_middle_vj_inner::<DailyData>(),
    }
}

fn remove_middle_vj_inner<T>() -> Result<(), Error>
where
    T: Timetables<
        Mission = generic_request::Mission,
        Position = generic_request::Position,
        Trip = generic_request::Trip,
    >,
    T: for<'a> TimetablesIter<'a>,
    T::Mission: 'static,
    T::Position: 'static,
{
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

    let mut data = launch::read::build_transit_data::<T>(&base_model);

    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    {
        let request_input = utils::make_request_from_config(&config)?;
        let responses = solver.solve_request(
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
        data.remove_real_time_vehicle(&vj_idx, &"2020-01-01".as_date())
            .unwrap();
    }

    {
        let request_input = utils::make_request_from_config(&config)?;
        let responses = solver.solve_request(
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
        data.remove_real_time_vehicle(&vj_idx, &"2020-01-01".as_date())
            .unwrap();
    }

    {
        let request_input = utils::make_request_from_config(&config)?;
        let responses = solver.solve_request(
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

#[rstest]
#[case(DataImplem::Periodic)]
#[case(DataImplem::Daily)]
#[case(DataImplem::PeriodicSplitVj)]
fn modify_vj(#[case] data_implem: DataImplem) -> Result<(), Error> {
    match data_implem {
        DataImplem::Periodic => modify_vj_inner::<PeriodicData>(),
        DataImplem::PeriodicSplitVj => modify_vj_inner::<PeriodicSplitVjData>(),
        DataImplem::Daily => modify_vj_inner::<DailyData>(),
    }
}

fn modify_vj_inner<T>() -> Result<(), Error>
where
    T: Timetables<
        Mission = generic_request::Mission,
        Position = generic_request::Position,
        Trip = generic_request::Trip,
    >,
    T: for<'a> TimetablesIter<'a>,
    T::Mission: 'static,
    T::Position: 'static,
{
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
    let request_input = utils::make_request_from_config(&config)?;

    let mut data = launch::read::build_transit_data::<T>(&base_model);

    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    {
        let model_refs = ModelRefs::new(&base_model, &real_time_model);
        let responses = solver.solve_request(
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
        let disruption_idx = DisruptionIdx::new(0);
        let impact_idx = ImpactIdx::new(0);
        let result = real_time_model.modify_trip(
            &base_model,
            &mut data,
            "first",
            &date,
            stop_times,
            disruption_idx,
            impact_idx,
        );
        assert!(result.is_ok());
    }

    {
        let mut request_input = request_input.clone();
        request_input.real_time_level = RealTimeLevel::RealTime;

        let model_refs = ModelRefs::new(&base_model, &real_time_model);

        let responses = solver.solve_request(
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

        let responses = solver.solve_request(
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

#[rstest]
#[case(DataImplem::Periodic)]
#[case(DataImplem::Daily)]
#[case(DataImplem::PeriodicSplitVj)]
fn insert_invalid_vj(#[case] data_implem: DataImplem) -> Result<(), Error> {
    match data_implem {
        DataImplem::Periodic => insert_invalid_vj_inner::<PeriodicData>(),
        DataImplem::PeriodicSplitVj => insert_invalid_vj_inner::<PeriodicSplitVjData>(),
        DataImplem::Daily => insert_invalid_vj_inner::<DailyData>(),
    }
}

fn insert_invalid_vj_inner<T>() -> Result<(), Error>
where
    T: Timetables<
        Mission = generic_request::Mission,
        Position = generic_request::Position,
        Trip = generic_request::Trip,
    >,
    T: for<'a> TimetablesIter<'a>,
    T::Mission: 'static,
    T::Position: 'static,
{
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

    let mut data = launch::read::build_transit_data::<T>(&base_model);

    // insert a vehicle with a date outside of the calendar of the data
    {
        let vehicle_journey_id = "invalid_date_vj".to_string();
        let date = "1999-01-01".as_date();
        let disruption_idx = DisruptionIdx::new(0);
        let impact_idx = ImpactIdx::new(0);
        let (vj_idx, stop_times) = real_time_model
            .add(
                disruption_idx,
                impact_idx,
                &vehicle_journey_id,
                &date,
                Vec::new(),
                &base_model,
            )
            .unwrap();
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
            &UTC,
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
        let date = "1999-01-01".as_date();

        let disruption_idx = DisruptionIdx::new(0);
        let impact_idx = ImpactIdx::new(0);
        let (vj_idx, stop_times) = real_time_model
            .add(
                disruption_idx,
                impact_idx,
                &vehicle_journey_id,
                &date,
                Vec::new(),
                &base_model,
            )
            .unwrap();
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
            &UTC,
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
        let disruption_idx = DisruptionIdx::new(0);
        let impact_idx = ImpactIdx::new(0);
        let stop_times = StopTimesBuilder::new()
            .st("A", "09:45:00")
            .st("B", "08:05:00")
            .st("C", "10:10:00")
            .finalize(&mut real_time_model, &base_model);

        let (vj_idx, stop_times) = real_time_model
            .add(
                disruption_idx,
                impact_idx,
                &vehicle_journey_id,
                &date,
                stop_times,
                &base_model,
            )
            .unwrap();
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
            &UTC,
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
            &UTC,
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
        let disruption_idx = DisruptionIdx::new(0);
        let impact_idx = ImpactIdx::new(0);
        let stop_times = StopTimesBuilder::new()
            .st("A", "09:45:00")
            .st("B", "10:05:00")
            .st("C", "10:10:00")
            .finalize(&mut real_time_model, &base_model);

        let (vj_idx, stop_times) = real_time_model
            .add(
                disruption_idx,
                impact_idx,
                &vehicle_journey_id,
                &date,
                stop_times,
                &base_model,
            )
            .unwrap();
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
            &UTC,
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
            &UTC,
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
