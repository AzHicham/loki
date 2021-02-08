use laxatips::{
    log::{info},
    transit_model::Model,
};
use laxatips::{ LoadsDailyData,  LoadsPeriodicData};
use laxatips::{
    DailyData, DepartAfter, LoadsDepartAfter, MultiCriteriaRaptor, PeriodicData, 
};

use laxatips::traits;

use log::warn;
use std::{fmt::Debug};
use traits::{RequestIO, RequestWithIters};

use failure::{bail, Error};
use std::time::SystemTime;

use structopt::StructOpt;

use laxatips_cli::{init_logger, parse_datetime, parse_duration, solve, build, BaseOptions};

fn main() {
    let _log_guard = init_logger();
    if let Err(err) = run() {
        for cause in err.iter_chain() {
            eprintln!("{}", cause);
        }
        std::process::exit(1);
    }
}


#[derive(StructOpt)]
#[structopt(name = "laxatips_cli", about = "Run laxatips from the command line.", rename_all = "snake_case")]
pub struct Options {
    
    #[structopt(flatten)]
    base : BaseOptions,

    /// Type of request to make :
    /// "classic" or "loads"
    #[structopt(long, default_value = "classic")]
    request_type: String,

    #[structopt(short = "n", long, default_value = "10")]
    nb_queries: u32,
}



fn run() -> Result<(), Error> {
    let options = Options::from_args();
    match options.base.implem.as_str() {
        "periodic" => launch::<PeriodicData>(options),
        "daily" => launch::<DailyData>(options),
        "loads_periodic" => launch::<LoadsPeriodicData>(options),
        "loads_daily" => launch::<LoadsDailyData>(options),
        _ => bail!(format!("Bad implem option : {}.", options.base.implem)),
    }
}

fn launch<Data>(options: Options) -> Result<(), Error>
where
    Data: traits::DataWithIters,
{
    let (data, model) = build(&options.base.ntfs_path, &options.base.loads_data_path)?;
    match options.request_type.as_str() {
        "classic" => build_engine_and_solve::<Data, DepartAfter<Data>>(&model, &data, &options),
        "loads" => build_engine_and_solve::<Data, LoadsDepartAfter<Data>>(&model, &data, &options),
        _ => {
            bail!("Invalid request_type : {}", options.request_type)
        }
    }
}

fn build_engine_and_solve<'data, Data, R>(
    model: &Model,
    data: &'data Data,
    options: &Options,
) -> Result<(), Error>
where
    R: RequestWithIters + RequestIO<'data, Data>,
    Data: traits::DataWithIters<
        Position = R::Position,
        Mission = R::Mission,
        Stop = R::Stop,
        Trip = R::Trip,
    >,
    R::Criteria : Debug
{
    let nb_of_stops = data.nb_of_stops();
    let nb_of_missions = data.nb_of_missions();

    let departure_datetime = match &options.base.departure_datetime {
        Some(string_datetime) => parse_datetime(&string_datetime)?,
        None => {
            let naive_date = data.calendar().first_date();
            naive_date.and_hms(8, 0, 0)
        }
    };


    let request_config = &options.base.request_config;

    let leg_arrival_penalty = parse_duration(&request_config.leg_arrival_penalty).unwrap();
    let leg_walking_penalty = parse_duration(&request_config.leg_walking_penalty).unwrap();
    let max_journey_duration = parse_duration(&request_config.max_journey_duration).unwrap();
    let max_nb_of_legs: u8 = request_config.max_nb_of_legs;
    let mut raptor = MultiCriteriaRaptor::<R>::new(nb_of_stops, nb_of_missions);

    let compute_timer = SystemTime::now();


    let mut total_nb_of_rounds = 0;
    let nb_queries = options.nb_queries;
    use rand::prelude::{IteratorRandom, SeedableRng};
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(1);
    for request_id in 0..nb_queries {
        let start_stop_area_uri = &model.stop_areas.values().choose(&mut rng).unwrap().id;
        let end_stop_area_uri = &model.stop_areas.values().choose(&mut rng).unwrap().id;

        let solve_result = solve(
            start_stop_area_uri,
            end_stop_area_uri,
            &mut raptor,
            model,
            data,
            &departure_datetime,
            &leg_arrival_penalty,
            &leg_walking_penalty,
            &max_journey_duration,
            max_nb_of_legs,
        );

        match solve_result {
            Ok(_responses) => {
                total_nb_of_rounds += raptor.nb_of_rounds();
            }
            Err(err) => {
                warn!(
                    "Error while solving request {} between stop_areas {} and {} : {}",
                    request_id,
                    start_stop_area_uri,
                    end_stop_area_uri,
                    err.to_string()
                );
            }
        }
    }
    let duration = compute_timer.elapsed().unwrap().as_millis();

    info!(
        "Average duration per request : {} ms",
        (duration as f64) / (nb_queries as f64)
    );
    info!(
        "Average nb of rounds : {}",
        (total_nb_of_rounds as f64) / (nb_queries as f64)
    );
    info!("Nb of requests : {}", nb_queries);
    
 
    Ok(())
}

