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
#![feature(test)]

#[path = "../tests/utils/mod.rs"]
mod utils;

extern crate test;
use launch::config::DataImplem;
use loki::model::{ModelRefs, real_time::RealTimeModel};
use test::Bencher;
use utils::{build_and_solve, model_builder::ModelBuilder, Config};

#[bench]
fn routing_daily_bench(bencher: &mut Bencher) {
    let model = ModelBuilder::new("2020-01-01", "2020-01-02")
        .vj("toto", |vj_builder| {
            vj_builder
                .st("A", "10:00:00")
                .st("B", "10:05:00")
                .st("C", "10:10:00");
        })
        .vj("tutu", |vj_builder| {
            vj_builder
                .st("A", "10:05:00")
                .st("B", "10:10:00")
                .st("C", "10:20:00");
        })
        .vj("tata", |vj_builder| {
            vj_builder
                .st("E", "10:05:00")
                .st("F", "10:20:00")
                .st("G", "10:30:00");
        })
        .add_transfer("B", "F", "00:02:00")
        .build();

    let config = Config::new("2020-01-01T09:59:00", "A", "G");
    let config = Config {
        data_implem: DataImplem::Daily,
        ..config
    };

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&model, &real_time_model);

    bencher.iter(|| {
        build_and_solve(&model_refs, &loki::LoadsData::empty(), &config).unwrap();
    });
}

#[bench]
fn routing_periodic_bench(bencher: &mut Bencher) {
    let model = ModelBuilder::new("2020-01-01", "2020-01-02")
        .vj("toto", |vj_builder| {
            vj_builder
                .st("A", "10:00:00")
                .st("B", "10:05:00")
                .st("C", "10:10:00");
        })
        .vj("tutu", |vj_builder| {
            vj_builder
                .st("A", "10:05:00")
                .st("B", "10:10:00")
                .st("C", "10:20:00");
        })
        .vj("tata", |vj_builder| {
            vj_builder
                .st("E", "10:05:00")
                .st("F", "10:20:00")
                .st("G", "10:30:00");
        })
        .add_transfer("B", "F", "00:02:00")
        .build();

    let config = Config::new("2020-01-01T09:59:00", "A", "G");
    let config = Config {
        data_implem: DataImplem::Periodic,
        ..config
    };
    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&model, &real_time_model);

    bencher.iter(|| {
        build_and_solve(&model_refs, &loki::LoadsData::empty(), &config).unwrap();
    });
}
