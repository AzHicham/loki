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

pub mod navitia_proto {
    include!(concat!(env!("OUT_DIR"), "/pbnavitia.rs"));
}

pub mod chaos_proto {
    include!(concat!(env!("OUT_DIR"), "/mod.rs"));
}

pub mod handle_kirin_message;
pub mod response;
pub mod zmq_worker;

pub mod compute_worker;
pub mod load_balancer;
pub mod master_worker;

pub mod config;
pub mod data_worker;

pub mod rabbitmq_worker;

use launch::{
    loki::tracing::{debug, info},
};

use structopt::StructOpt;

use std::{fs::File, io::BufReader, path::{PathBuf, Path}};

use failure::{bail, Error};

use config::Config;


#[derive(StructOpt)]
#[structopt(
    name = "loki_server",
    about = "Run loki server.",
    rename_all = "snake_case"
)]
pub struct Options {
    /// path to the json config file
    #[structopt(parse(from_os_str))]
    config_file: PathBuf,
}

fn main() {
    let _log_guard = launch::logger::init_logger();
    if let Err(err) = launch_server() {
        for cause in err.iter_chain() {
            eprintln!("{}", cause);
        }
        std::process::exit(1);
    }
}

fn launch_server() -> Result<(), Error> {
    let options = Options::from_args();
    let config = read_config(&options.config_file)?;
    launch_master_worker(config)
}

pub fn read_config(config_file: &Path) -> Result<Config, Error> {
    info!("Reading config from file {:?}", &config_file);
    let file = match File::open(&config_file) {
        Ok(file) => file,
        Err(e) => {
            bail!("Error opening config file {:?} : {}", &config_file, e)
        }
    };
    let reader = BufReader::new(file);
    let config: Config = serde_json::from_reader(reader)?;
    debug!("Launching with config : {:#?}", config);
    Ok(config)
}

fn launch_master_worker(config: Config) -> Result<(), Error> {
    let master_worker = master_worker::MasterWorker::new(config)?;
    master_worker.run_blocking()
}
