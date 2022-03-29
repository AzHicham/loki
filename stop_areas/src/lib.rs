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

use launch::{
    config,
    datetime::DateTimeRepresent,
    loki::{
        self,
        models::{base_model::BaseModel, real_time_model::RealTimeModel, ModelRefs},
    },
    solver::Solver,
};

use loki::tracing::{debug, error, info};

use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::{Context, Error};

use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
    name = "loki_stop_areas",
    about = "Perform a public transport request between two stop areas.",
    rename_all = "snake_case"
)]
pub struct Options {
    /// path to the config file
    #[structopt(parse(from_os_str))]
    config_file: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// name of the start stop_area
    pub start_stop_area: String,

    /// name of the end stop_area
    pub end_stop_area: String,

    /// Datetime of the query , formatted like 20190628T163215
    /// This datetime will be interpreted as a departure time or arrival time,
    /// depending on the value of the datetime_represent parameter.
    /// If none is given, all queries will be made at 08:00:00 on the first
    /// valid day of the dataset
    pub datetime: Option<String>,

    /// "departure_datetime" can represent
    /// a DepartureAfter datetime
    /// or ArrivalBefore datetime
    #[serde(default)]
    pub datetime_represent: DateTimeRepresent,

    /// Which comparator to use for the request
    /// "basic" or "loads"
    #[serde(default)]
    pub comparator_type: config::ComparatorType,

    pub launch_params: config::LaunchParams,

    pub request_params: config::RequestParams,
}

pub fn run() -> Result<(), Error> {
    let options = Options::from_args();

    let config = read_config(&options.config_file)?;

    launch(config)?;
    Ok(())
}

pub fn read_config(config_file_path: &Path) -> Result<Config, Error> {
    let content = fs::read_to_string(&config_file_path)
        .with_context(|| format!("Error opening config file {:?}", &config_file_path))?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

pub fn launch(config: Config) -> Result<(BaseModel, Vec<loki::Response>), Error> {
    use loki::DataTrait;

    let (data, base_model) = launch::read(&config.launch_params)?;

    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(&base_model, &real_time_model);

    let datetime = match &config.datetime {
        Some(string_datetime) => launch::datetime::parse_datetime(string_datetime)?,
        None => {
            let naive_date = data.calendar().first_date();
            naive_date.and_hms(8, 0, 0)
        }
    };

    let datetime_represent = &config.datetime_represent;

    let compute_timer = SystemTime::now();

    let start_stop_area_uri = &config.start_stop_area;
    let end_stop_area_uri = &config.end_stop_area;

    let request_input = launch::stop_areas::make_query_stop_areas(
        &base_model,
        &datetime,
        start_stop_area_uri,
        end_stop_area_uri,
        &config.request_params,
    )?;
    let solve_result = solver.solve_journey_request(
        &data,
        &model_refs,
        &request_input,
        None,
        &config.comparator_type,
        datetime_represent,
    );

    let duration = compute_timer.elapsed().unwrap().as_millis();
    info!("Duration : {} ms", duration as f64);

    match &solve_result {
        Err(err) => {
            error!("Error while solving request. {}", err);
        }
        Ok(responses) => {
            for response in responses.iter() {
                debug!("{}", response.print(&model_refs)?);
            }
        }
    }

    let responses = solve_result?;

    Ok((base_model, responses))
}

#[cfg(test)]
mod tests {

    use super::read_config;
    use std::{path::PathBuf, str::FromStr};

    #[test]
    fn test_config() {
        let path = PathBuf::from_str(env!("CARGO_MANIFEST_DIR"))
            .unwrap()
            .join("config.toml");

        let read_result = read_config(&path);
        assert!(
            read_config(&path).is_ok(),
            "Error while reading config file {:?} : {:?}",
            &path,
            read_result
        );
    }
}
