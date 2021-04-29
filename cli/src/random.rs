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

use loki::{log::info, transit_model::Model};


use loki::traits;

use log::{error, trace};

use failure::Error;
use std::time::SystemTime;

use structopt::StructOpt;
use serde::{Serialize, Deserialize};
use loki::{DailyData, PeriodicData};
use loki::{LoadsDailyData, LoadsPeriodicData};

use std::{fs::File, io::BufReader};
use failure::{bail};

use crate::{parse_datetime, solve, BaseConfig};

#[derive(StructOpt)]
#[structopt(
    name = "loki_random",
    about = "Perform random public transport requests.",
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
#[structopt(
    rename_all = "snake_case"
)]
pub struct ConfigCreator {
    #[structopt(flatten)]
    pub base: BaseConfig,

}

#[derive(StructOpt)]
pub struct ConfigFile {
    /// path to the json config file
    #[structopt(parse(from_os_str))]
    file: std::path::PathBuf,
}
#[derive(Serialize, Deserialize)]
#[derive(StructOpt)]
#[structopt(
    rename_all = "snake_case"
)]
pub struct Config {
    #[serde(flatten)]
    #[structopt(flatten)]
    pub base: BaseConfig,

    #[serde(default = "default_nb_of_queries")]
    #[structopt(long, default_value = "10")]
    pub nb_queries: u32,
}


pub fn default_nb_of_queries() -> u32 {
    10
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
            let json_string = serde_json::to_string_pretty(&config_creator.base)?;

            println!("{}", json_string);

            Ok(())
        }
        Options::Launch(config) => {
            launch(config)?;
            Ok(())
        }
    }

}


pub fn read_config(config_file : & ConfigFile) -> Result<Config, Error> {
    let file = match File::open(&config_file.file) {
        Ok(file) => file,
        Err(e) => {
            bail!("Error opening config file {:?} : {}", &config_file.file, e)
        }
    };
    let reader = BufReader::new(file);
    let config : Config = serde_json::from_reader(reader)?;
    Ok(config)
}


pub fn launch(config :  Config) -> Result<(), Error> {


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

pub fn config_launch<Data>(config: Config) -> Result<(), Error>
where
    Data: traits::DataWithIters,
{
    let (data, model) = launch::read(
        &config.base.launch_params,
    )?;
    match config.base.launch_params.criteria_implem {
        config::CriteriaImplem::Basic => build_engine_and_solve::<
            Data,
            solver::BasicCriteriaSolver<Data>,
        >(&model, &data, &config),
        config::CriteriaImplem::Loads => build_engine_and_solve::<
            Data,
            solver::LoadsCriteriaSolver<Data>,
        >(&model, &data, &config),
    }
}

fn build_engine_and_solve<Data, Solver>(
    model: &Model,
    data: &Data,
    config: &Config,
) -> Result<(), Error>
where
    Data: traits::DataWithIters,
    Solver: solver::Solver<Data>,
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

    let nb_queries = config.nb_queries;
    use rand::prelude::{IteratorRandom, SeedableRng};
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(1);
    for _ in 0..nb_queries {
        let start_stop_area_uri = &model.stop_areas.values().choose(&mut rng).unwrap().id;
        let end_stop_area_uri = &model.stop_areas.values().choose(&mut rng).unwrap().id;

        let solve_result = solve(
            start_stop_area_uri,
            end_stop_area_uri,
            &mut solver,
            model,
            data,
            &departure_datetime,
            &config.base,
        );
        match solve_result {
            Err(err) => {
                error!("Error while solving request : {}", err);
            }
            Ok(responses) => {
                for response in responses.iter() {
                    trace!("{}", response.print(model)?);
                }
            }
        }
    }
    let duration = compute_timer.elapsed().unwrap().as_millis();

    info!(
        "Average duration per request : {} ms",
        (duration as f64) / (nb_queries as f64)
    );
    // info!(
    //     "Average nb of rounds : {}",
    //     (total_nb_of_rounds as f64) / (nb_queries as f64)
    // );
    info!("Nb of requests : {}", nb_queries);

    Ok(())
}
