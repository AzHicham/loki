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
use failure::Error;
use launch::loki::chrono::NaiveDate;
use launch::loki::transit_model::objects::Date;
use loki::chrono::NaiveTime;
use loki::modelbuilder::ModelBuilder;
use loki::{NaiveDateTime, PeriodicData};
use std::str::FromStr;
use utils::{build_and_solve, make_pt_from_vehicle, make_stop_point, Config};

fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[test]
fn test_simple_routing() -> Result<(), Error> {
    init();

    let model = ModelBuilder::default()
        .calendar("service1", &["2020-01-01"])
        .route("1", |r| {
            r.name = String::from("bob");
        })
        .vj("toto", |vj_builder| {
            vj_builder
                .calendar("service1")
                .route("1")
                .st("A", "10:00:00", "10:00:01")
                .st("B", "10:05:00", "10:05:01")
                .st("C", "10:10:00", "10:10:01");
        })
        .validity_period(Date::from_str("2020-01-01")?, Date::from_str("2020-01-02")?)?
        .build();

    let config = Config::new(
        "20200101T085900".to_string(),
        "A".to_string(),
        "B".to_string(),
    );

    let responses = build_and_solve::<PeriodicData>(&model, &loki::LoadsData::empty(), &config)?;

    assert_eq!(model.vehicle_journeys.len(), 1);
    assert_eq!(responses.len(), 1);

    let journey = &responses[0];
    assert_eq!(journey.first_vj_uri(&model), "toto");
    assert_eq!(journey.nb_of_sections(), 1);
    assert_eq!(journey.connections.len(), 0);
    assert_eq!(
        journey.first_vehicle_board_datetime(),
        NaiveDateTime::new(
            NaiveDate::from_ymd(2020, 1, 1),
            NaiveTime::from_hms(9, 0, 1)
        )
    );
    assert_eq!(
        journey.first_vehicle.day_for_vehicle_journey,
        NaiveDate::from_ymd(2020, 1, 1)
    );

    assert_eq!(
        journey.last_vehicle_debark_datetime(),
        NaiveDateTime::new(
            NaiveDate::from_ymd(2020, 1, 1),
            NaiveTime::from_hms(9, 5, 0)
        )
    );

    assert_eq!(journey.nb_of_transfers(), 0);
    assert_eq!(journey.total_duration(), 360);

    let (from_sp, to_sp) = make_pt_from_vehicle(&journey.first_vehicle, &model)?;
    assert_eq!(from_sp.name, "A");
    assert_eq!(to_sp.name, "B");

    Ok(())
}

#[test]
fn test_routing_with_transfers() -> Result<(), Error> {
    init();

    let model = ModelBuilder::default()
        .calendar("service1", &["2020-01-01"])
        .vj("toto", |vj_builder| {
            vj_builder
                .calendar("service1")
                .route("1")
                .st("A", "10:00:00", "10:00:01")
                .st("B", "10:05:00", "10:05:01")
                .st("C", "10:10:00", "10:10:01");
        })
        .vj("tata", |vj_builder| {
            vj_builder
                .calendar("service1")
                .route("1")
                .st("E", "10:05:00", "10:05:01")
                .st("F", "10:20:00", "10:20:01")
                .st("G", "10:30:00", "10:30:01");
        })
        .add_transfer("B".into(), "F".into(), 120)
        .validity_period(Date::from_str("2020-01-01")?, Date::from_str("2020-01-02")?)?
        .build();

    let config = Config::new(
        "20200101T085900".to_string(),
        "A".to_string(),
        "G".to_string(),
    );

    let responses = build_and_solve::<PeriodicData>(&model, &loki::LoadsData::empty(), &config)?;

    assert_eq!(model.vehicle_journeys.len(), 2);
    assert_eq!(responses.len(), 1);

    let journey = &responses[0];
    assert_eq!(journey.first_vj_uri(&model), "toto");
    assert_eq!(journey.nb_of_sections(), 4);
    assert_eq!(
        journey.first_vehicle_board_datetime(),
        NaiveDateTime::new(
            NaiveDate::from_ymd(2020, 1, 1),
            NaiveTime::from_hms(9, 0, 1)
        )
    );
    assert_eq!(
        journey.first_vehicle.day_for_vehicle_journey,
        NaiveDate::from_ymd(2020, 1, 1)
    );

    assert_eq!(
        journey.last_vehicle_debark_datetime(),
        NaiveDateTime::new(
            NaiveDate::from_ymd(2020, 1, 1),
            NaiveTime::from_hms(9, 30, 0)
        )
    );

    assert_eq!(journey.nb_of_transfers(), 1);
    assert_eq!(journey.total_duration(), 1860);

    let (from_sp, to_sp) = make_pt_from_vehicle(&journey.first_vehicle, &model)?;
    assert_eq!(from_sp.name, "A");
    assert_eq!(to_sp.name, "B");

    assert_eq!(journey.connections.len(), 1);
    let transfer_section = &journey.connections[0].0;
    assert_eq!(
        make_stop_point(transfer_section.from_stop_point, &model)?.name,
        "B"
    );
    assert_eq!(
        make_stop_point(transfer_section.to_stop_point, &model)?.name,
        "F"
    );
    assert_eq!(
        transfer_section.from_datetime,
        NaiveDateTime::new(
            NaiveDate::from_ymd(2020, 1, 1),
            NaiveTime::from_hms(9, 5, 0)
        )
    );
    assert_eq!(
        transfer_section.to_datetime,
        NaiveDateTime::new(
            NaiveDate::from_ymd(2020, 1, 1),
            NaiveTime::from_hms(9, 7, 0)
        )
    );

    let waiting_section = &journey.connections[0].1;
    assert_eq!(
        make_stop_point(waiting_section.stop_point, &model)?.name,
        "F"
    );
    assert_eq!(
        waiting_section.from_datetime,
        NaiveDateTime::new(
            NaiveDate::from_ymd(2020, 1, 1),
            NaiveTime::from_hms(9, 7, 0)
        )
    );
    assert_eq!(
        waiting_section.to_datetime,
        NaiveDateTime::new(
            NaiveDate::from_ymd(2020, 1, 1),
            NaiveTime::from_hms(9, 20, 1)
        )
    );

    let (from_sp, to_sp) = make_pt_from_vehicle(&journey.connections[0].2, &model)?;
    assert_eq!(from_sp.name, "F");
    assert_eq!(to_sp.name, "G");

    Ok(())
}
