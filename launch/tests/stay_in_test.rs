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
use launch::config::launch_params::default_transfer_duration;

use loki::models::StopPointIdx;
use loki::tracing::info;
use loki::transit_data::data_interface::DataIters;
use loki::{
    models::{base_model::BaseModel, VehicleJourneyIdx},
    DataTrait, RealTimeLevel,
};
use utils::model_builder::ModelBuilder;

#[test]
fn simple_stay_in() -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // We set only one valid date in calendar for simplicity
    let model = ModelBuilder::new("2020-01-01", "2020-01-01")
        .vj("first", |vj_builder| {
            vj_builder
                .property("block_1")
                .st("A", "10:00:00")
                .st("B", "10:05:00")
                .st("C", "10:10:00");
        })
        .vj("second", |vj_builder| {
            vj_builder
                .property("block_1")
                .st("E", "10:20:00")
                .st("F", "10:30:00")
                .st("G", "10:40:00");
        })
        .build();

    let base_model =
        BaseModel::from_transit_model(model, loki::LoadsData::empty(), default_transfer_duration())
            .unwrap();

    let data = launch::read::build_transit_data(&base_model);

    let vehicle_journey_idx = base_model.vehicle_journey_idx("first").unwrap();
    let vj_idx_first = VehicleJourneyIdx::Base(vehicle_journey_idx);

    let vehicle_journey_idx = base_model.vehicle_journey_idx("second").unwrap();
    let vj_idx_second = VehicleJourneyIdx::Base(vehicle_journey_idx);

    let stop_point_idx = base_model.stop_point_idx("C").unwrap();
    let stop_point_idx = StopPointIdx::Base(stop_point_idx);
    let stop = data.stop_point_idx_to_stop(&stop_point_idx).unwrap();

    // we have only one Trip going through stop_point 'C'
    // Trip { vehicle_journey 'first' on date 2020-01-01 }
    let (mission, _) = data.missions_of(stop).next().unwrap();

    for trip in data.trips_of(&mission, RealTimeLevel::Base) {
        // we assert here to make sure we are iterating over the right trip/vehicle_journey
        assert_eq!(vj_idx_first, data.vehicle_journey_idx(&trip));
        let next_trip_stay_in = data.stay_in_next(&trip, RealTimeLevel::Base).unwrap();
        // more important, this assert test is we have a trip to stay_in after
        // trip { vj 'first' on date 2020-01-01 }
        let next_vj_idx = data.vehicle_journey_idx(&next_trip_stay_in);
        assert_eq!(next_vj_idx, vj_idx_second);
    }

    // we have only one Trip going through stop_point 'E'
    // Trip { vehicle_journey 'second' on date 2020-01-01
    let stop_point_idx = base_model.stop_point_idx("E").unwrap();
    let stop_point_idx = StopPointIdx::Base(stop_point_idx);
    let stop = data.stop_point_idx_to_stop(&stop_point_idx).unwrap();
    let (mission, _) = data.missions_of(stop).next().unwrap();

    for trip in data.trips_of(&mission, RealTimeLevel::Base) {
        // we assert here to make sure we are iterating over the right trip/vehicle_journey
        assert_eq!(vj_idx_second, data.vehicle_journey_idx(&trip));
        let next_trip_stay_in = data.stay_in_next(&trip, RealTimeLevel::Base);
        // more important, this assert test if we do not have a trip to stay_in after
        // trip { vj 'second' on date 2020-01-01 }
        assert!(next_trip_stay_in.is_none());
    }

    Ok(())
}

#[test]
fn multiple_stay() -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // We set only one valid date in calendar for simplicity
    let model = ModelBuilder::new("2020-01-01", "2020-01-01")
        .vj("first", |vj_builder| {
            vj_builder
                .property("block_1")
                .st("A", "10:00:00")
                .st("B", "10:05:00")
                .st("C", "10:10:00");
        })
        .vj("second_a", |vj_builder| {
            vj_builder
                .property("block_1")
                .st("E", "10:15:00")
                .st("F", "10:20:00")
                .st("G", "10:25:00");
        })
        .vj("second_b", |vj_builder| {
            vj_builder
                .property("block_1")
                .st("E", "10:20:00")
                .st("F", "10:25:00")
                .st("G", "10:30:00");
        })
        .build();

    let base_model =
        BaseModel::from_transit_model(model, loki::LoadsData::empty(), default_transfer_duration())
            .unwrap();

    let data = launch::read::build_transit_data(&base_model);

    let vehicle_journey_idx = base_model.vehicle_journey_idx("first").unwrap();
    let vj_idx_first = VehicleJourneyIdx::Base(vehicle_journey_idx);

    let vehicle_journey_idx = base_model.vehicle_journey_idx("second_a").unwrap();
    let vj_idx_second_a = VehicleJourneyIdx::Base(vehicle_journey_idx);

    let stop_point_idx = base_model.stop_point_idx("C").unwrap();
    let stop_point_idx = StopPointIdx::Base(stop_point_idx);
    let stop = data.stop_point_idx_to_stop(&stop_point_idx).unwrap();

    // we have only one Trip going through stop_point 'C'
    // Trip { vehicle_journey 'first' on date 2020-01-01 }
    let (mission, _) = data.missions_of(stop).next().unwrap();

    for trip in data.trips_of(&mission, RealTimeLevel::Base) {
        assert_eq!(vj_idx_first, data.vehicle_journey_idx(&trip));
        let next_trip_stay_in = data.stay_in_next(&trip, RealTimeLevel::Base).unwrap();

        let next_vj_idx = data.vehicle_journey_idx(&next_trip_stay_in);
        assert_eq!(next_vj_idx, vj_idx_second_a);
    }

    Ok(())
}

#[test]
fn multiple_stay_in_with_wrong_stoptimes() -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // We set only one valid date in calendar for simplicity
    let model = ModelBuilder::new("2020-01-01", "2020-01-01")
        .vj("first", |vj_builder| {
            vj_builder
                .property("block_1")
                .st("A", "10:00:00")
                .st("B", "10:05:00")
                .st("C", "10:10:00");
        })
        .vj("second_a", |vj_builder| {
            vj_builder
                .property("block_1")
                .st("E", "10:05:00")
                .st("F", "10:10:00")
                .st("G", "10:15:00");
        })
        .vj("second_b", |vj_builder| {
            vj_builder
                .property("block_1")
                .st("E", "10:15:00")
                .st("F", "10:20:00")
                .st("G", "10:25:00");
        })
        .build();

    let base_model =
        BaseModel::from_transit_model(model, loki::LoadsData::empty(), default_transfer_duration())
            .unwrap();

    let data = launch::read::build_transit_data(&base_model);

    let vehicle_journey_idx = base_model.vehicle_journey_idx("first").unwrap();
    let vj_idx_first = VehicleJourneyIdx::Base(vehicle_journey_idx);

    let stop_point_idx = base_model.stop_point_idx("C").unwrap();
    let stop_point_idx = StopPointIdx::Base(stop_point_idx);
    let stop = data.stop_point_idx_to_stop(&stop_point_idx).unwrap();

    // we have only one Trip going through stop_point 'C'
    // Trip { vehicle_journey 'first' on date 2020-01-01 }
    let (mission, _) = data.missions_of(stop).next().unwrap();

    for trip in data.trips_of(&mission, RealTimeLevel::Base) {
        assert_eq!(vj_idx_first, data.vehicle_journey_idx(&trip));
        let next_trip_stay_in = data.stay_in_next(&trip, RealTimeLevel::Base);
        assert!(next_trip_stay_in.is_none());
    }

    Ok(())
}

#[test]
fn stay_in_with_wrong_stoptimes() -> Result<(), Error> {
    let _log_guard = launch::logger::init_test_logger();

    // We set only one valid date in calendar for simplicity
    let model = ModelBuilder::new("2020-01-01", "2020-01-01")
        .vj("first", |vj_builder| {
            vj_builder
                .property("block_1")
                .st("A", "10:00:00")
                .st("B", "10:05:00")
                .st("C", "10:10:00");
        })
        .vj("second", |vj_builder| {
            vj_builder
                .property("block_1")
                .st("E", "10:05:00")
                .st("F", "10:30:00")
                .st("G", "10:40:00");
        })
        .build();

    let base_model =
        BaseModel::from_transit_model(model, loki::LoadsData::empty(), default_transfer_duration())
            .unwrap();

    let data = launch::read::build_transit_data(&base_model);

    let vehicle_journey_idx = base_model.vehicle_journey_idx("first").unwrap();
    let vj_idx_first = VehicleJourneyIdx::Base(vehicle_journey_idx);

    let stop_point_idx = base_model.stop_point_idx("C").unwrap();
    let stop_point_idx = StopPointIdx::Base(stop_point_idx);
    let stop = data.stop_point_idx_to_stop(&stop_point_idx).unwrap();

    // we have only one Trip going through stop_point 'C'
    // Trip { vehicle_journey 'first' on date 2020-01-01 }
    let (mission, _) = data.missions_of(stop).next().unwrap();

    for trip in data.trips_of(&mission, RealTimeLevel::Base) {
        // we assert here to make sure we are iterating over the right trip/vehicle_journey
        assert_eq!(vj_idx_first, data.vehicle_journey_idx(&trip));
        let next_trip_stay_in = data.stay_in_next(&trip, RealTimeLevel::Base);
        // more important, this assert test if we do not have a trip to stay_in after
        // trip { vj 'first' on date 2020-01-01 }
        // because departure time of vj 'second' at stop_point 'E' is before
        // arrival_time of vj 'first' at stop_point 'C'
        assert!(next_trip_stay_in.is_none());
    }

    Ok(())
}
