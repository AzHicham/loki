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
use failure::Error;
use launch::config::DataImplem;
use loki::{
    chrono_tz,
    models::{base_model::BaseModel, real_time_model::RealTimeModel, ModelRefs},
};
use utils::{
    build_and_solve,
    model_builder::{AsDate, AsDateTime, ModelBuilder},
    Config,
};

use rstest::rstest;

#[rstest]
#[case(DataImplem::Periodic)]
#[case(DataImplem::Daily)]
#[case(DataImplem::PeriodicSplitVj)]
fn test_daylight_saving_time_switch(#[case] data_implem: DataImplem) -> Result<(), Error> {
    utils::init_logger();

    // There is a daylight saving time switch in Europe/paris on 2020-10-25 :
    // - on 2020-10-24, "10:00:00" in Paris means "08:00:00" UTC
    // - on 2020-10-26, "10:00:00" in Paris means "09:00:00" UTC
    let model = ModelBuilder::new("2020-10-23", "2020-10-30")
        .vj("toto", |vj_builder| {
            vj_builder
                .timezone(&chrono_tz::Europe::Paris)
                .st("A", "10:00:00")
                .st("B", "10:05:00")
                .st("C", "10:10:00");
        })
        .build();
    let base_model = BaseModel::new(model);

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&base_model, &real_time_model);

    {
        let config = Config::new_timezoned("2020-10-24T06:00:00", &chrono_tz::UTC, "A", "B");
        let config = Config {
            data_implem,
            ..config
        };

        let responses = build_and_solve(&model_refs, &loki::LoadsData::empty(), &config)?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        let vehicle_sec = &journey.first_vehicle;
        // - on 2020-10-24, "10:00:00" in Paris means "08:00:00" UTC
        assert_eq!(
            vehicle_sec.from_datetime,
            "2020-10-24T08:00:00".as_datetime()
        );
    }

    {
        let config = Config::new_timezoned("2020-10-26T06:00:00", &chrono_tz::UTC, "A", "B");

        let responses = build_and_solve(&model_refs, &loki::LoadsData::empty(), &config)?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        let vehicle_sec = &journey.first_vehicle;
        // - on 2020-10-26, "10:00:00" in Paris means "09:00:00" UTC
        assert_eq!(
            vehicle_sec.from_datetime,
            "2020-10-26T09:00:00".as_datetime()
        );
    }

    Ok(())
}

#[rstest]
#[case(DataImplem::Periodic)]
#[case(DataImplem::Daily)]
#[case(DataImplem::PeriodicSplitVj)]
fn test_trip_over_daylight_saving_time_switch(
    #[case] data_implem: DataImplem,
) -> Result<(), Error> {
    utils::init_logger();

    // There is a daylight saving time switch in Europe/paris on 2020-10-25 at 02:00:00
    let model = ModelBuilder::new("2020-10-23", "2020-10-30")
        .vj("toto", |vj_builder| {
            vj_builder
                .timezone(&chrono_tz::Europe::Paris)
                .st("A", "00:00:00")
                .st("B", "01:05:00")
                .st("C", "02:10:00");
        })
        .build();

    let base_model = BaseModel::new(model);

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&base_model, &real_time_model);

    // We depart on 2020-10-23 at 22:00:00 UTC, so before the daylight saving time switch
    // this means we can board the vehicle journey on date 2020-10-24
    // as 00:00:00 on this day is 2020-10-23 at 22:00:00 UTC
    // we should arrive at 02:10:00 on 2020-10-24 which is 00:10:00 on 2020-10-24 UTC"
    {
        let config = Config::new_timezoned("2020-10-23T22:00:00", &chrono_tz::UTC, "A", "C");
        let config = Config {
            data_implem,
            ..config
        };

        let responses = build_and_solve(&model_refs, &loki::LoadsData::empty(), &config)?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        let vehicle_section = &journey.first_vehicle;
        assert_eq!(
            vehicle_section.from_datetime,
            "2020-10-23T22:00:00".as_datetime()
        );
        assert_eq!(
            vehicle_section.to_datetime,
            "2020-10-24T00:10:00".as_datetime()
        );
        assert_eq!(
            vehicle_section.day_for_vehicle_journey,
            "2020-10-24".as_date()
        );
    }

    // We depart on 2020-10-26 at 22:00:00 UTC, so after the daylight saving time switch
    // this means we can board the vehicle journey on date 2020-10-27
    // as 00:00:00 on this day is 2020-10-26 at 23:00:00 UTC
    // we should arrive at 02:10:00 on 2020-10-27 which is 01:10:00 on 2020-10-27 UTC"
    {
        let config = Config::new_timezoned("2020-10-26T22:00:00", &chrono_tz::UTC, "A", "C");
        let config = Config {
            data_implem,
            ..config
        };

        let responses = build_and_solve(&model_refs, &loki::LoadsData::empty(), &config)?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        let vehicle_section = &journey.first_vehicle;
        assert_eq!(
            vehicle_section.from_datetime,
            "2020-10-26T23:00:00".as_datetime()
        );
        assert_eq!(
            vehicle_section.to_datetime,
            "2020-10-27T01:10:00".as_datetime()
        );
        assert_eq!(
            vehicle_section.day_for_vehicle_journey,
            "2020-10-27".as_date()
        );
    }

    // We depart on 2020-10-24 at 22:00:00 UTC,
    // this is before the the daylight saving time switch.
    // We should be able to catch the vehicle journey on date 2020-10-25 (the day of the DST switch).
    // Since the DST switch happens at 02:00:00, and the local time are understood as
    // duration since "noon minus 12h", this means that local times for the vehicle journey on 2020-10-25
    // should be interpreted as "after the DST switch", i.e :
    // we board on A on 2020-10-24 at 23:00:00 UTC
    // we arrive on C on 2020-10-25 at 01:10:00 UTC
    {
        let config = Config::new_timezoned("2020-10-24T22:00:00", &chrono_tz::UTC, "A", "C");
        let config = Config {
            data_implem,
            ..config
        };

        let responses = build_and_solve(&model_refs, &loki::LoadsData::empty(), &config)?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        let vehicle_section = &journey.first_vehicle;
        assert_eq!(
            vehicle_section.from_datetime,
            "2020-10-24T23:00:00".as_datetime()
        );
        assert_eq!(
            vehicle_section.to_datetime,
            "2020-10-25T01:10:00".as_datetime()
        );
        assert_eq!(
            vehicle_section.day_for_vehicle_journey,
            "2020-10-25".as_date()
        );
    }

    Ok(())
}

#[rstest]
#[case(DataImplem::Periodic)]
#[case(DataImplem::Daily)]
#[case(DataImplem::PeriodicSplitVj)]
fn test_paris_london(#[case] data_implem: DataImplem) -> Result<(), Error> {
    utils::init_logger();

    // There is a daylight saving time switch in Europe/Paris AND Europe/London on 2020-10-25 at 02:00:00
    let model = ModelBuilder::new("2020-10-01", "2020-10-30")
        .vj("paris", |vj_builder| {
            vj_builder
                .timezone(&chrono_tz::Europe::Paris)
                .st("A", "10:00:00")
                .st("B", "11:05:00")
                .st("C", "12:10:00");
        })
        .vj("london", |vj_builder| {
            vj_builder
                .timezone(&chrono_tz::Europe::London)
                .st("C", "11:15:00")
                .st("D", "11:30:00")
                .st("E", "11:45:00");
        })
        .add_transfer("C", "C", "00:00:02")
        .build();

    let base_model = BaseModel::new(model);

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&base_model, &real_time_model);
    // Before the daylight saving time switch
    {
        let config = Config::new_timezoned("2020-10-23T08:00:00", &chrono_tz::UTC, "A", "E");
        let config = Config {
            data_implem,
            ..config
        };

        let responses = build_and_solve(&model_refs, &loki::LoadsData::empty(), &config)?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        let first_section = &journey.first_vehicle;

        assert_eq!(
            first_section.from_datetime,
            "2020-10-23T08:00:00".as_datetime()
        );
        assert_eq!(
            first_section.to_datetime,
            "2020-10-23T10:10:00".as_datetime()
        );

        let second_section = &journey.connections[0].2;
        assert_eq!(
            second_section.from_datetime,
            "2020-10-23T10:15:00".as_datetime()
        );
        assert_eq!(
            second_section.to_datetime,
            "2020-10-23T10:45:00".as_datetime()
        );
    }

    // After the daylight saving time switch
    {
        let config = Config::new_timezoned("2020-10-26T08:00:00", &chrono_tz::UTC, "A", "E");
        let config = Config {
            data_implem,
            ..config
        };

        let responses = build_and_solve(&model_refs, &loki::LoadsData::empty(), &config)?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        let first_section = &journey.first_vehicle;

        assert_eq!(
            first_section.from_datetime,
            "2020-10-26T09:00:00".as_datetime()
        );
        assert_eq!(
            first_section.to_datetime,
            "2020-10-26T11:10:00".as_datetime()
        );

        let second_section = &journey.connections[0].2;
        assert_eq!(
            second_section.from_datetime,
            "2020-10-26T11:15:00".as_datetime()
        );
        assert_eq!(
            second_section.to_datetime,
            "2020-10-26T11:45:00".as_datetime()
        );
    }

    Ok(())
}

#[rstest]
#[case(DataImplem::Periodic)]
#[case(DataImplem::Daily)]
#[case(DataImplem::PeriodicSplitVj)]
fn test_paris_new_york(#[case] data_implem: DataImplem) -> Result<(), Error> {
    utils::init_logger();

    // There is a daylight saving time switch in Europe/Paris  on 2020-10-25 at 02:00:00
    // But there is no switch in America/NewYork
    let model = ModelBuilder::new("2020-10-01", "2020-10-30")
        .vj("paris", |vj_builder| {
            vj_builder
                .timezone(&chrono_tz::Europe::Paris)
                .st("A", "14:00:00")
                .st("B", "15:05:00")
                .st("C", "16:10:00");
        })
        .vj("new_york", |vj_builder| {
            vj_builder
                .timezone(&chrono_tz::America::New_York)
                .st("C", "10:15:00")
                .st("D", "10:30:00")
                .st("E", "10:45:00");
        })
        .add_transfer("C", "C", "00:00:02")
        .build();

    let base_model = BaseModel::new(model);

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&base_model, &real_time_model);

    // Before the daylight saving time switch in Paris, we should be able to take the transfer at C
    // and hence get a journey from A to E
    {
        let config = Config::new_timezoned("2020-10-23T12:00:00", &chrono_tz::UTC, "A", "E");
        let config = Config {
            data_implem,
            ..config
        };

        let responses = build_and_solve(&model_refs, &loki::LoadsData::empty(), &config)?;

        assert_eq!(responses.len(), 1);
        let journey = &responses[0];
        let first_section = &journey.first_vehicle;

        assert_eq!(
            first_section.from_datetime,
            "2020-10-23T12:00:00".as_datetime()
        );
        assert_eq!(
            first_section.to_datetime,
            "2020-10-23T14:10:00".as_datetime()
        );

        let second_section = &journey.connections[0].2;
        assert_eq!(
            second_section.from_datetime,
            "2020-10-23T14:15:00".as_datetime()
        );
        assert_eq!(
            second_section.to_datetime,
            "2020-10-23T14:45:00".as_datetime()
        );
    }

    // After the daylight saving time switch in Paris, we should not be able to take the transfer at C
    // and hence should not get a journey from A to E
    {
        let config = Config::new_timezoned("2020-10-26T12:00:00", &chrono_tz::UTC, "A", "E");
        let config = Config {
            data_implem,
            ..config
        };

        let responses = build_and_solve(&model_refs, &loki::LoadsData::empty(), &config)?;

        assert_eq!(responses.len(), 0);
    }

    Ok(())
}
