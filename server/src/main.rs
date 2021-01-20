pub mod navitia_proto {
    include!(concat!(env!("OUT_DIR"), "/pbnavitia.rs"));
}

// pub mod navitia_proto;
mod response;

use laxatips::log::{debug, error, info, trace, warn}; 
use laxatips::traits::{Indices, Input, NetworkStructure, Response, TimeQueries, TransitIters, TransitTypes};
use laxatips::transit_model;
use laxatips::{
    DailyData, DepartAfterRequest, MultiCriteriaRaptor, PeriodicData, PositiveDuration,
};

use prost::Message;
use structopt::StructOpt;
use transit_model::Model;

use std::{hash::Hash, path::{Path, PathBuf}};

use slog::slog_o;
use slog::Drain;
use slog_async::OverflowStrategy;

use failure::{bail, format_err, Error};
use std::time::SystemTime;

use std::convert::TryFrom;

const DEFAULT_MAX_DURATION: PositiveDuration = PositiveDuration::from_hms(24, 0, 0);
const DEFAULT_MAX_NB_LEGS: u8 = 10;

#[derive(StructOpt)]
#[structopt(name = "laxatips_server", about = "Run laxatips server.")]
struct Options {
    /// directory of ntfs files to load
    #[structopt(short = "n", long = "ntfs", parse(from_os_str))]
    input: PathBuf,

    /// penalty to apply to arrival time for each vehicle leg in a journey
    #[structopt(short = "s", long)]
    socket: String,

    /// Timetable implementation to use : "periodic" (default) or "daily"
    #[structopt(long, default_value = "periodic")]
    implem: String,
}

fn read_ntfs<Data>(ntfs_path: &Path) -> Result<(Data, Model), Error> 
where Data : Input
{
    let model = transit_model::ntfs::read(ntfs_path)?;
    info!("Transit model loaded");
    info!(
        "Number of vehicle journeys : {}",
        model.vehicle_journeys.len()
    );
    info!("Number of routes : {}", model.routes.len());

    let data_timer = SystemTime::now();
    let default_transfer_duration = PositiveDuration::from_hms(0, 0, 60);
    let data = Data::new(&model, default_transfer_duration);
    let data_build_duration = data_timer.elapsed().unwrap().as_millis();
    info!("Data constructed in {} ms", data_build_duration);
    info!(
        "Validity dates between {} and {}",
        data.calendar().first_date(),
        data.calendar().last_date()
    );

    Ok((data, model))
}

fn make_engine_request_from_protobuf<'data, Data>(
    proto_request: &navitia_proto::Request,
    data : & 'data Data,
    model : & Model,
    default_max_duration: &PositiveDuration,
    default_max_nb_of_legs: u8,
) -> Result<DepartAfterRequest<'data,  Data>, Error> 
where Data : TransitTypes + Input
{
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
            let stop_point_uri = location_context
                .place
                .as_str()
                .trim_start_matches("stop_point:");            
            let duration = u32::try_from(location_context.access_duration)
                .map(|duration_u32| PositiveDuration::from_hms(0, 0, duration_u32))
                .ok()
                .or_else(|| {
                    warn!(
                        "The {}th departure stop point {} has a fallback duration {} \
                        that cannot be converted to u32. I ignore it",
                        idx, stop_point_uri, location_context.access_duration
                    );
                    None
                })?;

            Some((stop_point_uri, duration))
        });

    let arrivals_stop_point_and_fallback_duration = journey_request
        .destination
        .iter()
        .enumerate()
        .filter_map(|(idx, location_context)| {
            let stop_point_uri = location_context
                .place
                .as_str()
                .trim_start_matches("stop_point:");

            let duration = u32::try_from(location_context.access_duration)
                .map(|duration_u32| PositiveDuration::from_hms(0, 0, duration_u32))
                .ok()
                .or_else(|| {
                    warn!(
                        "The {}th arrival stop point {} has a fallback duration {}\
                        that cannot be converted to u32. I ignore it",
                        idx, stop_point_uri, location_context.access_duration
                    );
                    None
                })?;
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

    let max_duration_to_arrival = u32::try_from(journey_request.max_duration)
        .map(|duration| PositiveDuration::from_hms(0, 0, duration))
        .unwrap_or_else(|_| {
            warn!(
                "The max duration {} cannot be converted to a u32.\
                I'm gonna use the default {} as max duration",
                journey_request.max_duration, default_max_duration
            );
            default_max_duration.clone()
        });

    let max_nb_legs = u8::try_from(journey_request.max_transfers + 1).unwrap_or_else(|_| {
        warn!(
            "The max nb of transfers {} cannot be converted to a u8.\
                    I'm gonna use the default {} as the max nb of legs",
            journey_request.max_transfers, default_max_duration
        );
        default_max_nb_of_legs
    });

    let engine_request = DepartAfterRequest::new(model,
        data, 
        departure_datetime, 
        departures_stop_point_and_fallback_duration, 
        arrivals_stop_point_and_fallback_duration, 
        PositiveDuration::from_hms(0, 2, 0), //leg_arrival_penalty
        PositiveDuration::from_hms(0, 2, 0), //leg_walking_penalty,
        max_duration_to_arrival, 
        max_nb_legs
    )?;

    Ok(engine_request)
}

fn solve_protobuf<'data, Data>(
    model : & Model,
    proto_request: &navitia_proto::Request,
    data : & 'data Data,
    engine: &mut MultiCriteriaRaptor<DepartAfterRequest<'data,  Data>>,
) -> Result<navitia_proto::Response, Error> 
where 
Data: TimeQueries + NetworkStructure + Input + Indices + Response + for<'a> TransitIters<'a>,
Data::Mission: Hash + PartialEq + Eq,
{
    let request = make_engine_request_from_protobuf(
        &proto_request,
        data,
        model,
        &DEFAULT_MAX_DURATION,
        DEFAULT_MAX_NB_LEGS,
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
    debug!("Tree size : {}", engine.tree_size());
    for pt_journey in engine.responses() {
        let response = request.create_response(data, pt_journey);
        match response {
            Ok(journey) => {
                trace!("{}", journey.print(data, model)?);
            }
            Err(_) => {
                trace!("An error occured while converting an engine journey to response."); 
            }
        };
    }



    let journeys_iter = engine.responses().filter_map(|pt_journey| {
        request
            .create_response(data, pt_journey)
            .ok()
    });

    response::make_response(journeys_iter, data, model)

}

fn solve<'data, Data>(
    data : & 'data Data,
    model : & Model,
    engine: &mut MultiCriteriaRaptor<DepartAfterRequest<'data,  Data>>,
    socket: &zmq::Socket,
    zmq_message: &mut zmq::Message,
    response_bytes: &mut Vec<u8>,
) -> Result<(), Error> 
where
    Data: TimeQueries + NetworkStructure + Input + Indices + Response + for<'a> TransitIters<'a>,
    Data::Mission: Hash + PartialEq + Eq,
{
    socket
        .recv(zmq_message, 0)
        .map_err(|err| format_err!("Could not receive zmq message : \n {}", err))?;
    use std::ops::Deref;
    let proto_request = navitia_proto::Request::decode((*zmq_message).deref())
        .map_err(|err| format_err!("Could not decode zmq message into protobuf: \n {}", err))?;

    info!("Received request {:?}", proto_request.request_id);

    let solve_result = solve_protobuf(
        model, 
        &proto_request,
        data,
        engine,
    );

    let proto_response = match solve_result {
        Result::Err(err) => {
            error!(
                "Error while solving request {:?} : \n {}",
                proto_request.request_id, err
            );

            let mut proto_response = navitia_proto::Response::default();
            proto_response.set_response_type(navitia_proto::ResponseType::NoSolution);
            let mut proto_error = navitia_proto::Error::default();
            proto_error.set_id(navitia_proto::error::ErrorId::InternalError);
            proto_error.message = Some(format!("{}", err));
            proto_response.error = Some(proto_error);
            proto_response
        },
        Ok(proto_response) => {
            proto_response
        }
    };
    response_bytes.clear();

    proto_response
        .encode(response_bytes)
        .map_err(|err| format_err!("Could not encode protobuf into zmq message: \n {}", err))?;

    info!(
        "Sending response for request {:?}",
        proto_request.request_id
    );

    socket
        .send(&*response_bytes, 0)
        .map_err(|err| format_err!("Could not send zmq response : \n {}", err))?;

    Ok(())
}

fn server() -> Result<(), Error> {

    let options = Options::from_args();
    if options.implem == "periodic" {
        launch::<PeriodicData>(options)?;
    } else if options.implem == "daily" {
        launch::<DailyData>(options)?;
    } else {
        bail!(format!(
            "Bad implem option : {} .\n Allowed options are `periodic` or  `daily`",
            options.implem
        ));
    }
    Ok(())

}

fn launch<Data>(options: Options) -> Result<(), Error>
where
    Data: TransitTypes
        + Input
        + Indices
        + TimeQueries
        + Response
        + NetworkStructure
        + for<'a> TransitIters<'a>,
    Data::Mission: Hash + PartialEq + Eq,
{
    let context = zmq::Context::new();
    let responder = context.socket(zmq::REP).unwrap();

    responder
        .bind(&options.socket)
        .map_err(|err| format_err!("Could not bind socket {}. Error : {}", options.socket, err))?;

    let ntfs_path = options.input;

    let (data, model) = read_ntfs::<Data>(&ntfs_path)?;
    let mut engine = MultiCriteriaRaptor::<DepartAfterRequest<Data>>::new(data.nb_of_stops());

    let mut zmq_message = zmq::Message::new();
    let mut response_bytes: Vec<u8> = Vec::new();

    loop {
        let solve_result = solve(
            &data,
            & model,
            &mut engine,
            &responder,
            &mut zmq_message,
            &mut response_bytes,
        );
        if let Err(err) = solve_result {
            error!("Failed to solve request : ");
            for cause in err.iter_chain() {
                error!("{}", cause);
            }
        }
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

fn main() {
    let _log_guard = init_logger();
    if let Err(err) = server() {
        for cause in err.iter_chain() {
            eprintln!("{}", cause);
        }
        std::process::exit(1);
    }
}
