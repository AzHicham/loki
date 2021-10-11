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

use std::{fs::File, io::Write};

static MOD_RS: &[u8] = b"
/// Generated from protobuf.
/// @generated
pub mod gtfs_realtime;
/// Generated from protobuf.
/// @generated
/// This file is a protobuf extension, for test so far.
pub mod kirin;
/// Generated from protobuf.
/// @generated
/// This file is a protobuf extension, for test so far.
pub mod chaos;
";

fn main() {
    // create rust usable structs from protobuf files
    // see https://docs.rs/prost-build/0.6.1/prost_build/

    use std::env;
    let out_dir = env::var("OUT_DIR").unwrap();

    prost_build::compile_protos(
        &[
            "navitia-proto/request.proto",
            "navitia-proto/response.proto",
            "navitia-proto/task.proto",
            "navitia-proto/type.proto",
        ],
        &["navitia-proto/"],
    )
    .expect("Failed to generate protobuf code for navitia-proto.");
    println!("Writing protobuf code in {}/pbnavitia.rs", out_dir);

    protobuf_codegen_pure::Codegen::new()
        .out_dir(out_dir.as_str())
        .inputs(&[
            "chaos-proto/gtfs-realtime.proto",
            "chaos-proto/chaos.proto",
            "chaos-proto/kirin.proto",
        ])
        .include("chaos-proto")
        .run()
        .expect("Failed to generate protobuf code for chaos-proto.");
    File::create(out_dir + "/mod.rs")
        .expect("Could not create File mod.rs")
        .write_all(MOD_RS)
        .unwrap();
}
