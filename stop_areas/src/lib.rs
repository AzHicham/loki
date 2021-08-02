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

use launch::loki::{self, DataWithIters};
use launch::{
    config,
    loki::{DailyData, LoadsDailyData, LoadsPeriodicData, PeriodicData},
    solver,
};

use loki::log;

use loki::{log::debug, transit_model::Model};

use std::{fs::File, io::BufReader, time::SystemTime};

use failure::{bail, Error};

use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
    name = "loki_stop_areas",
    about = "Perform a public transport request between two stop areas.",
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
    file: std::path::PathBuf,
}
#[derive(Serialize, Deserialize, StructOpt)]
#[structopt(rename_all = "snake_case")]
pub struct Config {
    #[serde(flatten)]
    #[structopt(flatten)]
    pub launch_params: config::LaunchParams,

    #[serde(flatten)]
    #[structopt(flatten)]
    pub request_params: config::RequestParams,

    /// Departure datetime of the query, formatted like 20190628T163215
    /// If none is given, all queries will be made at 08:00:00 on the first
    /// valid day of the dataset
    #[structopt(long)]
    pub departure_datetime: Option<String>,

    /// Which comparator to use for the request
    /// "basic" or "loads"
    #[serde(default)]
    #[structopt(long, default_value)]
    pub comparator_type: config::ComparatorType,

    /// name of the start stop_area
    #[structopt(long)]
    pub start: String,

    /// name of the end stop_area
    #[structopt(long)]
    pub end: String,
}

pub fn run() -> Result<(), Error> {
    let options = Options::from_args();
    match options {
        Options::ConfigFile(config_file) => {
            let config = read_config(&config_file)?;
            launch(config)?;
            Ok(())
        }
        Options::CreateConfig(config_creator) => {
            let json_string = serde_json::to_string_pretty(&config_creator.config)?;

            println!("{}", json_string);

            Ok(())
        }
        Options::Launch(config) => {
            launch(config)?;
            Ok(())
        }
    }
}

pub fn read_config(config_file: &ConfigFile) -> Result<Config, Error> {
    let file = match File::open(&config_file.file) {
        Ok(file) => file,
        Err(e) => {
            bail!("Error opening config file {:?} : {}", &config_file.file, e)
        }
    };
    let reader = BufReader::new(file);
    let config: Config = serde_json::from_reader(reader).map_err(|err| {
        failure::format_err!(
            "Could not read config file {:?} : {}",
            config_file.file,
            err
        )
    })?;
    Ok(config)
}

pub fn launch(config: Config) -> Result<(Model, Vec<loki::Response>), Error> {
    match config.launch_params.data_implem {
        config::DataImplem::Periodic => config_launch::<PeriodicData>(config),
        config::DataImplem::Daily => config_launch::<DailyData>(config),
        config::DataImplem::LoadsPeriodic => config_launch::<LoadsPeriodicData>(config),
        config::DataImplem::LoadsDaily => config_launch::<LoadsDailyData>(config),
    }
}

fn config_launch<Data>(config: Config) -> Result<(Model, Vec<loki::Response>), Error>
where
    Data: DataWithIters,
{
    let (data, model) = launch::read(&config.launch_params)?;
    let result = match config.launch_params.criteria_implem {
        config::CriteriaImplem::Basic => build_engine_and_solve::<
            Data,
            solver::BasicCriteriaSolver<Data>,
        >(&model, &data, &config),
        config::CriteriaImplem::Loads => build_engine_and_solve::<
            Data,
            solver::LoadsCriteriaSolver<Data>,
        >(&model, &data, &config),
    };

    result.map(|responses| (model, responses))
}

fn build_engine_and_solve<Data, Solver>(
    model: &Model,
    data: &Data,
    config: &Config,
) -> Result<Vec<loki::Response>, Error>
where
    Data: DataWithIters,
    Solver: solver::Solver<Data>,
{
    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    let departure_datetime = match &config.departure_datetime {
        Some(string_datetime) => launch::datetime::parse_datetime(&string_datetime)?,
        None => {
            let naive_date = data.calendar().first_date();
            naive_date.and_hms(8, 0, 0)
        }
    };

    let compute_timer = SystemTime::now();

    let start_stop_area_uri = &config.start;
    let end_stop_area_uri = &config.end;

    let request_input = launch::stop_areas::make_query_stop_areas(
        model,
        &departure_datetime,
        start_stop_area_uri,
        end_stop_area_uri,
        &config.request_params,
    )?;
    let solve_result = solver.solve_request(data, model, &request_input, &config.comparator_type);

    let duration = compute_timer.elapsed().unwrap().as_millis();
    log::info!("Duration : {} ms", duration as f64);

    match &solve_result {
        Err(err) => {
            log::error!("Error while solving request : {}", err);
        }
        Ok(responses) => {
            for response in responses.iter() {
                debug!("{}", response.print(model)?);
            }
        }
    }

    let responses = solve_result?;
    Ok(responses)
}