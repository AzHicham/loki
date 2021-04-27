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

pub mod navitia_proto {
    include!(concat!(env!("OUT_DIR"), "/pbnavitia.rs"));
}

// pub mod navitia_proto;
mod response;

use launch::{config, solver};
use launch::loki;

use loki::transit_model;
use loki::{
    traits::{self, RequestInput},
};
use loki::{
    log::{error, info, warn},
    LoadsDailyData, LoadsPeriodicData,
};
use loki::{DailyData, PeriodicData, PositiveDuration};

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
    name = "loki_server",
    about = "Run loki server.",
    rename_all = "snake_case"
)]
pub enum Options {
    ConfigFile(ConfigFile),
}

#[derive(StructOpt)]
pub struct ConfigFile {
    /// path to the json config file
    #[structopt(parse(from_os_str))]
    file: PathBuf,
}

#[derive(Deserialize)]
pub struct Config {

    #[serde(flatten)]
    launch_params : config::LaunchParams,

    /// zmq socket to listen for protobuf requests
    /// that will be handled with "basic" comparator
    basic_requests_socket: String,

    /// zmq socket to listen for protobuf requests
    /// that will be handled with "loads" comparator
    loads_requests_socket: Option<String>,

    #[serde(default)]
    #[serde(flatten)]
    request_default_params : config::RequestParams,




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
    let config : Config = match options {
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
    match config.launch_params.data_implem {
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
    let (data, model) = launch::read::<Data>(
        &config.launch_params
    )?;

    match config.launch_params.criteria_implem {
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
    Solver: solver::Solver<'data, Data>,
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

fn solve<'data, Data, Solver: solver::Solver<'data, Data>>(
    socket: &zmq::Socket,
    zmq_message: &mut zmq::Message,
    data: &'data Data,
    model: &Model,
    solver: &mut Solver,
    config: &Config,
    comparator_type: config::ComparatorType,
) -> Result<Vec<loki::response::Response>, Error>
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
            let stop_point_uri =
                location_context
                    .place
                    .strip_prefix("stop_point:")
                    .or_else(|| {
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
            let stop_point_uri =
                location_context
                    .place
                    .strip_prefix("stop_point:")
                    .or_else(|| {
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
        departure_timestamp_u64, departure_datetime
    );

    let max_journey_duration = u32::try_from(journey_request.max_duration)
        .map(|duration| PositiveDuration::from_hms(0, 0, duration))
        .unwrap_or_else(|_| {
            warn!(
                "The max duration {} cannot be converted to a u32.\
                I'm gonna use the default {} as max duration",
                journey_request.max_duration, config.request_default_params.max_journey_duration
            );
            config.request_default_params.max_journey_duration.clone()
        });

    let max_nb_of_legs = u8::try_from(journey_request.max_transfers + 1).unwrap_or_else(|_| {
        warn!(
            "The max nb of transfers {} cannot be converted to a u8.\
                    I'm gonna use the default {} as the max nb of legs",
            journey_request.max_transfers, config.request_default_params.max_nb_of_legs
        );
        config.request_default_params.max_nb_of_legs
    });


    let request_input = RequestInput {
        departure_datetime,
        departures_stop_point_and_fallback_duration,
        arrivals_stop_point_and_fallback_duration,
        leg_arrival_penalty: config.request_default_params.leg_arrival_penalty,
        leg_walking_penalty: config.request_default_params.leg_walking_penalty,
        max_nb_of_legs,
        max_journey_duration,
    };

    let responses = solver.solve_request(data, model, request_input, &comparator_type)?;
    Ok(responses)
}

fn respond(
    solve_result: Result<Vec<loki::Response>, Error>,
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
