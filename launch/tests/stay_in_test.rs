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
use loki_launch::config::launch_params::default_transfer_duration;

use loki::{
    chrono::NaiveDate,
    models::{base_model::BaseModel, StopPointIdx, VehicleJourneyIdx},
    transit_data::data_interface::{DataIters, TransitTypes},
    transit_model::objects::Date,
    DataTrait, RealTimeLevel, TransitData,
};
use utils::model_builder::ModelBuilder;

fn get_first_trip_of_vehicle_journey(
    vj_id: &str,
    data: &TransitData,
    base_model: &BaseModel,
) -> Option<<TransitData as TransitTypes>::Trip> {
    let vehicle_journey_idx = base_model.vehicle_journey_idx(vj_id).unwrap();
    let vehicle_journey = base_model.vehicle_journey(vehicle_journey_idx);

    // take first stop_point_idx
    let stop_point_idx = &vehicle_journey.stop_times.first().unwrap().stop_point_idx;
    let stop_point_idx = StopPointIdx::Base(*stop_point_idx);
    let stop = data.stop_point_idx_to_stop(&stop_point_idx).unwrap();

    let (mission, _) = data.missions_of(stop).next().unwrap();
    data.trips_of(&mission, RealTimeLevel::Base).next()
}

fn get_next_trip(
    vj_id: &str,
    data: &TransitData,
    base_model: &BaseModel,
) -> Option<<TransitData as TransitTypes>::Trip> {
    let trip = get_first_trip_of_vehicle_journey(vj_id, data, base_model).unwrap();
    data.stay_in_next(&trip, RealTimeLevel::Base)
}

fn get_prev_trip(
    vj_id: &str,
    data: &TransitData,
    base_model: &BaseModel,
) -> Option<<TransitData as TransitTypes>::Trip> {
    let trip = get_first_trip_of_vehicle_journey(vj_id, data, base_model).unwrap();
    data.stay_in_previous(&trip, RealTimeLevel::Base)
}

fn is_forward_stay_in(
    vj_id: &str,
    expected_next_vj_id: Option<&str>,
    data: &TransitData,
    base_model: &BaseModel,
) -> bool {
    let has_next_trip_stay_in = get_next_trip(vj_id, data, base_model);
    match expected_next_vj_id {
        Some(expected_next_vj_id) => {
            let next_trip_stay_in = has_next_trip_stay_in.unwrap();
            let next_vj_idx = data.vehicle_journey_idx(&next_trip_stay_in);
            let expected_next_vj_idx = base_model.vehicle_journey_idx(expected_next_vj_id).unwrap();
            let expected_next_vj_idx = VehicleJourneyIdx::Base(expected_next_vj_idx);
            expected_next_vj_idx == next_vj_idx
        }
        None => has_next_trip_stay_in.is_none(),
    }
}

fn is_backward_stay_in(
    vj_id: &str,
    expected_prev_vj_id: Option<&str>,
    data: &TransitData,
    base_model: &BaseModel,
) -> bool {
    let has_next_trip_stay_in = get_prev_trip(vj_id, data, base_model);
    match expected_prev_vj_id {
        Some(expected_prev_vj_id) => {
            let next_trip_stay_in = has_next_trip_stay_in.unwrap();
            let next_vj_idx = data.vehicle_journey_idx(&next_trip_stay_in);
            let expected_prev_vj_idx = base_model.vehicle_journey_idx(expected_prev_vj_id).unwrap();
            let expected_prev_vj_idx = VehicleJourneyIdx::Base(expected_prev_vj_idx);
            expected_prev_vj_idx == next_vj_idx
        }
        None => has_next_trip_stay_in.is_none(),
    }
}

#[test]
fn simple_stay_in() -> Result<(), Error> {
    let _log_guard = loki_launch::logger::init_test_logger();

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

    let data = loki_launch::read::build_transit_data(&base_model);

    {
        // this assert test if we have a trip to stay_in after trip { vj 'first' on date 2020-01-01 }
        // we should find the following Trip { vj 'second' on date 2020-01-01 }
        assert!(is_forward_stay_in(
            "first",
            Some("second"),
            &data,
            &base_model
        ));

        // this assert test if we have a trip to stay_in after trip { vj 'second' on date 2020-01-01 }
        // we expect no stay_in trip
        assert!(is_forward_stay_in("second", None, &data, &base_model));
    }

    {
        // this assert test if we have a trip to stay_in before trip { vj 'second' on date 2020-01-01 }
        // we should find the following Trip { vj 'first' on date 2020-01-01 }
        assert!(is_backward_stay_in(
            "second",
            Some("first"),
            &data,
            &base_model
        ));

        // this assert test if we have a trip to stay_in before trip { vj 'first' on date 2020-01-01 }
        // we expect no trip
        assert!(is_backward_stay_in("first", None, &data, &base_model));
    }

    Ok(())
}

#[test]
fn multiple_stay() -> Result<(), Error> {
    let _log_guard = loki_launch::logger::init_test_logger();

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
                .st("R", "10:20:00")
                .st("F", "10:25:00")
                .st("G", "10:30:00");
        })
        .build();

    let base_model =
        BaseModel::from_transit_model(model, loki::LoadsData::empty(), default_transfer_duration())
            .unwrap();

    let data = loki_launch::read::build_transit_data(&base_model);

    {
        // this assert test if we have a trip to stay_in after trip { vj 'first' on date 2020-01-01 }
        // we should find the following Trip { vj 'second' on date 2020-01-01 }
        // and not Trip with vj_id 'second_b'
        assert!(is_forward_stay_in(
            "first",
            Some("second_a"),
            &data,
            &base_model
        ));

        // We test that both 'second_a' & 'second_b' have no next_trip stay_in
        assert!(is_forward_stay_in("second_a", None, &data, &base_model));

        assert!(is_forward_stay_in("second_b", None, &data, &base_model));
    }

    {
        // this assert test if we have a trip to stay_in before trip { vj 'second_a' on date 2020-01-01 }
        // we should find the following Trip { vj 'first' on date 2020-01-01 }
        // and not Trip with vj_id 'second_b'
        assert!(is_backward_stay_in(
            "second_a",
            Some("first"),
            &data,
            &base_model
        ));

        assert!(is_backward_stay_in("first", None, &data, &base_model));

        assert!(is_backward_stay_in("second_b", None, &data, &base_model));
    }

    Ok(())
}

#[test]
fn stay_in_with_wrong_stoptimes() -> Result<(), Error> {
    let _log_guard = loki_launch::logger::init_test_logger();

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

    let data = loki_launch::read::build_transit_data(&base_model);

    // this assert test if we have a trip to stay_in after trip { vj 'first' on date 2020-01-01 }
    // we should find no nxt_trip stay_in
    // because vj 'first' arrival time at stop_point 'C' is greater than
    // departure time of vj 'second_a' at stop_point 'E'
    assert!(is_forward_stay_in("first", None, &data, &base_model));
    assert!(is_forward_stay_in("second", None, &data, &base_model));

    assert!(is_backward_stay_in("first", None, &data, &base_model));
    assert!(is_backward_stay_in("second", None, &data, &base_model));

    Ok(())
}

#[test]
fn multiple_stay_in_with_wrong_stoptimes() -> Result<(), Error> {
    let _log_guard = loki_launch::logger::init_test_logger();

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

    let data = loki_launch::read::build_transit_data(&base_model);

    // this assert test if we have a trip to stay_in after trip { vj 'first' on date 2020-01-01 }
    // we should find no next_trip stay_in
    // because vj 'first' arrival time at stop_point 'C' is greater than
    // departure time of vj 'second_a' at stop_point 'E'
    // Also 'second_b' vj cannot be return because it's departure time at stop_point 'E' ig greater than
    // vj 'second_a' departure time at stop_point 'E'

    // Note: VJ are first sorted by the the departure time at first StopTime so
    // first < second_a < second_b
    // The ONLY POSSIBLE next_vj of 'first' is 'second_a'  but 'second_a' does not meet criteria to become the next_vj of 'first'
    // Following that 'first' does not have a next_vj. (even if 'second_b' meet required criteria to become next_vj of 'first')

    assert!(is_forward_stay_in("first", None, &data, &base_model));

    // 'second_b' is the next_vj stay_in of 'second_a'
    assert!(is_forward_stay_in(
        "second_a",
        Some("second_b"),
        &data,
        &base_model
    ));
    Ok(())
}

#[test]
fn chain_multiple_stay_in() -> Result<(), Error> {
    let _log_guard = loki_launch::logger::init_test_logger();

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
                .st("E", "10:10:00")
                .st("F", "10:15:00")
                .st("G", "10:20:00");
        })
        .vj("third", |vj_builder| {
            vj_builder
                .property("block_1")
                .st("H", "10:30:00")
                .st("I", "10:40:00")
                .st("J", "10:50:00");
        })
        .build();

    let base_model =
        BaseModel::from_transit_model(model, loki::LoadsData::empty(), default_transfer_duration())
            .unwrap();

    let data = loki_launch::read::build_transit_data(&base_model);

    {
        // this assert test if we have a trip to stay_in after trip { vj 'first' on date 2020-01-01 }
        // we should find the following Trip { vj 'second' on date 2020-01-01 }
        assert!(is_forward_stay_in(
            "first",
            Some("second"),
            &data,
            &base_model
        ));

        // this assert test if we have a trip to stay_in after trip { vj 'second' on date 2020-01-01 }
        // we should find the following Trip { vj 'third' on date 2020-01-01 }
        assert!(is_forward_stay_in(
            "second",
            Some("third"),
            &data,
            &base_model
        ));

        // this assert test if we have a trip to stay_in after trip { vj 'third' on date 2020-01-01 }
        // we should find no Trip
        assert!(is_forward_stay_in("third", None, &data, &base_model));
    }

    {
        assert!(is_backward_stay_in(
            "third",
            Some("second"),
            &data,
            &base_model
        ));

        assert!(is_backward_stay_in(
            "second",
            Some("first"),
            &data,
            &base_model
        ));

        assert!(is_backward_stay_in("first", None, &data, &base_model));
    }

    Ok(())
}

#[test]
fn stay_in_with_local_zone() -> Result<(), Error> {
    let _log_guard = loki_launch::logger::init_test_logger();

    // We set only one valid date in calendar for simplicity
    let model = ModelBuilder::new("2020-01-01", "2020-01-01")
        .vj("first", |vj_builder| {
            vj_builder
                .property("block_1")
                .st_detailed("A", "10:10:00", "10:10:00", 0u8, 0u8, Some(1u16))
                .st_detailed("B", "10:15:00", "10:15:00", 0u8, 0u8, Some(1u16))
                .st_detailed("C", "10:20:00", "10:20:00", 0u8, 0u8, Some(2u16));
        })
        .vj("second", |vj_builder| {
            vj_builder
                .property("block_1")
                .st("E", "10:20:00")
                .st("F", "10:30:00")
                .st("G", "10:40:00");
        })
        .vj("third", |vj_builder| {
            vj_builder
                .property("block_1")
                .st_detailed("H", "10:45:00", "10:45:00", 0u8, 0u8, Some(1u16))
                .st_detailed("I", "10:50:00", "10:50:00", 0u8, 0u8, Some(1u16))
                .st_detailed("J", "10:55:00", "10:55:00", 0u8, 0u8, Some(2u16));
        })
        .build();

    let base_model =
        BaseModel::from_transit_model(model, loki::LoadsData::empty(), default_transfer_duration())
            .unwrap();

    let data = loki_launch::read::build_transit_data(&base_model);

    {
        assert!(is_forward_stay_in("first", None, &data, &base_model));

        assert!(is_forward_stay_in("second", None, &data, &base_model));

        assert!(is_forward_stay_in("third", None, &data, &base_model));
    }

    {
        assert!(is_backward_stay_in("first", None, &data, &base_model));

        assert!(is_backward_stay_in("second", None, &data, &base_model));

        assert!(is_backward_stay_in("third", None, &data, &base_model));
    }

    Ok(())
}

#[test]
fn different_validity_day_stay_in() -> Result<(), Error> {
    let _log_guard = loki_launch::logger::init_test_logger();

    // We set only one valid date in calendar for simplicity
    let model = ModelBuilder::new("2020-01-01", "2020-01-02")
        .calendar_mut("c1", |c| {
            c.dates.insert(Date::from_ymd(2020, 1, 1));
        })
        .calendar_mut("c2", |c| {
            c.dates.insert(Date::from_ymd(2020, 1, 2));
        })
        .vj("first", |vj_builder| {
            vj_builder
                .calendar("c1")
                .property("block_1")
                .st("A", "10:00:00")
                .st("B", "10:05:00")
                .st("C", "10:10:00");
        })
        .vj("second", |vj_builder| {
            vj_builder
                .calendar("c2")
                .property("block_1")
                .st("E", "10:20:00")
                .st("F", "10:25:00")
                .st("G", "10:30:00");
        })
        .build();

    let base_model =
        BaseModel::from_transit_model(model, loki::LoadsData::empty(), default_transfer_duration())
            .unwrap();

    let data = loki_launch::read::build_transit_data(&base_model);

    // this assert test if we have a trip to stay_in after trip { vj 'first' on date 2020-01-01 }
    // we should find no Trip because vj 'second' is valid on a different day ie '2020-01-02"
    assert!(is_forward_stay_in("first", None, &data, &base_model));

    assert!(is_backward_stay_in("second", None, &data, &base_model));

    Ok(())
}

#[test]
fn multiple_day_stay_in() -> Result<(), Error> {
    let _log_guard = loki_launch::logger::init_test_logger();

    // We set 10 days of validity in the calendar
    let model = ModelBuilder::new("2020-01-01", "2020-01-10")
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
                .st("F", "10:25:00")
                .st("G", "10:30:00");
        })
        .build();

    let base_model =
        BaseModel::from_transit_model(model, loki::LoadsData::empty(), default_transfer_duration())
            .unwrap();

    let data = loki_launch::read::build_transit_data(&base_model);

    let vehicle_journey_idx = base_model.vehicle_journey_idx("first").unwrap();
    let vehicle_journey_first = base_model.vehicle_journey(vehicle_journey_idx);

    let vj_second_idx = base_model.vehicle_journey_idx("second").unwrap();
    let vj_second_idx = VehicleJourneyIdx::Base(vj_second_idx);

    // take first stop_point_idx of vehicle_journey_first
    let stop_point_idx = &vehicle_journey_first
        .stop_times
        .first()
        .unwrap()
        .stop_point_idx;
    let stop_point_idx = StopPointIdx::Base(*stop_point_idx);
    let stop = data.stop_point_idx_to_stop(&stop_point_idx).unwrap();

    let mut current_date = NaiveDate::from_ymd(2020, 1, 1);
    // we test that next_trip works of each day of validity
    let (mission, _) = data.missions_of(stop).next().unwrap();

    for trip in data.trips_of(&mission, RealTimeLevel::Base) {
        let next_trip_stay_in = data.stay_in_next(&trip, RealTimeLevel::Base).unwrap();
        let next_vj_idx = data.vehicle_journey_idx(&next_trip_stay_in);

        assert_eq!(current_date, data.day_of(&next_trip_stay_in));
        assert_eq!(next_vj_idx, vj_second_idx);
        current_date = current_date.succ();
    }
    assert_eq!(current_date, NaiveDate::from_ymd(2020, 1, 11));

    Ok(())
}

#[test]
fn past_midnight_on_same_valid_day_stay_in() -> Result<(), Error> {
    let _log_guard = loki_launch::logger::init_test_logger();

    let model = ModelBuilder::new("2020-01-01", "2020-01-01")
        .vj("first", |vj_builder| {
            vj_builder
                .property("block_1")
                .st("A", "23:00:00")
                .st("B", "24:00:00")
                .st("C", "24:30:00");
        })
        .vj("second", |vj_builder| {
            vj_builder
                .property("block_1")
                .st("E", "24:50:00")
                .st("F", "25:25:00")
                .st("G", "25:30:00");
        })
        .build();

    let base_model =
        BaseModel::from_transit_model(model, loki::LoadsData::empty(), default_transfer_duration())
            .unwrap();

    let data = loki_launch::read::build_transit_data(&base_model);

    assert!(is_forward_stay_in(
        "first",
        Some("second"),
        &data,
        &base_model
    ));

    assert!(is_backward_stay_in(
        "second",
        Some("first"),
        &data,
        &base_model
    ));

    Ok(())
}

#[test]
fn past_midnight_on_different_valid_day_stay_in() -> Result<(), Error> {
    let _log_guard = loki_launch::logger::init_test_logger();

    let model = ModelBuilder::new("2020-01-01", "2020-01-2")
        .calendar_mut("c1", |c| {
            c.dates.insert(Date::from_ymd(2020, 1, 1));
        })
        .calendar_mut("c2", |c| {
            c.dates.insert(Date::from_ymd(2020, 1, 2));
        })
        .vj("first", |vj_builder| {
            vj_builder
                .property("block_1")
                .calendar("c1")
                .st("A", "23:00:00")
                .st("B", "24:00:00")
                .st("C", "24:30:00");
        })
        .vj("second", |vj_builder| {
            vj_builder
                .calendar("c2")
                .property("block_1")
                .st("E", "00:50:00")
                .st("F", "01:25:00")
                .st("G", "01:30:00");
        })
        .build();

    let base_model =
        BaseModel::from_transit_model(model, loki::LoadsData::empty(), default_transfer_duration())
            .unwrap();

    let data = loki_launch::read::build_transit_data(&base_model);

    // a stay-in may happens only between trips with the same reference date
    // so even if the stop_times would allow to stay-in from (vj 'first' on 2020-01-01) to (vj 'second' on 2020-01-02)
    // the stay-in should be absent because of the different circulation dates
    assert!(is_forward_stay_in("first", None, &data, &base_model));

    assert!(is_backward_stay_in("second", None, &data, &base_model));

    Ok(())
}
