use launch::{
    config,
    loki::{
        self,
        models::{base_model::BaseModel, real_time_model::RealTimeModel, ModelRefs},
        request::generic_request,
        DailyData, PeriodicData, PeriodicSplitVjData, TransitData,
    },
    solver::Solver,
};
use loki::{tracing::debug, DataTrait};

use loki::timetables::{Timetables as TimetablesTrait, TimetablesIter};
use std::{convert::TryFrom, fs::File, io::BufReader, time::SystemTime};

use hdrhistogram::Histogram;
use rand::prelude::{IteratorRandom, SeedableRng};

use anyhow::{bail, Error};

use launch::datetime::DateTimeRepresent;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

fn main() {
    let _log_guard = launch::logger::init_logger();
    if let Err(err) = run() {
        eprintln!("{:?}", err);
        std::process::exit(1);
    }
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

    /// "departure_datetime" can represent
    /// a DepartureAfter datetime
    /// or ArrivalBefore datetime
    #[serde(default)]
    #[structopt(long, default_value)]
    pub datetime_represent: DateTimeRepresent,

    /// Which comparator to use for the request
    /// "basic" or "loads"
    #[serde(default)]
    #[structopt(long, default_value)]
    pub comparator_type: config::ComparatorType,

    /// Number of queries to perform
    #[serde(default = "default_nb_of_queries")]
    #[structopt(long, default_value = "10")]
    pub nb_queries: u32,

    /// Seed for random generator
    #[serde(default = "default_seed")]
    #[structopt(long, default_value = "0")]
    pub seed: u64,
}

pub fn default_nb_of_queries() -> u32 {
    10
}

pub fn default_seed() -> u64 {
    0
}

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

pub fn run() -> Result<(), Error> {
    let options = Options::from_args();
    match options {
        Options::ConfigFile(config_file) => {
            let config = read_config(&config_file)?;
            launch(&config)?;
            Ok(())
        }
        Options::CreateConfig(config_creator) => {
            let json_string = serde_json::to_string_pretty(&config_creator.config)?;

            println!("{}", json_string);

            Ok(())
        }
        Options::Launch(config) => {
            launch(&config)?;
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
    let config: Config = serde_json::from_reader(reader)?;
    Ok(config)
}

pub fn launch(config: &Config) -> Result<(), Error> {
    match config.launch_params.data_implem {
        config::DataImplem::Periodic => config_launch::<PeriodicData>(config),
        config::DataImplem::PeriodicSplitVj => config_launch::<PeriodicSplitVjData>(config),
        config::DataImplem::Daily => config_launch::<DailyData>(config),
    }
}

fn config_launch<Timetables>(config: &Config) -> Result<(), Error>
where
    Timetables: TimetablesTrait<
        Mission = generic_request::Mission,
        Position = generic_request::Position,
        Trip = generic_request::Trip,
    >,
    Timetables: for<'a> TimetablesIter<'a>,
    Timetables::Mission: 'static,
    Timetables::Position: 'static,
{
    let (data, model) = launch::read::<Timetables>(&config.launch_params)?;
    build_engine_and_solve(&model, &data, config)
}

fn build_engine_and_solve<Timetables>(
    base_model: &BaseModel,
    data: &TransitData<Timetables>,
    config: &Config,
) -> Result<(), Error>
where
    Timetables: TimetablesTrait<
        Mission = generic_request::Mission,
        Position = generic_request::Position,
        Trip = generic_request::Trip,
    >,
    Timetables: for<'a> TimetablesIter<'a>,
    Timetables::Mission: 'static,
    Timetables::Position: 'static,
{
    let real_time_model = RealTimeModel::new();
    let model_refs = ModelRefs::new(base_model, &real_time_model);

    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());

    let departure_datetime = match &config.departure_datetime {
        Some(string_datetime) => launch::datetime::parse_datetime(string_datetime)?,
        None => {
            let naive_date = data.calendar().first_date();
            naive_date.and_hms(8, 0, 0)
        }
    };

    let datetime_represent = &config.datetime_represent;

    let start_all = SystemTime::now();

    let nb_queries = config.nb_queries;

    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(config.seed);

    let mut histogram = Histogram::<u64>::new(3)?;
    for _ in 0..nb_queries {
        let start_stop_idx = base_model.stop_points().choose(&mut rng).unwrap();
        let end_stop_idx = base_model.stop_points().choose(&mut rng).unwrap();
        let start_stop_area_uri = base_model.stop_area_name(start_stop_idx);
        let end_stop_area_uri = base_model.stop_area_name(end_stop_idx);

        let request_input = launch::stop_areas::make_query_stop_areas(
            base_model,
            &departure_datetime,
            start_stop_area_uri,
            end_stop_area_uri,
            &config.request_params,
        )?;
        let before_solve = SystemTime::now();
        let solve_result = solver.solve_request(
            data,
            &model_refs,
            &request_input,
            None,
            &config.comparator_type,
            datetime_represent,
        );
        let solve_duration = before_solve.elapsed().unwrap().as_millis();
        let solve_duration_u64: u64 = TryFrom::try_from(solve_duration).unwrap();
        histogram.record(solve_duration_u64).unwrap();

        match solve_result {
            Err(err) => {
                log::error!("Error while solving request : {}", err);
            }
            Ok(responses) => {
                for response in &responses {
                    debug!("{}", response.print(&model_refs)?);
                }
            }
        }
    }
    let total_duration = start_all.elapsed().unwrap().as_millis();

    log::info!("Total duration : {} ms", total_duration);
    log::info!("Average duration per request : {} ms", histogram.mean());
    log::info!("Nb of requests : {}", nb_queries);
    log::info!("50'th percentile: {}", histogram.value_at_quantile(0.5));
    log::info!("70'th percentile: {}", histogram.value_at_quantile(0.7));
    log::info!("90'th percentile: {}", histogram.value_at_quantile(0.9));
    log::info!("99'th percentile: {}", histogram.value_at_quantile(0.99));
    log::info!("100'th percentile: {}", histogram.value_at_quantile(1.0));

    Ok(())
}
