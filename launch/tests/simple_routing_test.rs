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
use launch::solver::BasicCriteriaSolver;
use loki::modelbuilder::ModelBuilder;
use loki::PeriodicData;
use utils::{build_and_solve, Config};

fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[test]
fn test_simple_routing() -> Result<(), Error> {
    init();

    let model = ModelBuilder::default()
        .default_calendar(&["2020-01-01"])
        .vj("toto", |vj| {
            vj.route("1")
                .st("A", "10:00:00", "10:01:00")
                .st("B", "11:00:00", "11:01:00");
        })
        .vj("tata", |vj| {
            vj.st("A", "10:00:00", "10:01:00")
                .st("D", "11:00:00", "11:01:00");
        })
        .build();

    let config = Config::new(
        "20210728T100000".to_string(),
        "A".to_string(),
        "B".to_string(),
    );

    let responses = build_and_solve::<PeriodicData, BasicCriteriaSolver<PeriodicData>>(
        &model,
        &loki::LoadsData::empty(),
        &config,
    )?;

    assert_eq!(responses.len(), 1);

    assert_eq!(model.vehicle_journeys.len(), 2);

    Ok(())
}
