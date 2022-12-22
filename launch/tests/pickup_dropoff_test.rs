// Copyright  (C) 2020, Hove and/or its affiliates. All rights reserved.
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
use loki::{models::base_model::BaseModel, PositiveDuration};
use loki_launch::{
    config::ComparatorType,
    loki::models::{real_time_model::RealTimeModel, ModelRefs},
};
use rstest::rstest;
use utils::{build_and_solve, model_builder::ModelBuilder, Config};

#[rstest]
#[case(ComparatorType::Loads)]
#[case(ComparatorType::Basic)]
fn test_forbidden_pickup(#[case] comparator_type: ComparatorType) -> Result<(), Error> {
    let _log_guard = loki_launch::logger::init_test_logger();

    let model = ModelBuilder::new("2020-01-01", "2020-01-02")
        .vj("toto", |vj_builder| {
            vj_builder
                .route("1")
                .st_detailed("A", "10:00:00", "10:00:00", 1, 0, None) // pickup = 1 means pickup forbidden
                .st("B", "10:05:00")
                .st("C", "10:10:00");
        })
        .build();

    let base_model = BaseModel::from_transit_model(
        model,
        loki::OccupancyData::empty(),
        PositiveDuration::zero(),
    )
    .unwrap();

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&base_model, &real_time_model);

    let config = Config::new("2020-01-01T09:59:00", "A", "B");
    let config = Config {
        comparator_type,
        ..config
    };

    let responses = build_and_solve(&model_refs, &config)?;

    assert_eq!(base_model.nb_of_vehicle_journeys(), 1);
    assert_eq!(responses.len(), 0);

    Ok(())
}

#[rstest]
#[case(ComparatorType::Loads)]
#[case(ComparatorType::Basic)]
fn test_forbidden_dropoff(#[case] comparator_type: ComparatorType) -> Result<(), Error> {
    let _log_guard = loki_launch::logger::init_test_logger();

    let model = ModelBuilder::new("2020-01-01", "2020-01-02")
        .vj("toto", |vj_builder| {
            vj_builder
                .route("1")
                .st("A", "10:00:00")
                .st_detailed("B", "10:05:00", "10:05:00", 0, 1, None) // dropoff = 1 means pickup forbidden
                .st("C", "10:10:00");
        })
        .build();

    let base_model = BaseModel::from_transit_model(
        model,
        loki::OccupancyData::empty(),
        PositiveDuration::zero(),
    )
    .unwrap();

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&base_model, &real_time_model);

    let config = Config::new("2020-01-01T09:59:00", "A", "B");
    let config = Config {
        comparator_type,
        ..config
    };

    let responses = build_and_solve(&model_refs, &config)?;

    assert_eq!(base_model.nb_of_vehicle_journeys(), 1);
    assert_eq!(responses.len(), 0);

    Ok(())
}

#[rstest]
#[case(ComparatorType::Loads)]
#[case(ComparatorType::Basic)]
fn test_skipped_stop(#[case] comparator_type: ComparatorType) -> Result<(), Error> {
    let _log_guard = loki_launch::logger::init_test_logger();

    let model = ModelBuilder::new("2020-01-01", "2020-01-02")
        .vj("toto", |vj_builder| {
            vj_builder
                .route("1")
                .st("A", "10:00:00")
                .st_detailed("B", "10:05:00", "10:05:00", 3, 3, None) // pickup = dropoff = 3 means skipped stop
                .st("C", "10:10:00");
        })
        .build();

    let base_model = BaseModel::from_transit_model(
        model,
        loki::OccupancyData::empty(),
        PositiveDuration::zero(),
    )
    .unwrap();

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&base_model, &real_time_model);

    assert_eq!(base_model.nb_of_vehicle_journeys(), 1);

    // we can go from A to C
    {
        let config = Config::new("2020-01-01T09:59:00", "A", "C");
        let config = Config {
            comparator_type,
            ..config
        };
        let responses = build_and_solve(&model_refs, &config)?;
        assert_eq!(responses.len(), 1);
    }

    // but not from A to B
    {
        let config = Config::new("2020-01-01T09:59:00", "A", "B");
        let config = Config {
            comparator_type,
            ..config
        };
        let responses = build_and_solve(&model_refs, &config)?;
        assert_eq!(responses.len(), 0);
    }

    // and not from B to C
    {
        let config = Config::new("2020-01-01T09:59:00", "B", "C");
        let config = Config {
            comparator_type,
            ..config
        };
        let responses = build_and_solve(&model_refs, &config)?;
        assert_eq!(responses.len(), 0);
    }

    Ok(())
}
