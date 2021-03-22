use loki::config;
use loki::transit_model;
use loki::PositiveDuration;
use loki::{
    config::RequestParams, log::trace, response, traits::RequestInput, transit_model::Model,
};

use loki::traits;

use slog::slog_o;
use slog::Drain;
use slog_async::OverflowStrategy;

use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use chrono::NaiveDateTime;
use failure::{bail, Error};

use structopt::StructOpt;

pub mod stop_areas;

pub mod random;

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "snake_case")]
pub struct RequestConfig {
    /// penalty to apply to arrival time for each vehicle leg in a journey
    #[structopt(long, default_value = config::DEFAULT_LEG_ARRIVAL_PENALTY)]
    pub leg_arrival_penalty: PositiveDuration,

    /// penalty to apply to walking time for each vehicle leg in a journey
    #[structopt(long, default_value = config::DEFAULT_LEG_WALKING_PENALTY)]
    pub leg_walking_penalty: PositiveDuration,

    /// maximum number of vehicle legs in a journey
    #[structopt(long, default_value = config::DEFAULT_MAX_NB_LEGS)]
    pub max_nb_of_legs: u8,

    /// maximum duration of a journey
    #[structopt(long, default_value = config::DEFAULT_MAX_JOURNEY_DURATION)]
    pub max_journey_duration: PositiveDuration,
}

impl Default for RequestConfig {
    fn default() -> Self {
        let max_nb_of_legs: u8 = FromStr::from_str(config::DEFAULT_MAX_NB_LEGS).unwrap();
        Self {
            leg_arrival_penalty: FromStr::from_str(config::DEFAULT_LEG_ARRIVAL_PENALTY).unwrap(),
            leg_walking_penalty: FromStr::from_str(config::DEFAULT_LEG_WALKING_PENALTY).unwrap(),
            max_nb_of_legs,
            max_journey_duration: FromStr::from_str(config::DEFAULT_MAX_JOURNEY_DURATION).unwrap(),
        }
    }
}

impl Display for RequestConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "--leg_arrival_penalty {} --leg_walking_penalty {} --max_nb_of_legs {} --max_journey_duration {}",
                self.leg_arrival_penalty,
                self.leg_walking_penalty,
                self.max_nb_of_legs,
                self.max_journey_duration
        )
    }
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "snake_case")]
pub struct BaseOptions {
    #[structopt(flatten)]
    pub request_config: RequestConfig,

    /// directory of ntfs files to load
    #[structopt(short = "n", long = "ntfs")]
    pub ntfs_path: String,

    /// path to the passengers loads file
    #[structopt(short = "l", long = "loads_data")]
    pub loads_data_path: Option<String>,

    /// The default transfer duration between a stop point and itself
    #[structopt(long, default_value = config::DEFAULT_TRANSFER_DURATION)]
    pub default_transfer_duration: PositiveDuration,

    /// Departure datetime of the query, formatted like 20190628T163215
    /// If none is given, all queries will be made at 08:00:00 on the first
    /// valid day of the dataset
    #[structopt(long)]
    pub departure_datetime: Option<String>,

    /// Timetable implementation to use :
    /// "periodic" (default) or "daily"
    ///  or "loads_periodic" or "loads_daily"
    #[structopt(long, default_value = "loads_periodic")]
    pub data_implem: config::DataImplem,

    /// Type used for storage of criteria
    /// "classic" or "loads"
    #[structopt(long, default_value = "loads")]
    pub criteria_implem: config::CriteriaImplem,

    /// Which comparator to use for the request
    /// "basic" or "loads"
    #[structopt(long, default_value = "loads")]
    pub comparator_type: config::ComparatorType,
}

impl Display for BaseOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let departure_option = match &self.departure_datetime {
            Some(datetime) => format!("--departure_datetime {}", datetime),
            None => String::new(),
        };
        let loads_data_option = match & self.loads_data_path {
            Some(path) => format!("--loads_data {}", path),
            None => String::new(),
        };
        write!(
            f,
            "--ntfs {}  {} {} --data_implem {} --criteria_implem {} --comparator_type {} {}",
            self.ntfs_path,
            loads_data_option,
            departure_option,
            self.data_implem,
            self.criteria_implem,
            self.comparator_type,
            self.request_config.to_string()
        )
    }
}

pub fn init_logger() -> slog_scope::GlobalLoggerGuard {
    let decorator = slog_term::TermDecorator::new().stdout().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let mut builder = slog_envlogger::LogBuilder::new(drain).filter(None, slog::FilterLevel::Info);
    if let Ok(s) = std::env::var("RUST_LOG") {
        builder = builder.parse(&s);
    }
    let drain = slog_async::Async::new(builder.build())
        .chan_size(256) // Double the default size
        .overflow_strategy(OverflowStrategy::Block)
        .build()
        .fuse();
    let logger = slog::Logger::root(drain, slog_o!());

    let scope_guard = slog_scope::set_global_logger(logger);
    slog_stdlog::init().unwrap();
    scope_guard
}

pub fn make_query_stop_area(
    model: &transit_model::Model,
    from_stop_area: &str,
    to_stop_area: &str,
) -> Result<(Vec<String>, Vec<String>), Error> {
    use std::collections::BTreeSet;
    let mut start_sa_set = BTreeSet::new();
    let from_stop_area_idx = model
        .stop_areas
        .get_idx(from_stop_area)
        .ok_or_else(|| failure::format_err!("No stop area named `{}` found.", from_stop_area))?;
    start_sa_set.insert(from_stop_area_idx);
    let start_stop_points: Vec<String> = model
        .get_corresponding(&start_sa_set)
        .iter()
        .map(|idx| model.stop_points[*idx].id.clone())
        .collect();
    let mut end_sa_set = BTreeSet::new();

    let to_stop_area_idx = model
        .stop_areas
        .get_idx(to_stop_area)
        .ok_or_else(|| failure::format_err!("No stop area named `{}` found.", to_stop_area))?;
    end_sa_set.insert(to_stop_area_idx);
    let end_stop_points = model
        .get_corresponding(&end_sa_set)
        .iter()
        .map(|idx| model.stop_points[*idx].id.clone())
        .collect();
    Ok((start_stop_points, end_stop_points))
}

pub fn parse_datetime(string_datetime: &str) -> Result<NaiveDateTime, Error> {
    let try_datetime = NaiveDateTime::parse_from_str(string_datetime, "%Y%m%dT%H%M%S");
    match try_datetime {
        Ok(datetime) => Ok(datetime),
        Err(_) => bail!(
            "Unable to parse {} as a datetime. Expected format is 20190628T163215",
            string_datetime
        ),
    }
}

pub fn solve<'data, Data, Solver>(
    start_stop_area_uri: &str,
    end_stop_area_uri: &str,
    solver: &mut Solver, // &mut MultiCriteriaRaptor<DepartAfter<'data, Data>>,
    model: &Model,
    data: &'data Data,
    departure_datetime: &NaiveDateTime,
    options: &BaseOptions,
) -> Result<Vec<response::Response>, Error>
where
    Solver: traits::Solver<'data, Data>,
    Data: traits::DataWithIters,
{
    trace!(
        "Request start stop area : {}, end stop_area : {}",
        start_stop_area_uri,
        end_stop_area_uri
    );
    let (start_stop_point_uris, end_stop_point_uris) =
        make_query_stop_area(model, start_stop_area_uri, end_stop_area_uri)?;
    let departures_stop_point_and_fallback_duration = start_stop_point_uris
        .iter()
        .map(|uri| (uri.as_str(), PositiveDuration::zero()));

    let arrivals_stop_point_and_fallback_duration = end_stop_point_uris
        .iter()
        .map(|uri| (uri.as_str(), PositiveDuration::zero()));

    let request_config = &options.request_config;
    let params = RequestParams {
        leg_arrival_penalty: request_config.leg_arrival_penalty,
        leg_walking_penalty: request_config.leg_walking_penalty,
        max_nb_of_legs: request_config.max_nb_of_legs,
        max_journey_duration: request_config.max_journey_duration,
    };

    let request_input = RequestInput {
        departure_datetime: *departure_datetime,
        departures_stop_point_and_fallback_duration,
        arrivals_stop_point_and_fallback_duration,
        params,
    };

    let responses = solver.solve_request(data, model, request_input, &options.comparator_type)?;

    Ok(responses)
}
