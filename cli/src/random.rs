use loki::{log::info, solver, transit_model::Model};

use loki::config;
use loki::traits;

use log::{error, trace};

use failure::Error;
use std::time::SystemTime;

use structopt::StructOpt;

use crate::{parse_datetime, solve, BaseOptions};

#[derive(StructOpt)]
#[structopt(
    name = "loki_random",
    about = "Perform random public transport requests.",
    rename_all = "snake_case"
)]
pub struct Options {
    #[structopt(flatten)]
    pub base: BaseOptions,

    #[structopt(short = "n", long, default_value = "10")]
    pub nb_queries: u32,
}

pub fn launch<Data>(options: Options) -> Result<(), Error>
where
    Data: traits::DataWithIters,
{
    let (data, model) = loki::launch_utils::read(
        &options.base.ntfs_path,
        & loki::config::InputType::Ntfs,
        options.base.loads_data_path.clone(),
        &options.base.default_transfer_duration,
    )?;
    match options.base.criteria_implem {
        config::CriteriaImplem::Basic => build_engine_and_solve::<
            Data,
            solver::BasicCriteriaSolver<'_, Data>,
        >(&model, &data, &options),
        config::CriteriaImplem::Loads => build_engine_and_solve::<
            Data,
            solver::LoadsCriteriaSolver<'_, Data>,
        >(&model, &data, &options),
    }
}

fn build_engine_and_solve<'data, Data, Solver>(
    model: &Model,
    data: &'data Data,
    options: &Options,
) -> Result<(), Error>
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

    let nb_queries = options.nb_queries;
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
            &options.base,
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
