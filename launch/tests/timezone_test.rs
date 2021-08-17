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
use loki::chrono_tz;
use utils::model_builder::ModelBuilder;
use utils::model_builder::{AsDate, AsDateTime};
use utils::{build_and_solve, Config};

#[test]
fn test_daylight_saving_time_switch() -> Result<(), Error> {
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

    {
        let config = Config::new_timezoned("2020-10-24T06:00:00", &chrono_tz::UTC, "A", "B");

        let responses = build_and_solve(&model, &loki::LoadsData::empty(), &config)?;

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

        let responses = build_and_solve(&model, &loki::LoadsData::empty(), &config)?;

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

#[test]
fn test_trip_over_daylight_saving_time_switch() -> Result<(), Error> {
    utils::init_logger();

    // There is a daylight saving time switch in Europe/paris on 2020-10-25 :
    // - on 2020-10-24, "10:00:00" in Paris means "08:00:00" UTC
    // - on 2020-10-26, "10:00:00" in Paris means "09:00:00" UTC
    let model = ModelBuilder::new("2020-10-23", "2020-10-30")
        .vj("toto", |vj_builder| {
            vj_builder
                .timezone(&chrono_tz::Europe::Paris)
                .st("A", "00:00:00")
                .st("B", "01:05:00")
                .st("C", "02:10:00");
        })
        .build();

    {
        let config = Config::new_timezoned("2020-10-23T22:00:00", &chrono_tz::UTC, "A", "C");

        let responses = build_and_solve(&model, &loki::LoadsData::empty(), &config)?;

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

    {
        let config = Config::new_timezoned("2020-10-26T22:00:00", &chrono_tz::UTC, "A", "C");

        let responses = build_and_solve(&model, &loki::LoadsData::empty(), &config)?;

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

    {
        let config = Config::new_timezoned("2020-10-24T22:00:00", &chrono_tz::UTC, "A", "C");

        let responses = build_and_solve(&model, &loki::LoadsData::empty(), &config)?;

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
