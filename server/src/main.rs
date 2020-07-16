// pub mod navitia_proto {
//     include!(concat!(env!("OUT_DIR"), "/pbnavitia.rs"));
// }

pub mod navitia_proto;

use prost::Message;
use laxatips::log::{debug, info, warn, trace};
use laxatips::transit_model;
use laxatips::{
    DepartAfterRequest as EngineRequest, 
    MultiCriteriaRaptor, 
    PositiveDuration, 
    TransitData,
};

use std::fs;
use std::path::Path;

use slog::slog_o;
use slog::Drain;
use slog_async::OverflowStrategy;

use failure::{bail, format_err, Error};
use std::time::SystemTime;

use std::convert::TryFrom;

mod response;


const DEFAULT_MAX_DURATION : PositiveDuration = PositiveDuration{seconds : 24*60*60};
const DEFAULT_MAX_NB_LEGS : u8 = 10;

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

fn fill_engine_request_from_protobuf(
    proto_request : &navitia_proto::Request, 
    engine_request : & mut EngineRequest,
    model :& transit_model::Model,
    transit_data : & TransitData,
    default_max_duration : & PositiveDuration,
    default_max_nb_of_legs : u8,
) -> Result<(), Error>
{

    if proto_request.requested_api != (navitia_proto::Api::PtPlanner as i32) {
        let has_api = navitia_proto::Api::from_i32(proto_request.requested_api);
        if let Some(api) = has_api {
            bail!("Api requested is {:?} whereas only PtPlanner is supported", api);
        }
        else {
            bail!("Invalid \"requested_api\" provided {:?}", proto_request.requested_api);
        }
        
    }

    let journey_request = proto_request.journeys.as_ref().ok_or_else(|| {
        format_err!("request.journey should not be empty for api PtPlanner.")
    })?;

    println!("{:#?}", journey_request);

    let departure_stops_and_fallback_duration = journey_request.origin.iter().enumerate()
        .filter_map(|(idx, location_context)| {
            let stop_point_uri = location_context.place.as_str().trim_start_matches("stop_point:");
            let stop_idx = model
                .stop_points
                .get_idx(stop_point_uri)
                .or_else(|| {
                    warn!("The {}th departure stop point {} is not found in model.\
                            I ignore it.", 
                            idx,
                            stop_point_uri
                        );
                    None
                })?;
            let stop = transit_data
                .stop_point_idx_to_stop(&stop_idx)
                .or_else(|| {
                    warn!(
                        "The {}th departure stop point {} with idx {:?} is not found in transit_data.\
                        I ignore it",
                        idx,
                        stop_point_uri,
                        stop_idx
                    );
                    None
                })?;
            let duration = u32::try_from(location_context.access_duration)
                .map(|duration_u32| {
                    PositiveDuration {
                        seconds : duration_u32
                   }
                })
                .ok()
                .or_else(|| {
                    warn!(
                        "The {}th departure stop point {} has a fallback duration {}\
                        that cannot be converted to u32. I ignore it",
                        idx,
                        stop_point_uri,
                        location_context.access_duration
                    );
                    None
                })?;

            Some((stop.clone(), duration))

        });


    let arrival_stops_and_fallback_duration = journey_request.destination.iter().enumerate()
        .filter_map(|(idx, location_context)| {
            let stop_point_uri = location_context.place.as_str().trim_start_matches("stop_point:");
            let stop_idx = model
                .stop_points
                .get_idx(stop_point_uri)
                .or_else(|| {
                    warn!("The {}th arrival stop point {} is not found in model.\
                            I ignore it.", 
                            idx,
                            stop_point_uri
                        );
                    None
                })?;
            let stop = transit_data
                .stop_point_idx_to_stop(&stop_idx)
                .or_else(|| {
                    warn!(
                        "The {}th arrival stop point {} with idx {:?} is not found in transit_data.\
                        I ignore it",
                        idx,
                        stop_point_uri,
                        stop_idx
                    );
                    None
                })?;

            let duration = u32::try_from(location_context.access_duration)
                .map(|duration_u32| {
                    PositiveDuration {
                        seconds : duration_u32
                   }
                })
                .ok()
                .or_else(|| {
                    warn!(
                        "The {}th arrival stop point {} has a fallback duration {}\
                        that cannot be converted to u32. I ignore it",
                        idx,
                        stop_point_uri,
                        location_context.access_duration
                    );
                    None
                })?;
                Some((stop.clone(), duration))

        });


    let departure_timestamp_u64 = journey_request.datetimes.get(0).ok_or_else(|| {
        format_err!("Not datetime provided.")
    })?;
    let departure_timestamp_i64 = i64::try_from(*departure_timestamp_u64).map_err(|_| {
        format_err!("The datetime {} cannot be converted to a valid utc timestamp.", departure_timestamp_u64)
    })?;
    let departure_datetime = transit_data.calendar.timestamp_to_seconds_since_start(departure_timestamp_i64).ok_or_else(|| {
        format_err!("The timestamp {} is out of bound of the allowed dates.\
                     Allowed dates are between {} and {}.", 
                    departure_timestamp_i64,
                    transit_data.calendar.first_date(),
                    transit_data.calendar.last_date()
                )
    })?;


    let max_duration = u32::try_from(journey_request.max_duration).map(|duration| {
        PositiveDuration{ seconds : duration}
    })
    .unwrap_or_else(|_| {
        warn!("The max duration {} cannot be converted to a u32.\
                I'm gonna use the default {} as max duration", 
                journey_request.max_duration,
                default_max_duration
            );
        default_max_duration.clone()
    });
    
    // .unwrap_or_else(|| 
    //     warn!("The max duration {} cannot be converted to a u32.", journey_request.max_duration);

    // });
    let max_arrival_time = departure_datetime.clone() + max_duration;


    let max_nb_of_legs = u8::try_from(journey_request.max_transfers + 1)
        .unwrap_or_else(|_| {
            warn!("The max nb of transfers {} cannot be converted to a u8.\
                    I'm gonna use the default {} as the max nb of legs", 
                    journey_request.max_transfers,
                    default_max_duration
                );
                default_max_nb_of_legs
        });




    engine_request.update(
        departure_datetime.clone(),
        departure_stops_and_fallback_duration,
        arrival_stops_and_fallback_duration,
        PositiveDuration{seconds : 120}, //leg_arrival_penalty
        PositiveDuration{seconds : 120}, //leg_walking_penalty,
        max_arrival_time,
        max_nb_of_legs, //max_nb_of_legs,
    );

    Ok(())

}

fn run2() ->  Result<(), Error> {
    let request_filepath = Path::new("./tests/auvergne/with_resp/auvergne_03_request.proto");
    let proto_request_bytes = fs::read(request_filepath)?;
    let proto_request = navitia_proto::Request::decode(proto_request_bytes.as_slice())?;
    println!("Request : \n{:#?}", proto_request);

    let resp_filepath = Path::new("./tests/auvergne/with_resp/auvergne_03_resp.proto");
    let proto_resp_bytes = fs::read(resp_filepath)?;
    let proto_resp = navitia_proto::Response::decode(proto_resp_bytes.as_slice())?;
    println!("Response : \n {:#?}", proto_resp);

    Ok(())
}

fn run() ->  Result<(), Error>  {

    let ntfs_path = Path::new("/home/pascal/artemis/artemis_data/fr-auv/fusio/");

    let request_filepath = Path::new("./tests/auvergne/request6.proto");
    // let request_filepath = Path::new("./tests/request1.proto");

    
    let (model, transit_data) = read_ntfs(ntfs_path)?;

    

    let proto_request_bytes = fs::read(request_filepath)?;
    let proto_request = navitia_proto::Request::decode(proto_request_bytes.as_slice())?;


    

    let mut engine = MultiCriteriaRaptor::<EngineRequest>::new(transit_data.nb_of_stops());

    let mut engine_request = EngineRequest::new_default(&transit_data);


    fill_engine_request_from_protobuf(&proto_request, 
        & mut engine_request, 
        & model, 
        & transit_data, 
        &DEFAULT_MAX_DURATION, 
        DEFAULT_MAX_NB_LEGS
    )?;


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
             .create_response(pt_journey, &transit_data)
             .unwrap();            
        trace!("{}", response.print(&transit_data, &model)?);
    }

    // let proto_response = navitia_proto::Response::default();

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
    if let Err(err) = run2() {
        for cause in err.iter_chain() {
            eprintln!("{}", cause);
        }
        std::process::exit(1);
    }
}
