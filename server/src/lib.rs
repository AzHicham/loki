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

pub mod navitia_proto {
    include!(concat!(env!("OUT_DIR"), "/pbnavitia.rs"));
}

pub mod chaos_proto {
    include!(concat!(env!("OUT_DIR"), "/mod.rs"));
}

#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_derive_enum;
extern crate core;

pub mod handle_chaos_message;
pub mod handle_kirin_message;
pub mod response;

pub mod chaos;
pub mod compute_worker;
pub mod data_downloader;
pub mod http_worker;
pub mod load_balancer;
pub mod master_worker;
pub mod status_worker;
pub mod zmq_worker;

pub mod data_worker;
pub mod server_config;

use launch::loki::tracing::{debug, info};
use server_config::ServerConfig;

use structopt::StructOpt;

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Error};

#[derive(StructOpt)]
#[structopt(
    name = "loki_server",
    about = "Run loki server.",
    rename_all = "snake_case"
)]
pub struct Options {
    /// path to the config file
    #[structopt(parse(from_os_str))]
    config_file: PathBuf,
}

pub fn launch_server() -> Result<(), Error> {
    let options = Options::from_args();
    let config = read_config(&options.config_file)?;
    launch_master_worker(config)
}

pub fn read_config(config_file_path: &Path) -> Result<ServerConfig, Error> {
    info!("Reading config from file {:?}", &config_file_path);
    let content = fs::read_to_string(&config_file_path)
        .with_context(|| format!("Error opening config file {:?}", &config_file_path))?;
    let config: ServerConfig = toml::from_str(&content)?;
    debug!("Launching with config : {:#?}", config);
    Ok(config)
}

pub fn launch_master_worker(config: ServerConfig) -> Result<(), Error> {
    let master_worker = master_worker::MasterWorker::new(config)?;
    master_worker.run_blocking()
}
