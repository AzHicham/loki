use loki::{solver, transit_model::Model};

use loki::{config, traits};
use log::{error, info, trace};

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
        & loki::config::InputType::Ntfs,
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
