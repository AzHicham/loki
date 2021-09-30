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

// pub mod navitia_proto;
pub mod response;

// pub mod program;
// pub mod worker;

pub mod zmq_worker;

pub mod compute_worker;
pub mod master_worker;
mod realtime;

use launch::loki::{
    realtime::rt_model::RealTimeModel,
    timetables::{Timetables as TimetablesTrait, TimetablesIter},
    tracing::{debug, error, info, warn},
    transit_model, DailyData, PeriodicData, PeriodicSplitVjData, PositiveDuration, RequestInput,
};


use structopt::StructOpt;

use std::{fs::File, io::BufReader, path::PathBuf, thread};

use failure::{bail, Error};

use std::convert::TryFrom;

use crate::realtime::{BrockerConfig, RealTimeWorker};
use launch::datetime::DateTimeRepresent;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

#[derive(StructOpt)]
#[structopt(
    name = "loki_server",
    about = "Run loki server.",
    rename_all = "snake_case"
)]
pub enum Options {
    /// Create a config file from cli arguments
    CreateConfig(ConfigCreator),
    /// Launch from a config file
    ConfigFile(ConfigFile),
    /// Launch from cli arguments
    Launch(Config),
}

#[derive(StructOpt)]
#[structopt(rename_all = "snake_case")]
pub struct ConfigCreator {
    #[structopt(flatten)]
    pub config: Config,
}

#[derive(StructOpt)]
pub struct ConfigFile {
    /// path to the json config file
    #[structopt(parse(from_os_str))]
    file: PathBuf,
}

#[derive(Serialize, Deserialize, StructOpt, Debug)]
#[structopt(rename_all = "snake_case")]
pub struct Config {
    #[serde(flatten)]
    #[structopt(flatten)]
    launch_params: config::LaunchParams,

    /// zmq socket to listen for protobuf requests
    /// that will be handled with "basic" comparator
    #[structopt(long)]
    basic_requests_socket: String,

    /// zmq socket to listen for protobuf requests
    /// that will be handled with "loads" comparator
    #[structopt(long)]
    loads_requests_socket: Option<String>,

    #[serde(flatten)]
    #[structopt(flatten)]
    request_default_params: config::RequestParams,

    /// number of workers that solve requests in parallel
    #[structopt(long, default_value = DEFAULT_NB_THREADS)]
    #[serde(default = "default_nb_thread")]
    nb_workers: usize,
}

pub const DEFAULT_NB_THREADS: &str = "2";

pub fn default_nb_thread() -> usize {
    use std::str::FromStr;
    usize::from_str(DEFAULT_NB_THREADS).unwrap()
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
    match options {
        Options::ConfigFile(config_file) => {
            let config = read_config(&config_file)?;
            launch_master_worker(config)?;
            Ok(())
        }
        Options::CreateConfig(config_creator) => {
            let json_string = serde_json::to_string_pretty(&config_creator.config)?;

            println!("{}", json_string);

            Ok(())
        }
        Options::Launch(config) => {
            launch_master_worker(config)?;
            Ok(())
        }
    }
}

pub fn read_config(config_file: &ConfigFile) -> Result<Config, Error> {
    info!("Reading config from file {:?}", &config_file.file);
    let file = match File::open(&config_file.file) {
        Ok(file) => file,
        Err(e) => {
            bail!("Error opening config file {:?} : {}", &config_file.file, e)
        }
    };
    let reader = BufReader::new(file);
    let config: Config = serde_json::from_reader(reader)?;
    debug!("Launching with config : {:#?}", config);
    Ok(config)
}

fn launch_master_worker(config: Config) -> Result<(), Error> {
    let (data, model) = launch::read::<master_worker::MyTimetable>(&config.launch_params)?;
    let master_worker = master_worker::MasterWorker::new(
        model,
        data,
        config.nb_workers,
        config.basic_requests_socket,
        &config.request_default_params,
    )?;
    master_worker.run_blocking()
}
