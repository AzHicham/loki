use laxatips::{
    log::{debug, info, trace},
    response,
    transit_model::Model,
};
use laxatips::{transit_model, LoadsData};
use laxatips::{MultiCriteriaRaptor, PositiveDuration};
use laxatips::config::Implem;

use laxatips::traits;
use laxatips::config;

use log::warn;
use slog::slog_o;
use slog::Drain;
use slog_async::OverflowStrategy;

use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use chrono::NaiveDateTime;
use failure::{bail, Error};
use std::time::SystemTime;

use structopt::StructOpt;

pub mod stop_areas;

pub mod random;

const DEFAULT_LEG_ARRIVAL_PENALTY: &str = "00:02:00";
const DEFAULT_LEG_WALKING_PENALTY: &str = "00:02:00";
const DEFAULT_MAX_NB_LEGS: &str = "10";
const DEFAULT_MAX_JOURNEY_DURATION: &str = "24:00:00";

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "snake_case")]
pub struct RequestConfig {
    /// penalty to apply to arrival time for each vehicle leg in a journey
    #[structopt(long, default_value = DEFAULT_LEG_ARRIVAL_PENALTY)]
    pub leg_arrival_penalty: PositiveDuration,

    /// penalty to apply to walking time for each vehicle leg in a journey
    #[structopt(long, default_value = DEFAULT_LEG_WALKING_PENALTY)]
    pub leg_walking_penalty: PositiveDuration,

    /// maximum number of vehicle legs in a journey
    #[structopt(long, default_value = DEFAULT_MAX_NB_LEGS)]
    pub max_nb_of_legs: u8,

    /// maximum duration of a journey
    #[structopt(long, default_value = DEFAULT_MAX_JOURNEY_DURATION)]
    pub max_journey_duration: PositiveDuration,
}

impl Default for RequestConfig {
    fn default() -> Self {
        let max_nb_of_legs: u8 = FromStr::from_str(DEFAULT_MAX_NB_LEGS).unwrap();
        Self {
            leg_arrival_penalty: FromStr::from_str(DEFAULT_LEG_ARRIVAL_PENALTY).unwrap(),
            leg_walking_penalty: FromStr::from_str(DEFAULT_LEG_WALKING_PENALTY).unwrap(),
            max_nb_of_legs,
            max_journey_duration: FromStr::from_str(DEFAULT_MAX_JOURNEY_DURATION).unwrap(),
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
    pub loads_data_path: String,

    /// Departure datetime of the query, formatted like 20190628T163215
    /// If none is given, all queries will be made at 08:00:00 on the first
    /// valid day of the dataset
    #[structopt(long)]
    pub departure_datetime: Option<String>,

    /// Timetable implementation to use :
    /// "periodic" (default) or "daily"
    ///  or "loads_periodic" or "loads_daily"
    #[structopt(long, default_value = "periodic")]
    pub implem: Implem,

    /// Type of request to make :
    /// "classic" or "loads"
    #[structopt(long, default_value = "classic")]
    pub request_type: config::RequestType,
}

impl Display for BaseOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let departure_option = match &self.departure_datetime {
            Some(datetime) => format!("--departure_datetime {}", datetime),
            None => String::new(),
        };
        write!(
            f,
            "--ntfs {} --loads_data {} {} --implem {} --request_type {} {}",
            self.ntfs_path,
            self.loads_data_path,
            departure_option,
            self.implem,
            self.request_type,
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

pub fn build<Data>(ntfs_path: &str, loads_data_path: &str) -> Result<(Data, Model), Error>
where
    Data: traits::Data,
{
    let model = transit_model::ntfs::read(ntfs_path)?;
    info!("Transit model loaded");
    info!(
        "Number of vehicle journeys : {}",
        model.vehicle_journeys.len()
    );
    info!("Number of routes : {}", model.routes.len());

    let loads_data = LoadsData::new(&loads_data_path, &model).unwrap_or_else(|err| {
        warn!(
            "Error while reading the passenger loads file at {:?} : {:?}",
            &loads_data_path,
            err.source()
        );
        warn!("I'll use default loads.");
        LoadsData::empty()
    });

    let data_timer = SystemTime::now();
    let default_transfer_duration = PositiveDuration::from_hms(0, 0, 60);
    let data = Data::new(&model, &loads_data, default_transfer_duration);
    let data_build_time = data_timer.elapsed().unwrap().as_millis();
    info!("Data constructed");
    info!("Data build duration {} ms", data_build_time);
    info!("Number of missions {} ", data.nb_of_missions());
    info!("Number of trips {} ", data.nb_of_trips());
    info!(
        "Validity dates between {} and {}",
        data.calendar().first_date(),
        data.calendar().last_date()
    );
    Ok((data, model))
}

pub fn solve<'data, Data, R>(
    start_stop_area_uri: &str,
    end_stop_area_uri: &str,
    engine: &mut MultiCriteriaRaptor<R>, // &mut MultiCriteriaRaptor<DepartAfter<'data, Data>>,
    model: &Model,
    data: &'data Data,
    departure_datetime: &NaiveDateTime,
    leg_arrival_penalty: &PositiveDuration,
    leg_walking_penalty: &PositiveDuration,
    max_duration_to_arrival: &PositiveDuration,
    max_nb_of_legs: u8,
) -> Result<Vec<response::Journey<Data>>, Error>
where
    R: traits::RequestWithIters + traits::RequestIO<'data, Data>,
    Data: traits::DataWithIters<
        Position = R::Position,
        Mission = R::Mission,
        Stop = R::Stop,
        Trip = R::Trip,
    >,
    R::Criteria: Debug,
{
    trace!(
        "Request start stop area : {}, end stop_area : {}",
        start_stop_area_uri,
        end_stop_area_uri
    );
    let (start_stop_point_uris, end_stop_point_uris) =
        make_query_stop_area(model, start_stop_area_uri, end_stop_area_uri)?;
    let start_stops = start_stop_point_uris
        .iter()
        .map(|uri| (uri.as_str(), PositiveDuration::zero()));

    let end_stops = end_stop_point_uris
        .iter()
        .map(|uri| (uri.as_str(), PositiveDuration::zero()));

    let request = R::new(
        model,
        data,
        *departure_datetime,
        start_stops,
        end_stops,
        *leg_arrival_penalty,
        *leg_walking_penalty,
        *max_duration_to_arrival,
        max_nb_of_legs,
    )?;

    debug!("Start computing journey");
    let request_timer = SystemTime::now();
    engine.compute(&request);
    debug!(
        "Journeys computed in {} ms with {} rounds",
        request_timer.elapsed().unwrap().as_millis(),
        engine.nb_of_rounds()
    );
    debug!("Nb of journeys found : {}", engine.nb_of_journeys());
    debug!("Tree size : {:#}", engine.tree_size());
    let mut responses = Vec::new();
    for pt_journey in engine.responses() {
        let response = request.create_response(data, pt_journey);
        match response {
            Ok(journey) => {
                responses.push(journey);
            }
            Err(_) => {
                trace!("An error occured while converting an engine journey to response.");
            }
        };
    }

    Ok(responses)
}
