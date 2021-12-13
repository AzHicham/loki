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
    models::{base_model::BaseModel, real_time_model::RealTimeModel, ModelRefs, VehicleJourneyIdx},
    request::generic_request,
    timetables::{Timetables, TimetablesIter},
    DailyData, DataTrait, DataUpdate, PeriodicData, PeriodicSplitVjData, RealTimeLevel,
};
use utils::{
    disruption_builder::{modify, StopTimesBuilder},
    model_builder::{AsDate, ModelBuilder},
    Config,
};

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

    let base_model = BaseModel::from_transit_model(model, loki::LoadsData::empty());

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&base_model, &real_time_model);

    let mut data =
        launch::read::build_transit_data::<T>(&base_model, &config.default_transfer_duration);

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

    let vehicle_journey_idx = base_model.vehicle_journeys.get_idx("first").unwrap();
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

    let base_model = BaseModel::from_transit_model(model, loki::LoadsData::empty());

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&base_model, &real_time_model);

    let config = Config::new("2020-01-01T08:00:00", "A", "C");

    let mut data =
        launch::read::build_transit_data::<T>(&base_model, &config.default_transfer_duration);

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
        let vehicle_journey_idx = base_model.vehicle_journeys.get_idx("first").unwrap();
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
        let vehicle_journey_idx = base_model.vehicle_journeys.get_idx("second").unwrap();
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
        let vehicle_journey_idx = base_model.vehicle_journeys.get_idx("third").unwrap();
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

    let base_model = BaseModel::from_transit_model(model, loki::LoadsData::empty());

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&base_model, &real_time_model);

    let config = Config::new("2020-01-01T10:50:00", "A", "C");

    let mut data =
        launch::read::build_transit_data::<T>(&base_model, &config.default_transfer_duration);

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
        let vehicle_journey_idx = base_model.vehicle_journeys.get_idx("first").unwrap();
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
        let vehicle_journey_idx = base_model.vehicle_journeys.get_idx("third").unwrap();
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

    let base_model = BaseModel::from_transit_model(model, loki::LoadsData::empty());

    let mut real_time_model = RealTimeModel::new();

    let config = Config::new("2020-01-01T09:50:00", "A", "C");
    let request_input = utils::make_request_from_config(&config)?;

    let mut data =
        launch::read::build_transit_data::<T>(&base_model, &config.default_transfer_duration);

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
        let disruption = {
            let stop_times = StopTimesBuilder::new()
                .st("A", "09:45:00")
                .st("B", "10:05:00")
                .st("C", "10:10:00");
            modify("first", "2020-01-01", stop_times)
        };
        real_time_model.apply_disruption(&disruption, &base_model, &mut data);
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
