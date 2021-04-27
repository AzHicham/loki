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

use launch::config;
use launch::loki;
use launch::solver;


use loki::{transit_model::Model};

use log::{error, info, trace};
use loki::{traits};

use serde::{Serialize, Deserialize};
use std::{fs::File, io::BufReader};
use failure::Error;
use failure::{bail, format_err};
use std::time::SystemTime;

use structopt::StructOpt;

use loki::{DailyData, PeriodicData};
use loki::{LoadsDailyData, LoadsPeriodicData};

use crate::{parse_datetime, solve, BaseConfig};

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
    ConfigFile(ConfigFile)
}

#[derive(StructOpt, Debug)]
#[structopt(
    rename_all = "snake_case"
)]
pub struct ConfigCreator {
    /// type of input data given (ntfs/gtfs)
    pub input_type : config::InputDataType, 

    /// directory containing ntfs/gtfs files to load
    pub input_path : String, 
  
    /// name of the start stop_area
    pub start: String,

    /// name of the end stop_area
    pub end: String,

}

#[derive(StructOpt)]
pub struct ConfigFile {
    /// path to the json config file
    #[structopt(parse(from_os_str))]
    file: std::path::PathBuf,
}
#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub base: BaseConfig,

    /// name of the start stop_area
    pub start: String,

    /// name of the end stop_area
    pub end: String,
}

pub fn run() -> Result<(), Error> {
    let options = Options::from_args(); 
    match options {
        Options::ConfigFile(config_file) => {
            let config = read_config(&config_file)?;
            launch(config)?;
            Ok(())
        },
        Options::CreateConfig(config_creator) => {
            let config = create_config(config_creator)?;
            let json_string = serde_json::to_string_pretty(&config)?;

            println!("{}", json_string);

            Ok(())
        }
    }

}

pub fn create_config(config_creator : ConfigCreator) -> Result<Config, Error> {
    let minimal_string = format!(r#" {{ 
        "input_data_path" : "{}", 
        "input_data_type" : "{}",
        "start" : "{}",
        "end" : "{}"
        }} "#,
        config_creator.input_path,
        config_creator.input_type,
        config_creator.start,
        config_creator.end,
    );
    let config : Config
     = serde_json::from_str(&minimal_string)?;
    Ok(config)
}






pub fn read_config(config_file : & ConfigFile) -> Result<Config, Error> {
    let file = match File::open(&config_file.file) {
        Ok(file) => file,
        Err(e) => {
            bail!("Error opening config file {:?} : {}", &config_file.file, e)
        }
    };
    let reader = BufReader::new(file);
    let config : Config = serde_json::from_reader(reader).map_err(|err| {
        format_err!("Could not read config file {:?} : {}", config_file.file, err)
    })?;
    Ok(config)
}

pub fn launch(config :  Config) -> Result<(Model, Vec<loki::Response>), Error> {

    match config.base.launch_params.data_implem {
        config::DataImplem::Periodic => {
            config_launch::<PeriodicData>(config)
        }
        config::DataImplem::Daily => {
            config_launch::<DailyData>(config)
        }
        config::DataImplem::LoadsPeriodic => {
            config_launch::<LoadsPeriodicData>(config)
        }
        config::DataImplem::LoadsDaily => {
            config_launch::<LoadsDailyData>(config)
        }
    }
}

fn config_launch<Data>(config: Config) -> Result<(Model, Vec<loki::Response>), Error>
where
    Data: traits::DataWithIters,
{
    let (data, model) = launch::read(
        &config.base.launch_params
    )?;
    let result = match config.base.launch_params.criteria_implem {
        config::CriteriaImplem::Basic => 
        {
            build_engine_and_solve::<Data,solver::BasicCriteriaSolver<'_, Data>>
                (&model, &data, &config)
        },
        config::CriteriaImplem::Loads => 
        {
            build_engine_and_solve::<Data, solver::LoadsCriteriaSolver<'_, Data>>
                (&model, &data, &config)
        },
    };

    result.map(|responses| (model, responses))
}

fn build_engine_and_solve<'data, Data, Solver>(
    model: &Model,
    data: &'data Data,
    config: &Config,
) -> Result<Vec<loki::Response>, Error>
where
    Data: traits::DataWithIters,
    Solver: solver::Solver<'data, Data>,
{
    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    let departure_datetime = match &config.base.departure_datetime {
        Some(string_datetime) => parse_datetime(&string_datetime)?,
        None => {
            let naive_date = data.calendar().first_date();
            naive_date.and_hms(8, 0, 0)
        }
    };

    let compute_timer = SystemTime::now();

    let start_stop_area_uri = &config.start;
    let end_stop_area_uri = &config.end;

    let solve_result = solve(
        start_stop_area_uri,
        end_stop_area_uri,
        &mut solver,
        model,
        data,
        &departure_datetime,
        &config.base,
    );

    let duration = compute_timer.elapsed().unwrap().as_millis();
    info!("Duration : {} ms", duration as f64);

    match &solve_result {
        Err(err) => {
            error!("Error while solving request : {}", err);
        }
        Ok(responses) => {
            for response in responses.iter() {
                trace!("{}", response.print(model)?);
            }
        }
    }

    solve_result
}
