pub mod navitia_proto {
    include!(concat!(env!("OUT_DIR"), "/pbnavitia.rs"));
}

// pub mod navitia_proto;
mod response;

use laxatips::config;
use laxatips::transit_model;
use laxatips::{
    config::RequestParams,
    solver,
    traits::{self, RequestInput},
};
use laxatips::{
    log::{error, info, warn},
    LoadsDailyData, LoadsPeriodicData,
};
use laxatips::{DailyData, PeriodicData, PositiveDuration};

use prost::Message;
use structopt::StructOpt;
use transit_model::Model;

use std::{fs::File, io::BufReader, path::PathBuf};

use slog::slog_o;
use slog::Drain;
use slog_async::OverflowStrategy;

use failure::{bail, format_err, Error};

use std::convert::TryFrom;

use serde::Deserialize;

#[derive(StructOpt)]
#[structopt(
    name = "laxatips_server",
    about = "Run laxatips server.",
    rename_all = "snake_case"
)]
pub enum Options {
    Cli(Config),
    ConfigFile(ConfigFile),
}

#[derive(StructOpt)]
pub struct ConfigFile {
    /// path to the json config file
    #[structopt(parse(from_os_str))]
    file: PathBuf,
}

#[derive(StructOpt, Deserialize)]
pub struct Config {
    /// directory of ntfs files to load
    #[structopt(short = "n", long = "ntfs", parse(from_os_str))]
    ntfs_path: PathBuf,

    /// path to the passengers loads file
    #[structopt(short = "l", long = "loads_data", parse(from_os_str))]
    loads_data_path: PathBuf,

    /// zmq socket to listen for protobuf requests
    /// that will be handled with "basic" comparator
    #[structopt(long)]
    basic_requests_socket: String,

    /// zmq socket to listen for protobuf requests
    /// that will be handled with "loads" comparator
    #[structopt(long)]
    loads_requests_socket: Option<String>,

    /// Type used for storage of criteria
    /// "classic" or "loads"
    #[structopt(long, default_value = "loads")]
    criteria_implem: config::CriteriaImplem,

    /// Timetable implementation to use :
    /// "periodic" (default) or "daily"
    ///  or "loads_periodic" or "loads_daily"
    #[structopt(long, default_value = "loads_periodic")]
    data_implem: config::DataImplem,

    /// The default transfer duration between a stop point and itself
    #[structopt(long, default_value = config::DEFAULT_TRANSFER_DURATION)]
    #[serde(default = "config::default_transfer_duration")]
    default_transfer_duration: PositiveDuration,

    /// penalty to apply to arrival time for each vehicle leg in a journey
    #[structopt(long, default_value = config::DEFAULT_LEG_ARRIVAL_PENALTY)]
    #[serde(default = "config::default_leg_arrival_penalty")]
    leg_arrival_penalty: PositiveDuration,

    /// penalty to apply to walking time for each vehicle leg in a journey
    #[structopt(long, default_value = config::DEFAULT_LEG_WALKING_PENALTY)]
    #[serde(default = "config::default_leg_walking_penalty")]
    leg_walking_penalty: PositiveDuration,

    /// maximum number of vehicle legs in a journey
    #[structopt(long, default_value = config::DEFAULT_MAX_NB_LEGS)]
    #[serde(default = "config::default_max_nb_of_legs")]
    max_nb_of_legs: u8,

    /// maximum duration of a journey
    #[structopt(long, default_value = config::DEFAULT_MAX_JOURNEY_DURATION)]
    #[serde(default = "config::default_max_journey_duration")]
    max_journey_duration: PositiveDuration,
}

fn main() {
    let _log_guard = init_logger();
    if let Err(err) = launch_server() {
        for cause in err.iter_chain() {
            eprintln!("{}", cause);
        }
        std::process::exit(1);
    }
}

fn init_logger() -> slog_scope::GlobalLoggerGuard {
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

fn launch_server() -> Result<(), Error> {
    let options = Options::from_args();
    let config = match options {
        Options::Cli(config) => config,
        Options::ConfigFile(config_file) => {
            let file = match File::open(&config_file.file) {
                Ok(file) => file,
                Err(e) => {
                    bail!("Error opening config file {:?} : {}", &config_file.file, e)
                }
            };
            let reader = BufReader::new(file);
            let result = serde_json::from_reader(reader);
            match result {
                Ok(config) => config,
                Err(e) => bail!("Error reading config file {:?} : {}", &config_file.file, e),
            }
        }
    };
    match config.data_implem {
        config::DataImplem::Periodic => launch::<PeriodicData>(config),
        config::DataImplem::Daily => launch::<DailyData>(config),
        config::DataImplem::LoadsPeriodic => launch::<LoadsPeriodicData>(config),
        config::DataImplem::LoadsDaily => launch::<LoadsDailyData>(config),
    }
}

fn launch<Data>(config: Config) -> Result<(), Error>
where
    Data: traits::DataWithIters,
{
    let (data, model) = laxatips::launch_utils::read_ntfs::<Data, _, _>(
        &config.ntfs_path,
        &config.loads_data_path,
        &config.default_transfer_duration,
    )?;

    match config.criteria_implem {
        config::CriteriaImplem::Basic => {
            server_loop::<Data, solver::BasicCriteriaSolver<'_, Data>>(&model, &data, &config)
        }
        config::CriteriaImplem::Loads => {
            server_loop::<Data, solver::LoadsCriteriaSolver<'_, Data>>(&model, &data, &config)
        }
    }
}

fn server_loop<'data, Data, Solver>(
    model: &Model,
    data: &'data Data,
    config: &Config,
) -> Result<(), Error>
where
    Data: traits::DataWithIters,
    Solver: traits::Solver<'data, Data>,
{
    let mut solver = Solver::new(data.nb_of_stops(), data.nb_of_missions());
    let context = zmq::Context::new();
    let basic_requests_socket = context
        .socket(zmq::REP)
        .map_err(|err| format_err!("Could not create a socket. Error : {}", err))?;

    basic_requests_socket
        .bind(&config.basic_requests_socket)
        .map_err(|err| {
            format_err!(
                "Could not bind socket {}. Error : {}",
                config.basic_requests_socket,
                err
            )
        })?;

    let loads_requests_socket = context
        .socket(zmq::REP)
        .map_err(|err| format_err!("Could not create a socket. Error : {}", err))?;

    if let Some(socket) = &config.loads_requests_socket {
        loads_requests_socket
            .bind(socket)
            .map_err(|err| format_err!("Could not bind socket {}. Error : {}", socket, err))?;
    }

    info!("Ready to receive requests");

    let mut zmq_message = zmq::Message::new();
    let mut response_bytes: Vec<u8> = Vec::new();
    loop {
        let mut items = [
            basic_requests_socket.as_poll_item(zmq::POLLIN),
            loads_requests_socket.as_poll_item(zmq::POLLIN),
        ];
        zmq::poll(&mut items, -1)
            .map_err(|err| format_err!("Error while polling zmq sockets : {}", err))?;

        if items[0].is_readable() {
            let socket = &basic_requests_socket;
            let comparator_type = config::ComparatorType::Basic;
            let solve_result = solve(
                socket,
                &mut zmq_message,
                data,
                model,
                &mut solver,
                config,
                comparator_type,
            );
            let result = respond(solve_result, model, &mut response_bytes, socket);
            result
                .err()
                .map(|err| error!("Error while sending zmq response : {}", err));
        }

        if items[1].is_readable() {
            let socket = &loads_requests_socket;
            let comparator_type = config::ComparatorType::Loads;
            let solve_result = solve(
                socket,
                &mut zmq_message,
                data,
                model,
                &mut solver,
                config,
                comparator_type,
            );
            let result = respond(solve_result, model, &mut response_bytes, socket);
            result
                .err()
                .map(|err| error!("Error while sending zmq response : {}", err));
        }
    }
}

fn solve<'data, Data, Solver: traits::Solver<'data, Data>>(
    socket: &zmq::Socket,
    zmq_message: &mut zmq::Message,
    data: &'data Data,
    model: &Model,
    solver: &mut Solver,
    config: &Config,
    comparator_type: config::ComparatorType,
) -> Result<Vec<laxatips::response::Response>, Error>
where
    Data: traits::DataWithIters,
{
    let proto_request = decode_zmq_message(socket, zmq_message)?;
    info!(
        "Received request {:?} of type {:?}",
        proto_request.request_id, comparator_type
    );

    if proto_request.requested_api != (navitia_proto::Api::PtPlanner as i32) {
        let has_api = navitia_proto::Api::from_i32(proto_request.requested_api);
        if let Some(api) = has_api {
            bail!(
                "Api requested is {:?} whereas only PtPlanner is supported",
                api
            );
        } else {
            bail!(
                "Invalid \"requested_api\" provided {:?}",
                proto_request.requested_api
            );
        }
    }

    let journey_request = proto_request
        .journeys
        .as_ref()
        .ok_or_else(|| format_err!("request.journey should not be empty for api PtPlanner."))?;

    // println!("{:#?}", journey_request);
    let departures_stop_point_and_fallback_duration = journey_request
        .origin
        .iter()
        .enumerate()
        .filter_map(|(idx, location_context)| {
            let duration = u32::try_from(location_context.access_duration)
                .map(|duration_u32| PositiveDuration::from_hms(0, 0, duration_u32))
                .ok()
                .or_else(|| {
                    warn!(
                        "The {}th departure stop point {} has a fallback duration {} \
                        that cannot be converted to u32. I ignore it",
                        idx, location_context.place, location_context.access_duration
                    );
                    None
                })?;
            let stop_point_uri = location_context.place.strip_prefix("stop_point:")
                .or_else( || {
                    warn!(
                        "The {}th arrival stop point has an uri {} \
                        that doesn't start with `stop_point:`. I ignore it",
                        idx, location_context.place,
                    );
                    None
                })?;
            // let trimmed = location_context.place.trim_start_matches("stop_point:");
            // let stop_point_uri = format!("StopPoint:{}", trimmed);
            // let stop_point_uri = location_context.place.clone();
            Some((stop_point_uri, duration))
        });

    let arrivals_stop_point_and_fallback_duration = journey_request
        .destination
        .iter()
        .enumerate()
        .filter_map(|(idx, location_context)| {
            let duration = u32::try_from(location_context.access_duration)
                .map(|duration_u32| PositiveDuration::from_hms(0, 0, duration_u32))
                .ok()
                .or_else(|| {
                    warn!(
                        "The {}th arrival stop point {} has a fallback duration {}\
                        that cannot be converted to u32. I ignore it",
                        idx, location_context.place, location_context.access_duration
                    );
                    None
                })?;
            let stop_point_uri = location_context.place.strip_prefix("stop_point:")
                .or_else( || {
                    warn!(
                        "The {}th arrival stop point has an uri {} \
                        that doesn't start with `stop_point:`. I ignore it",
                        idx, location_context.place,
                    );
                    None
                })?;
            // let trimmed = location_context.place.trim_start_matches("stop_point:");
            // let stop_point_uri = format!("StopPoint:{}", trimmed);
            // let stop_point_uri = location_context.place.clone();
            Some((stop_point_uri, duration))
        });

    let departure_timestamp_u64 = journey_request
        .datetimes
        .get(0)
        .ok_or_else(|| format_err!("Not departure datetime provided."))?;
    let departure_timestamp_i64 = i64::try_from(*departure_timestamp_u64).map_err(|_| {
        format_err!(
            "The departure datetime {} cannot be converted to a valid i64 timestamp.",
            departure_timestamp_u64
        )
    })?;
    let departure_datetime = chrono::NaiveDateTime::from_timestamp(departure_timestamp_i64, 0);

    info!(
        "Requested timestamp {}, datetime {}",
        departure_timestamp_u64,
        chrono::NaiveDateTime::from_timestamp(departure_timestamp_i64, 0)
    );

    let max_journey_duration = u32::try_from(journey_request.max_duration)
        .map(|duration| PositiveDuration::from_hms(0, 0, duration))
        .unwrap_or_else(|_| {
            warn!(
                "The max duration {} cannot be converted to a u32.\
                I'm gonna use the default {} as max duration",
                journey_request.max_duration, config.max_journey_duration
            );
            config.max_journey_duration.clone()
        });

    let max_nb_of_legs = u8::try_from(journey_request.max_transfers + 1).unwrap_or_else(|_| {
        warn!(
            "The max nb of transfers {} cannot be converted to a u8.\
                    I'm gonna use the default {} as the max nb of legs",
            journey_request.max_transfers, config.max_nb_of_legs
        );
        config.max_nb_of_legs
    });

    let params = RequestParams {
        leg_arrival_penalty: config.leg_arrival_penalty,
        leg_walking_penalty: config.leg_walking_penalty,
        max_nb_of_legs,
        max_journey_duration,
    };

    let request_input = RequestInput {
        departure_datetime,
        departures_stop_point_and_fallback_duration,
        arrivals_stop_point_and_fallback_duration,
        params,
    };

    let responses = solver.solve_request(data, model, request_input, &comparator_type)?;
    Ok(responses)
}

fn respond(
    solve_result: Result<Vec<laxatips::Response>, Error>,
    model: &Model,
    response_bytes: &mut Vec<u8>,
    socket: &zmq::Socket,
) -> Result<(), Error> {
    let proto_response = match solve_result {
        Result::Err(err) => {
            error!("Error while solving request : {}", err);
            make_error_response(err)
        }
        Ok(journeys) => {
            let response_result = response::make_response(journeys, model);
            match response_result {
                Result::Err(err) => {
                    error!(
                        "Error while encoding protobuf response for request : {}",
                        err
                    );
                    make_error_response(err)
                }
                Ok(resp) => resp,
            }
        }
    };
    response_bytes.clear();

    proto_response.encode(response_bytes).map_err(|err| {
        format_err!(
            "Could not encode protobuf response into a zmq message: \n {}",
            err
        )
    })?;

    info!("Sending protobuf response. ");

    socket
        .send(&*response_bytes, 0)
        .map_err(|err| format_err!("Could not send zmq response : \n {}", err))?;

    Ok(())
}

fn make_error_response(error: Error) -> navitia_proto::Response {
    let mut proto_response = navitia_proto::Response::default();
    proto_response.set_response_type(navitia_proto::ResponseType::NoSolution);
    let mut proto_error = navitia_proto::Error::default();
    proto_error.set_id(navitia_proto::error::ErrorId::InternalError);
    proto_error.message = Some(format!("{}", error));
    proto_response.error = Some(proto_error);
    proto_response
}

fn decode_zmq_message(
    socket: &zmq::Socket,
    zmq_message: &mut zmq::Message,
) -> Result<navitia_proto::Request, Error> {
    socket
        .recv(zmq_message, 0)
        .map_err(|err| format_err!("Could not receive zmq message : \n {}", err))?;
    use std::ops::Deref;
    navitia_proto::Request::decode((*zmq_message).deref())
        .map_err(|err| format_err!("Could not decode zmq message into protobuf: \n {}", err))
}
