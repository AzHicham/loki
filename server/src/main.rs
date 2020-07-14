pub mod navitia_proto {
    include!(concat!(env!("OUT_DIR"), "/pbnavitia.rs"));
}

use prost::Message;
use laxatips::log::{debug, info, trace};
use laxatips::transit_model;
use laxatips::{
    DepartAfterRequest, MultiCriteriaRaptor, PositiveDuration, SecondsSinceDatasetStart,
    TransitData,
};

use std::fs;
use std::path::Path;

use slog::slog_o;
use slog::Drain;
use slog_async::OverflowStrategy;

use chrono::NaiveDateTime;
use failure::{bail, format_err, Error};
use std::time::SystemTime;

use std::convert::TryFrom;

fn read_ntfs(ntfs_path : & Path) -> Result<(transit_model::model::Model, TransitData), Error> {
    let model = transit_model::ntfs::read(ntfs_path)?;
    info!("Transit model loaded");
    info!(
        "Number of vehicle journeys : {}",
        model.vehicle_journeys.len()
    );
    info!("Number of routes : {}", model.routes.len());

    let data_timer = SystemTime::now();
    let default_transfer_duration = PositiveDuration { seconds: 60 };
    let transit_data = TransitData::new(&model, default_transfer_duration);
    let data_build_duration = data_timer.elapsed().unwrap().as_millis();
    info!("Data constructed in {} ms", data_build_duration);
    info!("Number of pattern {} ", transit_data.nb_of_patterns());
    info!("Number of timetables {} ", transit_data.nb_of_timetables());
    info!("Number of vehicles {} ", transit_data.nb_of_vehicles());
    info!(
        "Validity dates between {} and {}",
        transit_data.calendar.first_date(),
        transit_data.calendar.last_date()
    );

    Ok((model, transit_data))
}

fn run() ->  Result<(), Error>  {

    let ntfs_path = Path::new("/home/pascal/artemis/artemis_data/fr-auv/fusio/");

    let request_filepath = Path::new("./tests/auvergne/request6.proto");
    // let request_filepath = Path::new("./tests/request1.proto");

    
    let (model, transit_data) = read_ntfs(ntfs_path)?;

    

    let request_bytes = fs::read(request_filepath)?;
    let request = navitia_proto::Request::decode(request_bytes.as_slice())?;


    if request.requested_api != (navitia_proto::Api::PtPlanner as i32) {
        let has_api = navitia_proto::Api::from_i32(request.requested_api);
        if let Some(api) = has_api {
            bail!("Api requested is {:?} whereas only PtPlanner is supported", api);
        }
        else {
            bail!("Invalid \"requested_api\" provided {:?}", request.requested_api);
        }
        
    }

    let journey_request = &request.journeys.ok_or_else(|| {
        format_err!("request.journey should not be empty for api PtPlanner.")
    })?;

    println!("{:#?}", journey_request);

    let departure_stops_and_fallback_duration = journey_request.origin.iter()
        .map(|location_context| {
            let stop_point_uri = location_context.place.as_str().trim_start_matches("stop_point:");
            let stop_idx = model
                .stop_points
                .get_idx(stop_point_uri)
                .ok_or_else(|| format_err!("Departure stop point {} not found in model", stop_point_uri))?;
            let stop = transit_data
                .stop_point_idx_to_stop(&stop_idx)
                .ok_or_else(|| {
                    format_err!(
                        "Departure stop point {} with idx {:?} not found in transit_data.",
                        stop_point_uri,
                        stop_idx
                    )
                })?;
            let duration_u32 = u32::try_from(location_context.access_duration)?;
            let duration = PositiveDuration {
                 seconds : duration_u32
            };
            Ok((stop.clone(), duration))

        })
        .collect::<Result<Vec<_>, Error>>()?;

    let arrival_stops_and_fallback_duration = journey_request.destination.iter()
        .map(|location_context| {
            let stop_point_uri = location_context.place.as_str().trim_start_matches("stop_point:");
            let stop_idx = model
                .stop_points
                .get_idx(stop_point_uri)
                .ok_or_else(|| format_err!("Arrival stop point {} not found in model", stop_point_uri))?;
            let stop = transit_data
                .stop_point_idx_to_stop(&stop_idx)
                .ok_or_else(|| {
                    format_err!(
                        "Arrival stop point {} with idx {:?} not found in transit_data.",
                        stop_point_uri,
                        stop_idx
                    )
                })?;

            let duration_u32 = u32::try_from(location_context.access_duration)?;
            let duration = PositiveDuration {
                 seconds : duration_u32
            };
            Ok((stop.clone(), duration))

        })
        .collect::<Result<Vec<_>, Error>>()?;


    let departure_timestamp_u64 = journey_request.datetimes.get(0).ok_or_else(|| {
        format_err!("Not datetime provided.")
    })?;
    let departure_timestamp_i64 = i64::try_from(*departure_timestamp_u64).map_err(|_| {
        format_err!("The datetime {} cannot be converted to a valid utc timestamp.", departure_timestamp_u64)
    })?;
    let departure_datetime = transit_data.calendar.timestamp_to_seconds_since_start(departure_timestamp_i64).ok_or_else(|| {
        format_err!("The timestamp {} is out of bound of the allowed dates.", departure_timestamp_i64)
    })?;

    let engine_request = DepartAfterRequest::new(
        &transit_data,
        departure_datetime.clone(),
        departure_stops_and_fallback_duration,
        arrival_stops_and_fallback_duration,
        PositiveDuration{seconds : 120}, //leg_arrival_penalty
        PositiveDuration{seconds : 120}, //leg_walking_penalty,
        departure_datetime + PositiveDuration{seconds : 24*60*60},
        10, //max_nb_of_legs,
    );

    let mut engine = MultiCriteriaRaptor::<DepartAfterRequest>::new(transit_data.nb_of_stops());




    debug!("Start computing journey");
    let request_timer = SystemTime::now();
    engine.compute(&engine_request);
    debug!(
        "Journeys computed in {} ms with {} rounds",
        request_timer.elapsed().unwrap().as_millis(),
        engine.nb_of_rounds()
    );
    debug!("Nb of journeys found : {}", engine.nb_of_journeys());
    debug!("Tree size : {}", engine.tree_size());
    for pt_journey in engine.responses() {
        let response = engine_request
            .create_response_from_engine_result(pt_journey)
            .unwrap();

        trace!("{}", transit_data.print_response(&response, &model)?);
    }

    Ok(())
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
    if let Err(err) = run() {
        for cause in err.iter_chain() {
            eprintln!("{}", cause);
        }
        std::process::exit(1);
    }
}
