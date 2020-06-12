

mod transit_data;
mod engine;
mod request;



use transit_model;
use std::path::PathBuf;

use transit_data::time::{ PositiveDuration, SecondsSinceDatasetStart};
use log::{info, error};

use slog::slog_o;
use slog::Drain;
use slog_async::OverflowStrategy;

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
    // let input_dir = PathBuf::from("tests/fixtures/small_ntfs/");
    // let start_stop_point_uri = "sp_1";
    // let end_stop_point_uri = "sp_3";

    let input_dir = PathBuf::from("tests/fixtures/ntfs_rennes/");
    let start_stop_point_uri = "SAR:SP:1001";
    let end_stop_point_uri = "SAR:SP:6006";

    let model = transit_model::ntfs::read(input_dir).unwrap();
    info!("Transit model loaded");
    info!("Number of vehicle journeys : {}", model.vehicle_journeys.len());
    info!("Number of routes : {}", model.routes.len());
    // info!("{:#?}", model.stop_points.values().take(2).collect::<Vec<_>>());

    let transit_data = transit_data::data::TransitData::new(&model, PositiveDuration::zero());

    info!("Data constructed");
    info!("Number of pattern {} ", transit_data.nb_of_patterns());
    info!("Number of timetables {} ", transit_data.nb_of_timetables());
    info!("Number of vehicles {} ", transit_data.nb_of_vehicles());

    let start_stop_point_idx = model.stop_points.get_idx(&start_stop_point_uri).unwrap_or_else( || {
        error!("Start stop point {} not found in model", start_stop_point_uri);
        panic!();
    });
    let end_stop_point_idx = model.stop_points.get_idx(&end_stop_point_uri).unwrap_or_else( || {
        error!("End stop point {} not found in model", end_stop_point_uri);
        panic!();
    });

    let start_stop = transit_data.stop_point_idx_to_stop(&start_stop_point_idx).unwrap().clone();
    let end_stop = transit_data.stop_point_idx_to_stop(&end_stop_point_idx).unwrap().clone();

    let start_stops = vec![(start_stop, PositiveDuration::zero())];
    let end_stops = vec![(end_stop, PositiveDuration::zero())];

    let departure_datetime = SecondsSinceDatasetStart::zero();

    let request = request::depart_after::Request::new(&transit_data, departure_datetime, start_stops, end_stops);

    let mut raptor = engine::multicriteria_raptor::MultiCriteriaRaptor::new(&request);
    info!("Start computing journey");
    raptor.compute();
    info!("Journeys computed");
    info!("Nb of journeys found : {}", raptor.nb_of_journeys());

    // let a_few_vj : Vec<_> = collections.vehicle_journeys.values().take(2).collect();
    // println!("{:#?}", a_few_vj);

}

