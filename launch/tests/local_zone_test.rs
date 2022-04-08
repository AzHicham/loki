// Copyright  (C) 2022, Hove and/or its affiliates. All rights reserved.
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
use launch::loki::models::{real_time_model::RealTimeModel, ModelRefs};
use loki::{models::base_model::BaseModel, DataTrait, PositiveDuration};

use utils::{build_and_solve, model_builder::ModelBuilder, Config};

#[test]
fn test_local_zone_routing() -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    let model = ModelBuilder::new("2022-01-01", "2022-01-02")
        .vj("LocalZone", |vj_builder| {
            vj_builder
                .route("1")
                .st_detailed("A", "10:00:00", "10:00:00", 0u8, 0u8, None)
                .st_detailed("B", "10:05:00", "10:05:00", 0u8, 0u8, None)
                .st_detailed("C", "10:10:00", "10:10:00", 0u8, 0u8, Some(1u16))
                .st_detailed("D", "10:15:00", "10:15:00", 0u8, 0u8, Some(1u16))
                .st_detailed("E", "10:20:00", "10:20:00", 0u8, 0u8, Some(2u16));
        })
        .build();

    let base_model =
        BaseModel::from_transit_model(model, loki::LoadsData::empty(), PositiveDuration::zero())
            .unwrap();

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&base_model, &real_time_model);

    let config = Config::new("2022-01-01T09:59:00", "A", "B");

    assert_eq!(base_model.nb_of_vehicle_journeys(), 1);

    let responses = build_and_solve(&model_refs, &config)?;

    // The stops "A" and "B" are in the same local zone
    // so we cannot go from "A" to "B" with the vehicle_journey "LocalZone"
    // thus we should find no journey for this request
    assert_eq!(responses.len(), 0);

    let config = Config::new("2022-01-01T09:59:00", "A", "C");

    let responses = build_and_solve(&model_refs, &config)?;

    // The stops "A" and "C" are not in the same zone
    // so we should get a journey in the response
    assert_eq!(responses.len(), 1);

    let journey = &responses[0];
    assert_eq!(journey.nb_of_sections(), 1);

    Ok(())
}

#[test]
fn test_local_zone_timetable() -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    let model = ModelBuilder::new("2022-01-01", "2022-01-02")
        .vj("LocalZone", |vj_builder| {
            vj_builder
                .route("1")
                .st_detailed("A", "10:00:00", "10:00:00", 0u8, 0u8, None)
                .st_detailed("B", "10:05:00", "10:05:00", 0u8, 0u8, None)
                .st_detailed("C", "10:10:00", "10:10:00", 0u8, 0u8, Some(1u16))
                .st_detailed("D", "10:15:00", "10:15:00", 0u8, 0u8, Some(1u16))
                .st_detailed("E", "10:20:00", "10:20:00", 0u8, 0u8, Some(2u16));
        })
        .build();

    let base_model =
        BaseModel::from_transit_model(model, loki::LoadsData::empty(), PositiveDuration::zero())
            .unwrap();

    assert_eq!(base_model.nb_of_vehicle_journeys(), 1);

    // Since there are 3 different local zone, 3 missions must be created
    let expected_mission_nb = 3;
    let data = launch::read::build_transit_data(&base_model);
    assert_eq!(data.nb_of_missions(), expected_mission_nb);

    Ok(())
}

#[test]
fn test_local_zone_routing_multiple_vj() -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    let model = ModelBuilder::new("2022-01-01", "2022-01-02")
        .vj("LocalZone", |vj_builder| {
            vj_builder
                .route("1")
                .st_detailed("A", "10:00:00", "10:00:00", 0u8, 0u8, None)
                .st_detailed("B", "10:04:00", "10:04:00", 0u8, 0u8, None)
                .st_detailed("C", "10:09:00", "10:09:00", 0u8, 0u8, Some(1u16))
                .st_detailed("D", "10:14:00", "10:14:00", 0u8, 0u8, Some(1u16));
        })
        .vj("NoLocalZone", |vj_builder| {
            vj_builder
                .route("1")
                .st_detailed("A", "10:00:00", "10:00:00", 0u8, 0u8, None)
                .st_detailed("B", "10:05:00", "10:05:00", 0u8, 0u8, None)
                .st_detailed("C", "10:10:00", "10:10:00", 0u8, 0u8, None)
                .st_detailed("D", "10:15:00", "10:15:00", 0u8, 0u8, None);
        })
        .build();

    let base_model =
        BaseModel::from_transit_model(model, loki::LoadsData::empty(), PositiveDuration::zero())
            .unwrap();

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&base_model, &real_time_model);

    // We should be able to go from A to B with the vehicle NoLocalZone
    {
        let config = Config::new("2022-01-01T09:59:00", "A", "B");

        assert_eq!(base_model.nb_of_vehicle_journeys(), 2);

        let responses = build_and_solve(&model_refs, &config)?;

        let journey = &responses[0];
        assert_eq!(responses.len(), 1);
        assert_eq!(journey.first_vj_uri(&model_refs), "NoLocalZone");
    }

    // Between A and C we can use both LocalZone and NoLocalZone
    // but LocalZone arrives earlier, so we should use it
    {
        let config = Config::new("2022-01-01T09:59:00", "A", "C");

        let responses = build_and_solve(&model_refs, &config)?;

        assert_eq!(responses.len(), 1);

        let journey = &responses[0];
        assert_eq!(journey.nb_of_sections(), 1);
        let journey = &responses[0];
        assert_eq!(responses.len(), 1);
        assert_eq!(journey.first_vj_uri(&model_refs), "LocalZone");
    }

    Ok(())
}

#[test]
fn test_local_zone_routing_one_local_zone() -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    let model = ModelBuilder::new("2022-01-01", "2022-01-02")
        .vj("LocalZone", |vj_builder| {
            vj_builder
                .route("1")
                .st_detailed("A", "10:00:00", "10:00:00", 0u8, 0u8, Some(1u16))
                .st_detailed("B", "10:05:00", "10:05:00", 0u8, 0u8, Some(1u16))
                .st_detailed("C", "10:10:00", "10:10:00", 0u8, 0u8, Some(1u16))
                .st_detailed("D", "10:15:00", "10:15:00", 0u8, 0u8, Some(1u16))
                .st_detailed("E", "10:20:00", "10:20:00", 0u8, 0u8, Some(1u16));
        })
        .build();

    let base_model =
        BaseModel::from_transit_model(model, loki::LoadsData::empty(), PositiveDuration::zero())
            .unwrap();

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&base_model, &real_time_model);

    assert_eq!(base_model.nb_of_vehicle_journeys(), 1);

    let config = Config::new("2022-01-01T09:59:00", "A", "E");

    let responses = build_and_solve(&model_refs, &config)?;
    assert_eq!(responses.len(), 1);

    Ok(())
}
