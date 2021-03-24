// Copyright  2020-2021, Kisio Digital and/or its affiliates. All rights reserved.
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

use loki::{solver, transit_model::Model};

use log::{error, info, trace};
use loki::{config, traits};

use std::fmt::{Debug, Display};

use failure::Error;
use std::time::SystemTime;

use structopt::StructOpt;

use crate::{parse_datetime, solve, BaseOptions};

#[derive(StructOpt, Debug)]
#[structopt(
    name = "loki_stop_areas",
    about = "Perform a public transport request between two stop areas.",
    rename_all = "snake_case"
)]
pub struct Options {
    #[structopt(flatten)]
    pub base: BaseOptions,

    #[structopt(long)]
    pub start: String,

    #[structopt(long)]
    pub end: String,
}

impl Display for Options {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "loki_cli {} --start {} --end {}",
            self.base.to_string(),
            self.start,
            self.end
        )
    }
}

pub fn launch<Data>(options: Options) -> Result<(Model, Vec<loki::Response>), Error>
where
    Data: traits::DataWithIters,
{
    let (data, model) = loki::launch_utils::read(
        &options.base.ntfs_path,
        &loki::config::InputType::Ntfs,
        options.base.loads_data_path.clone(),
        &options.base.default_transfer_duration,
    )?;
    let responses = match options.base.criteria_implem {
        config::CriteriaImplem::Basic => build_engine_and_solve::<
            Data,
            solver::BasicCriteriaSolver<'_, Data>,
        >(&model, &data, &options),
        config::CriteriaImplem::Loads => build_engine_and_solve::<
            Data,
            solver::LoadsCriteriaSolver<'_, Data>,
        >(&model, &data, &options),
    };
    responses.map(|responses| (model, responses))
}

fn build_engine_and_solve<'data, Data, Solver>(
    model: &Model,
    data: &'data Data,
    options: &Options,
) -> Result<Vec<loki::Response>, Error>
where
    Data: traits::DataWithIters,
    Solver: traits::Solver<'data, Data>,
{
    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    let departure_datetime = match &options.base.departure_datetime {
        Some(string_datetime) => parse_datetime(&string_datetime)?,
        None => {
            let naive_date = data.calendar().first_date();
            naive_date.and_hms(8, 0, 0)
        }
    };

    let compute_timer = SystemTime::now();

    let start_stop_area_uri = &options.start;
    let end_stop_area_uri = &options.end;

    let solve_result = solve(
        start_stop_area_uri,
        end_stop_area_uri,
        &mut solver,
        model,
        data,
        &departure_datetime,
        &options.base,
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
