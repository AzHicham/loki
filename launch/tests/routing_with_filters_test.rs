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
use anyhow::Error;
use launch::config::{ComparatorType, DataImplem};

use loki::transit_model::objects::Availability::Available;
use loki::{
    models::{base_model::BaseModel, real_time_model::RealTimeModel, ModelRefs},
    PositiveDuration, RealTimeLevel,
};
use rstest::{fixture, rstest};
use utils::{build_and_solve, from_to_stop_point_names, model_builder::ModelBuilder, Config};

#[fixture]
pub fn fixture_model() -> BaseModel {
    let model = ModelBuilder::new("2020-01-01", "2020-01-02")
        .equipment("EQW", |e| e.wheelchair_boarding = Available)
        .equipment("EQWB", |e| {
            e.wheelchair_boarding = Available;
            e.bike_accepted = Available
        })
        .network("N1", |n| n.name = "N1".into())
        .route("R1", |r| r.name = "R1".into())
        .route("R2", |r| r.name = "R2".into())
        .route("R3", |r| r.name = "R3".into())
        .route("R4", |r| r.name = "R3".into())
        .stop_area("sa:A", |_| {})
        .stop_area("sa:B", |_| {})
        .stop_area("sa:C", |_| {})
        .stop_area("sa:F", |_| {})
        .stop_area("sa:G", |_| {})
        .stop_point("A", |sp| {
            sp.equipment_id = Some("EQWB".to_string());
            sp.stop_area_id = "sa:A".to_string()
        })
        .stop_point("B", |sp| {
            sp.equipment_id = Some("EQW".to_string());
            sp.stop_area_id = "sa:B".to_string()
        })
        .stop_point("C", |sp| {
            sp.equipment_id = Some("EQWB".to_string());
            sp.stop_area_id = "sa:C".to_string()
        })
        .stop_point("F", |sp| {
            sp.equipment_id = Some("EQWB".to_string());
            sp.stop_area_id = "sa:F".to_string()
        })
        .stop_point("G", |sp| {
            sp.equipment_id = Some("EQW".to_string());
            sp.stop_area_id = "sa:G".to_string()
        })
        .vj("toto", |vj_builder| {
            vj_builder
                .route("R1")
                .st("A", "10:00:00")
                .st("B", "10:05:00")
                .st("C", "10:10:00")
                .add_property("wheelchair_accessible", "1");
        })
        .vj("toto_bike", |vj_builder| {
            vj_builder
                .route("R1")
                .st("A", "11:00:00")
                .st("B", "11:05:00")
                .st("C", "11:10:00")
                .add_property("bike_accepted", "1");
        })
        .vj("tata", |vj_builder| {
            vj_builder
                .route("R2")
                .st("E", "10:05:00")
                .st("F", "10:20:00")
                .st("G", "10:30:00")
                .add_property("wheelchair_accessible", "1")
                .add_property("bike_accepted", "1");
        })
        .vj("titi", |vj_builder| {
            vj_builder
                .route("R3")
                .st("H", "10:35:00")
                .st("I", "10:40:00")
                .st("J", "10:45:00");
        })
        .vj("txtx", |vj_builder| {
            vj_builder
                .route("R4")
                .st("C", "10:35:00")
                .st("J", "10:55:00");
        })
        .add_transfer("B", "F", "00:02:00")
        .add_transfer("G", "H", "00:02:00")
        .add_transfer("C", "I", "00:02:00")
        .add_transfer("C", "C", "00:02:00")
        .build();

    let loads_data = loki::LoadsData::empty();
    BaseModel::from_transit_model(model, loads_data, PositiveDuration::zero()).unwrap()
}

#[rstest]
#[case(ComparatorType::Loads, DataImplem::Periodic)]
#[case(ComparatorType::Basic, DataImplem::Periodic)]
#[case(ComparatorType::Loads, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::PeriodicSplitVj)]
fn test_no_filter(
    #[case] comparator_type: ComparatorType,
    #[case] data_implem: DataImplem,
    fixture_model: BaseModel,
) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    let config = Config::new("2020-01-01T09:59:00", "A", "J");
    let config = Config {
        comparator_type,
        data_implem,
        ..config
    };
    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&fixture_model, &real_time_model);

    let responses = build_and_solve(&model_refs, &config)?;

    assert_eq!(responses.len(), 1);

    let journey = &responses[0];
    assert_eq!(journey.nb_of_sections(), 4);
    assert_eq!(journey.connections.len(), 1);

    // First Vehicle
    let vehicle_sec = &journey.first_vehicle;
    assert_eq!(journey.first_vj_uri(&model_refs), "toto");
    let (from_sp, to_sp) =
        from_to_stop_point_names(vehicle_sec, &model_refs, &RealTimeLevel::Base)?;
    assert_eq!(from_sp, "A");
    assert_eq!(to_sp, "C");

    Ok(())
}

#[rstest]
#[case(ComparatorType::Loads, DataImplem::Periodic)]
#[case(ComparatorType::Basic, DataImplem::Periodic)]
#[case(ComparatorType::Loads, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::PeriodicSplitVj)]
fn test_filter_forbidden_stop_point(
    #[case] comparator_type: ComparatorType,
    #[case] data_implem: DataImplem,
    fixture_model: BaseModel,
) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // With Filter : stop_point:C is forbidden
    let config = Config::new("2020-01-01T09:59:00", "A", "J");
    let config = Config {
        comparator_type,
        data_implem,
        forbidden_uri: vec!["stop_point:C"],
        ..config
    };

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&fixture_model, &real_time_model);

    let responses = build_and_solve(&model_refs, &config)?;

    assert_eq!(responses.len(), 1);

    let journey = &responses[0];
    assert_eq!(journey.nb_of_sections(), 7);
    assert_eq!(journey.connections.len(), 2);

    // First Vehicle
    let vehicle_sec = &journey.first_vehicle;
    assert_eq!(journey.first_vj_uri(&model_refs), "toto");
    let (from_sp, to_sp) =
        from_to_stop_point_names(vehicle_sec, &model_refs, &RealTimeLevel::Base)?;
    assert_eq!(from_sp, "A");
    assert_eq!(to_sp, "B");

    Ok(())
}

#[rstest]
#[case(ComparatorType::Loads, DataImplem::Periodic)]
#[case(ComparatorType::Basic, DataImplem::Periodic)]
#[case(ComparatorType::Loads, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::PeriodicSplitVj)]
fn test_filter_allowed_stop_point(
    #[case] comparator_type: ComparatorType,
    #[case] data_implem: DataImplem,
    fixture_model: BaseModel,
) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // With Filter : stop_point:C is forbidden
    let config = Config::new("2020-01-01T09:59:00", "A", "J");
    let config = Config {
        comparator_type,
        data_implem,
        allowed_uri: vec![
            "stop_point:A",
            "stop_point:B",
            "stop_point:F",
            "stop_point:G",
            "stop_point:H",
            "stop_point:J",
        ],
        ..config
    };

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&fixture_model, &real_time_model);

    let responses = build_and_solve(&model_refs, &config)?;

    assert_eq!(responses.len(), 1);

    let journey = &responses[0];
    assert_eq!(journey.nb_of_sections(), 7);
    assert_eq!(journey.connections.len(), 2);

    // First Vehicle
    let vehicle_sec = &journey.first_vehicle;
    assert_eq!(journey.first_vj_uri(&model_refs), "toto");
    let (from_sp, to_sp) =
        from_to_stop_point_names(vehicle_sec, &model_refs, &RealTimeLevel::Base)?;
    assert_eq!(from_sp, "A");
    assert_eq!(to_sp, "B");

    Ok(())
}

#[rstest]
#[case(ComparatorType::Loads, DataImplem::Periodic)]
#[case(ComparatorType::Basic, DataImplem::Periodic)]
#[case(ComparatorType::Loads, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::PeriodicSplitVj)]
fn test_filter_forbidden_route(
    #[case] comparator_type: ComparatorType,
    #[case] data_implem: DataImplem,
    fixture_model: BaseModel,
) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // With Filter : stop_point:C is forbidden
    let config = Config::new("2020-01-01T09:59:00", "A", "J");
    let config = Config {
        comparator_type,
        data_implem,
        forbidden_uri: vec!["route:R2", "route:R3"],
        ..config
    };
    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&fixture_model, &real_time_model);

    let responses = build_and_solve(&model_refs, &config)?;

    assert_eq!(responses.len(), 1);

    let journey = &responses[0];
    assert_eq!(journey.nb_of_sections(), 4);
    assert_eq!(journey.connections.len(), 1);

    // First Vehicle
    let vehicle_sec = &journey.first_vehicle;
    assert_eq!(journey.first_vj_uri(&model_refs), "toto");
    let (from_sp, to_sp) =
        from_to_stop_point_names(vehicle_sec, &model_refs, &RealTimeLevel::Base)?;
    assert_eq!(from_sp, "A");
    assert_eq!(to_sp, "C");

    Ok(())
}

#[rstest]
#[case(ComparatorType::Loads, DataImplem::Periodic)]
#[case(ComparatorType::Basic, DataImplem::Periodic)]
#[case(ComparatorType::Loads, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::PeriodicSplitVj)]
fn test_filter_allowed_route(
    #[case] comparator_type: ComparatorType,
    #[case] data_implem: DataImplem,
    fixture_model: BaseModel,
) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // With Filter : stop_point:C is forbidden
    let config = Config::new("2020-01-01T09:59:00", "A", "J");
    let config = Config {
        comparator_type,
        data_implem,
        allowed_uri: vec!["route:R1", "route:R4"],
        ..config
    };
    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&fixture_model, &real_time_model);

    let responses = build_and_solve(&model_refs, &config)?;

    assert_eq!(responses.len(), 1);

    let journey = &responses[0];
    assert_eq!(journey.nb_of_sections(), 4);
    assert_eq!(journey.connections.len(), 1);

    // First Vehicle
    let vehicle_sec = &journey.first_vehicle;
    assert_eq!(journey.first_vj_uri(&model_refs), "toto");
    let (from_sp, to_sp) =
        from_to_stop_point_names(vehicle_sec, &model_refs, &RealTimeLevel::Base)?;
    assert_eq!(from_sp, "A");
    assert_eq!(to_sp, "C");

    Ok(())
}

#[rstest]
#[case(ComparatorType::Loads, DataImplem::Periodic)]
#[case(ComparatorType::Basic, DataImplem::Periodic)]
#[case(ComparatorType::Loads, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::PeriodicSplitVj)]
fn test_filter_wheelchair_no_solution(
    #[case] comparator_type: ComparatorType,
    #[case] data_implem: DataImplem,
    fixture_model: BaseModel,
) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // With Filter : wheelchair_accessible = true,
    let config = Config::new("2020-01-01T09:59:00", "E", "G");
    let config = Config {
        comparator_type,
        data_implem,
        wheelchair_accessible: true,
        ..config
    };

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&fixture_model, &real_time_model);

    let responses = build_and_solve(&model_refs, &config)?;

    // As E & G stop_point are not marked as wheelchair_accessible
    // we should not have a response
    assert_eq!(responses.len(), 0);

    // Solve with the same config but wheelchair_accssible = false
    // We should have a response
    let config = Config {
        wheelchair_accessible: false,
        ..config
    };
    let responses = build_and_solve(&model_refs, &config)?;
    assert_eq!(responses.len(), 1);

    Ok(())
}

#[rstest]
#[case(ComparatorType::Loads, DataImplem::Periodic)]
#[case(ComparatorType::Basic, DataImplem::Periodic)]
#[case(ComparatorType::Loads, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::PeriodicSplitVj)]
fn test_filter_bike(
    #[case] comparator_type: ComparatorType,
    #[case] data_implem: DataImplem,
    fixture_model: BaseModel,
) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // With Filter : wheelchair_accessible = true,
    let config = Config::new("2020-01-01T09:59:00", "A", "C");
    let config = Config {
        comparator_type,
        data_implem,
        bike_accessible: true,
        ..config
    };

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&fixture_model, &real_time_model);

    let responses = build_and_solve(&model_refs, &config)?;

    // A & C stop_points are marked are bike_accessible
    // but only vj "toto_bike" is bike accessible
    // So even if "toto_bike" arrive later (1h later) than vj "toto", we should take "toto_bike"
    assert_eq!(responses.len(), 1);
    let journey = &responses[0];
    assert_eq!(journey.first_vj_uri(&model_refs), "toto_bike");

    // In the case we don't require bike_accessible
    // we use "toto" vj
    let config = Config {
        bike_accessible: false,
        ..config
    };
    let responses = build_and_solve(&model_refs, &config)?;
    assert_eq!(responses.len(), 1);
    let journey = &responses[0];
    assert_eq!(journey.first_vj_uri(&model_refs), "toto");

    Ok(())
}

#[rstest]
#[case(ComparatorType::Loads, DataImplem::Periodic)]
#[case(ComparatorType::Basic, DataImplem::Periodic)]
#[case(ComparatorType::Loads, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::Daily)]
#[case(ComparatorType::Basic, DataImplem::PeriodicSplitVj)]
fn test_filter_accessibility_with_transfer(
    #[case] comparator_type: ComparatorType,
    #[case] data_implem: DataImplem,
    fixture_model: BaseModel,
) -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // With Filter : wheelchair_accessible = true,
    let config = Config::new("2020-01-01T09:59:00", "A", "G");
    let config = Config {
        comparator_type,
        data_implem,
        wheelchair_accessible: true,
        bike_accessible: false,
        ..config
    };

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&fixture_model, &real_time_model);

    let responses = build_and_solve(&model_refs, &config)?;

    // A & B & F stop_points are marked as bike_accessible & wheelchair_accessible
    // "toto_bike" is bike accessible & "toto" is wheelchair_accessible
    // "tata" is bike accessible &  wheelchair_accessible
    // But stop_point "G" is marked as wheelchair_accessible only
    // We should find a response when we query with wheelchair_accessible = true
    // but not with bike_accessible = true
    assert_eq!(responses.len(), 1);
    let journey = &responses[0];
    assert_eq!(journey.first_vj_uri(&model_refs), "toto");

    let config = Config {
        wheelchair_accessible: false,
        bike_accessible: true,
        ..config
    };
    let responses = build_and_solve(&model_refs, &config)?;
    assert_eq!(responses.len(), 0);

    let config = Config {
        wheelchair_accessible: true,
        bike_accessible: true,
        ..config
    };
    let responses = build_and_solve(&model_refs, &config)?;
    assert_eq!(responses.len(), 0);

    Ok(())
}
