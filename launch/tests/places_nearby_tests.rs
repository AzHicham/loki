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
use loki::{
    models::{base_model::BaseModel, real_time_model::RealTimeModel, ModelRefs},
    places_nearby::{solve_places_nearby_request, BadPlacesNearby},
    transit_model::objects::Coord,
    PositiveDuration,
};
use rstest::{fixture, rstest};
use utils::model_builder::ModelBuilder;

#[fixture]
pub fn fixture_model() -> BaseModel {
    let model = ModelBuilder::new("2020-01-01", "2020-01-02")
        .network("N", |n| n.name = "N".into())
        .route("R", |r| r.name = "R".into())
        .stop_area("SA1", |sa| {
            sa.coord = Coord {
                lon: 2.325581,
                lat: 48.82241,
            }
        })
        .stop_area("SA2", |_| {})
        .stop_point("A", |sp| {
            sp.stop_area_id = "SA1".to_string();
            sp.coord = Coord {
                lon: 2.325624,
                lat: 48.823395,
            }
        })
        .stop_point("B", |sp| {
            sp.stop_area_id = "SA1".to_string();
            sp.coord = Coord {
                lon: 2.32618,
                lat: 48.822944,
            };
        })
        .stop_point("C", |sp| {
            sp.stop_area_id = "SA1".to_string();
            sp.coord = Coord {
                lon: 2.326294,
                lat: 48.82361,
            };
        })
        .stop_point("D", |sp| {
            sp.stop_area_id = "SA1".to_string();
            sp.coord = Coord {
                lon: 2.325804,
                lat: 48.823634,
            };
        })
        .stop_point("E", |sp| {
            sp.stop_area_id = "SA2".to_string();
            sp.coord = Coord {
                lon: 2.357239,
                lat: 48.831515,
            };
        })
        .stop_point("F", |sp| {
            sp.stop_area_id = "SA2".to_string();
            sp.coord = Coord {
                lon: 2.355906,
                lat: 48.830203,
            };
        })
        .vj("toto", |vj_builder| {
            vj_builder
                .route("R")
                .st("A", "10:00:00")
                .st("B", "10:01:00")
                .st("C", "10:02:00")
                .st("D", "10:03:00")
                .st("E", "10:04:00")
                .st("F", "10:05:00");
        })
        .build();

    let occupancy_data = loki::OccupancyData::empty();
    BaseModel::from_transit_model(model, occupancy_data, PositiveDuration::zero()).unwrap()
}

#[rstest]
fn places_nearby_error_handling(fixture_model: BaseModel) -> Result<(), Error> {
    let _log_guard = loki_launch::logger::init_test_logger();

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&fixture_model, &real_time_model);

    let places_nearby_iter = solve_places_nearby_request(&model_refs, "2.32610:48.82325", 500_f64);
    assert!(matches!(
        places_nearby_iter,
        Err(BadPlacesNearby::InvalidEntryPoint(_))
    ));

    let places_nearby_iter = solve_places_nearby_request(&model_refs, "A", 500_f64);
    assert!(matches!(
        places_nearby_iter,
        Err(BadPlacesNearby::InvalidEntryPoint(_))
    ));

    let places_nearby_iter = solve_places_nearby_request(&model_refs, "stop_pointA", 500_f64);
    assert!(matches!(
        places_nearby_iter,
        Err(BadPlacesNearby::InvalidEntryPoint(_))
    ));

    let places_nearby_iter =
        solve_places_nearby_request(&model_refs, "coord:2.32610;48.82325", 500_f64);
    assert!(matches!(
        places_nearby_iter,
        Err(BadPlacesNearby::InvalidFormatCoord(_))
    ));

    let places_nearby_iter = solve_places_nearby_request(&model_refs, "coord::", 500_f64);
    assert!(matches!(
        places_nearby_iter,
        Err(BadPlacesNearby::InvalidFormatCoord(_))
    ));

    let places_nearby_iter = solve_places_nearby_request(&model_refs, "coord:", 500_f64);
    assert!(matches!(
        places_nearby_iter,
        Err(BadPlacesNearby::InvalidFormatCoord(_))
    ));

    let places_nearby_iter = solve_places_nearby_request(&model_refs, "coord:2.32610:", 500_f64);
    assert!(matches!(
        places_nearby_iter,
        Err(BadPlacesNearby::InvalidFormatCoord(_))
    ));

    let places_nearby_iter = solve_places_nearby_request(&model_refs, "stop_point:Z", 500_f64);
    assert!(matches!(
        places_nearby_iter,
        Err(BadPlacesNearby::InvalidPtObject(_))
    ));

    let places_nearby_iter =
        solve_places_nearby_request(&model_refs, "coord:400.545:50.1854", 500_f64);

    assert!(matches!(
        places_nearby_iter,
        Err(BadPlacesNearby::InvalidRangeCoord(_))
    ));

    Ok(())
}

#[rstest]
fn places_nearby_coord(fixture_model: BaseModel) -> Result<(), Error> {
    let _log_guard = loki_launch::logger::init_test_logger();

    places_nearby_impl_test(
        &fixture_model,
        "coord:2.32610:48.82325",
        500_f64,
        &[("A", 38), ("B", 34), ("C", 42), ("D", 47)],
    )?;

    places_nearby_impl_test(
        &fixture_model,
        "coord:2.32610:48.82325",
        46_f64,
        &[("A", 38), ("B", 34), ("C", 42)],
    )?;

    places_nearby_impl_test(
        &fixture_model,
        "coord:2.35579:48.83104",
        500_f64,
        &[("E", 118), ("F", 93)],
    )?;

    Ok(())
}

#[rstest]
fn places_nearby_stop_point(fixture_model: BaseModel) -> Result<(), Error> {
    let _log_guard = loki_launch::logger::init_test_logger();

    places_nearby_impl_test(
        &fixture_model,
        "stop_point:A",
        500_f64,
        &[("A", 0), ("B", 64), ("C", 54), ("D", 29)],
    )?;

    places_nearby_impl_test(
        &fixture_model,
        "stop_point:E",
        500_f64,
        &[("E", 0), ("F", 175)],
    )?;

    Ok(())
}

#[rstest]
fn places_nearby_stop_area(fixture_model: BaseModel) -> Result<(), Error> {
    let _log_guard = loki_launch::logger::init_test_logger();

    places_nearby_impl_test(
        &fixture_model,
        "stop_area:SA1",
        500_f64,
        &[("A", 109), ("B", 73), ("C", 143), ("D", 137)],
    )?;

    places_nearby_impl_test(
        &fixture_model,
        "stop_area:SA2",
        500_f64,
        &[("E", 87), ("F", 87)],
    )?;

    Ok(())
}

fn places_nearby_impl_test(
    fixture_model: &BaseModel,
    uri: &str,
    radius_search: f64,
    expected_res: &[(&str, i32)],
) -> Result<(), Error> {
    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(fixture_model, &real_time_model);

    let places_nearby_iter = solve_places_nearby_request(&model_refs, uri, radius_search);
    assert!(places_nearby_iter.is_ok());

    let places_nearby_iter = places_nearby_iter.unwrap();
    let sp_list: Vec<(&str, i32)> = places_nearby_iter
        .into_iter()
        .map(|(idx, distance)| (model_refs.stop_point_name(&idx), distance as i32))
        .collect();

    assert_eq!(sp_list, *expected_res);

    Ok(())
}
