use laxatips::transit_model;
use laxatips::{ TransitData, MultiCriteriaRaptor, DepartAfterRequest, PositiveDuration, SecondsSinceDatasetStart};
use laxatips::log::{info, error};

use std::path::PathBuf;

use slog::slog_o;
use slog::Drain;
use slog_async::OverflowStrategy;

use std::time::SystemTime;


use structopt::StructOpt;


#[derive(StructOpt)]
#[structopt(name = "laxatips_cli", about = "Run laxatips from the command line.")]
struct Options {
    /// directory of ntfs files to load
    #[structopt(short = "n", long = "ntfs", parse(from_os_str),)]
    input: PathBuf,

    // /// current datetime
    // #[structopt(
    //     short = "x",
    //     long,
    //     parse(try_from_str),
    //     default_value = &transit_model::CURRENT_DATETIME
    // )]
    // current_datetime: DateTime<FixedOffset>,


    /// transfer penalty to apply
    #[structopt(long, default_value = "120")]
    transfer_arrival_penalty: u32,

    #[structopt(long, default_value = "120")]
    transfer_walking_penalty: u32,

    #[structopt(long, default_value = "10")]
    max_nb_transfer: u8,

    #[structopt(long, default_value = "100")]
    nb_queries : u32,

}
// #[derive(StructOpt)]
// struct RandomStopAreas {
//     #[structopt(short ="n", long, default_value = "100")]
//     nb_queries : u32,

// }





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


fn random_stop_uri(model : & transit_model::Model) -> String {
    use rand::prelude::*;
    let mut rng = thread_rng();
    model.stop_points.values().choose(& mut rng).unwrap().id.to_owned()
}

fn make_random_queries_stop_point(model: & transit_model::Model, nb_of_queries : usize ) -> Vec<(String, String)> {
    use rand::prelude::*;
    //let mut rng = thread_rng();
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(10);
    let mut result = Vec::new();
    for _ in 0..nb_of_queries {
        let start_uri = model.stop_points.values().choose(& mut rng).unwrap().id.to_owned();
        let stop_uri = model.stop_points.values().choose(& mut rng).unwrap().id.to_owned();
        result.push((start_uri, stop_uri));
    }
    result 
}



fn make_random_queries_stop_area(model: & transit_model::Model, nb_of_queries : u32 ) -> Vec<(Vec<String>, Vec<String>)> {
    use rand::prelude::*;
    //let mut rng = thread_rng();
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(1);
    let mut result = Vec::new();
    for _ in 0..nb_of_queries {
        let start_stop_area = &model.stop_areas.values().choose(& mut rng).unwrap().id;
        let end_stop_area = &model.stop_areas.values().choose(& mut rng).unwrap().id;
        info!("Requesting stop areas {} {}", start_stop_area, end_stop_area);
        result.push(make_query_stop_area(model, &start_stop_area, &end_stop_area));
    }
    result 
}

fn make_query_stop_area(model: & transit_model::Model, from_stop_area : &str, to_stop_area : &str ) -> (Vec<String>, Vec<String>) {
    use std::collections::BTreeSet;
    let mut start_sa_set = BTreeSet::new();
    start_sa_set.insert(model.stop_areas.get_idx(from_stop_area).unwrap());
    let start_stop_points : Vec<String>= model.get_corresponding(&start_sa_set).iter().map(|idx| model.stop_points[*idx].id.clone()).collect();
    let mut end_sa_set = BTreeSet::new();
    end_sa_set.insert(model.stop_areas.get_idx(to_stop_area).unwrap());
    let end_stop_points = model.get_corresponding(&end_sa_set).iter().map(|idx| model.stop_points[*idx].id.clone()).collect();
    (start_stop_points, end_stop_points)
}

fn main() {
    let _log_guard = init_logger();

    let options = Options::from_args();
    let input_dir = options.input;

    // let input_dir = PathBuf::from("tests/fixtures/small_ntfs/");
    // let start_stop_point_uri = "sp_1";
    // let end_stop_point_uri = "sp_3";

    // let input_dir = PathBuf::from("tests/fixtures/ntfs_rennes/");
    // let start_stop_point_uri = "SAR:SP:1001";
    // let end_stop_point_uri = "SAR:SP:6006";

    // let input_dir = PathBuf::from("/home/pascal/data/paris/ntfs/");
    // let start_stop_point_uri = "OIF:SP:59:3619855";
    // let end_stop_point_uri = "OIF:SP:6:109";

    let model = transit_model::ntfs::read(input_dir).unwrap();
    info!("Transit model loaded");
    info!("Number of vehicle journeys : {}", model.vehicle_journeys.len());
    info!("Number of routes : {}", model.routes.len());

    // use std::fs::File;
    // use std::io::BufReader;
    // use std::io::BufWriter;
    // // {
    // //     let out_file = File::create("foo.txt").unwrap();
    // //     let writer = BufWriter::new(out_file);
    // //     serde_json::to_writer_pretty(writer, &model.into_collections()).expect("error writing");
    // // }
    // let file = File::open("foo.txt").unwrap();
    // let reader = BufReader::new(file);
    // let collections : transit_model::model::Collections = serde_json::from_reader(reader).unwrap();
    // println!("{:#?}", collections.datasets);
    // println!("{:#?}", collections.calendars);
    // let model : transit_model::Model = transit_model::Model::new(collections).unwrap();

    // println!("{:#?}", model.datasets);
    // println!("{:#?}", model.lines);


    let data_timer = SystemTime::now();
    let default_transfer_duration = PositiveDuration{seconds : 60};
    let transit_data = TransitData::new(&model, default_transfer_duration);
    let data_build_time = data_timer.elapsed().unwrap().as_millis();
    info!("Data constructed");
    info!("Number of pattern {} ", transit_data.nb_of_patterns());
    info!("Number of timetables {} ", transit_data.nb_of_timetables());
    info!("Number of vehicles {} ", transit_data.nb_of_vehicles());




    let nb_of_stops = transit_data.nb_of_stops();

    let mut raptor = MultiCriteriaRaptor::new(nb_of_stops);


    let start_ends = vec![("SAR:SP:1001", "SAR:SP:6006"), ("SAR:SP:6000", "SAR:SP:6006")];
    let start_ends = vec![("SAR:SP:1660", "SAR:SP:6005"),("SAR:SP:1001", "SAR:SP:6005"),("SAR:SP:1617", "SAR:SP:6005")];
    // let start_ends = vec![("SAR:SP:1001", "SAR:SP:6006"); 1000];

    let start_ends = vec![("OIF:SP:59:3619855", "OIF:SP:6:109"); 1];
    let start_ends = vec![("OIF:SP:8775815:810:A", "OIF:SP:88:241"); 1];
    let start_ends = make_random_queries_stop_area(&model, options.nb_queries);

    // let start_ends = vec![make_query_stop_area(&model, &"OIF:SA:8739384", &"OIF:SA:8768600")];

    let compute_timer = SystemTime::now();
    for (start_stop_point_uris, end_stop_point_uris) in &start_ends {
        let start_stops = start_stop_point_uris.iter().map(|uri| {
            let stop_idx = model.stop_points.get_idx(&uri).unwrap_or_else( || {
                error!("Start stop point {} not found in model", uri);
                panic!();
            });
            let stop = transit_data.stop_point_idx_to_stop(&stop_idx).unwrap().clone();
            (stop, PositiveDuration::zero())
        }).collect();

        let end_stops = end_stop_point_uris.iter().map(|uri| {
            let stop_idx = model.stop_points.get_idx(&uri).unwrap_or_else( || {
                error!("Start stop point {} not found in model", uri);
                panic!();
            });
            let stop = transit_data.stop_point_idx_to_stop(&stop_idx).unwrap().clone();
            (stop, PositiveDuration::zero())
        }).collect();


    
        let departure_datetime = SecondsSinceDatasetStart::zero() + PositiveDuration{ seconds : 8*60*60} + PositiveDuration{ seconds : 24*60*60}; 

        let transfer_arrival_penalty = PositiveDuration{ seconds : options.transfer_arrival_penalty};
        let transfer_walking_penalty = PositiveDuration{ seconds : options.transfer_walking_penalty};
        let max_arrival_time = departure_datetime.clone() + PositiveDuration{ seconds : 12*60*60};
        let max_nb_transfer : u8 = options.max_nb_transfer;

        let request = DepartAfterRequest::new(&transit_data, departure_datetime, start_stops, end_stops, transfer_arrival_penalty, transfer_walking_penalty, max_arrival_time, max_nb_transfer);

        info!("Start computing journey");
        let request_timer = SystemTime::now();
        raptor.compute(&request);
        info!("Journeys computed in {} ms with {} rounds", request_timer.elapsed().unwrap().as_millis(), raptor.nb_of_rounds());
        info!("Nb of journeys found : {}", raptor.nb_of_journeys());
        info!("Tree size : {}", raptor.tree_size());
        for pt_journey in raptor.responses() {
            let response = request.create_response_from_engine_result(pt_journey).unwrap();
            // info!("{:#?}", criteria);
            transit_data.print_response(&response, &model);
        }
    
    }
    let duration = compute_timer.elapsed().unwrap().as_millis();
    info!("Data build duration {} ms", data_build_time);
    info!("Average duration per request : {} ms", (duration as f64) / (start_ends.len() as f64));
    // request.solve_with(raptor) <- call raptor, and fill request.responses
    // read request.responses and print them

    // request.fill_with(request data)


    // let a_few_vj : Vec<_> = collections.vehicle_journeys.values().take(2).collect();
    // println!("{:#?}", a_few_vj);

}