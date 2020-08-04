use laxatips::log::{debug, info, trace};
use laxatips::transit_model;
use laxatips::{
    LaxatipsData,
    DepartAfterRequest, MultiCriteriaRaptor, PositiveDuration, 
    
};

use std::path::PathBuf;

use slog::slog_o;
use slog::Drain;
use slog_async::OverflowStrategy;

use chrono::NaiveDateTime;
use failure::{bail, Error};
use std::time::SystemTime;

use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "laxatips_cli", about = "Run laxatips from the command line.")]
struct Options {
    /// directory of ntfs files to load
    #[structopt(short = "n", long = "ntfs", parse(from_os_str))]
    input: PathBuf,

    /// penalty to apply to arrival time for each vehicle leg in a journey
    #[structopt(long, default_value = "00:02:00")]
    leg_arrival_penalty: String,

    /// penalty to apply to walking time for each vehicle leg in a journey
    #[structopt(long, default_value = "00:02:00")]
    leg_walking_penalty: String,

    /// maximum number of vehicle legs in a journey
    #[structopt(long, default_value = "10")]
    max_nb_of_legs: u8,

    /// maximum duration of a journey
    #[structopt(long, default_value = "24:00:00")]
    max_journey_duration: String,

    /// Departure datetime of the query, formatted like 20190628T163215
    /// If none is given, all queries will be made at 08:00:00 on the first
    /// valid day of the dataset
    #[structopt(long)]
    departure_datetime: Option<String>,

    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt)]
enum Command {
    Random(Random),
    StopAreas(StopAreas),
}

#[derive(StructOpt)]
struct Random {
    #[structopt(short = "n", long, default_value = "10")]
    nb_queries: u32,
}

#[derive(StructOpt)]
struct StopAreas {
    #[structopt(long)]
    start: String,

    #[structopt(long)]
    end: String,
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

fn make_query_stop_area(
    model: &transit_model::Model,
    from_stop_area: &str,
    to_stop_area: &str,
) -> (Vec<String>, Vec<String>) {
    use std::collections::BTreeSet;
    let mut start_sa_set = BTreeSet::new();
    start_sa_set.insert(model.stop_areas.get_idx(from_stop_area).unwrap());
    let start_stop_points: Vec<String> = model
        .get_corresponding(&start_sa_set)
        .iter()
        .map(|idx| model.stop_points[*idx].id.clone())
        .collect();
    let mut end_sa_set = BTreeSet::new();
    end_sa_set.insert(model.stop_areas.get_idx(to_stop_area).unwrap());
    let end_stop_points = model
        .get_corresponding(&end_sa_set)
        .iter()
        .map(|idx| model.stop_points[*idx].id.clone())
        .collect();
    (start_stop_points, end_stop_points)
}

fn parse_datetime(string_datetime: &str) -> Result<NaiveDateTime, Error> {
    let try_datetime = NaiveDateTime::parse_from_str(string_datetime, "%Y%m%dT%H%M%S");
    match try_datetime {
        Ok(datetime) => Ok(datetime),
        Err(_) => bail!(
            "Unable to parse {} as a datetime. Expected format is 20190628T163215",
            string_datetime
        ),
    }
}

fn parse_duration(string_duration: &str) -> Result<PositiveDuration, Error> {
    let mut t = string_duration.split(':');
    let (hours, minutes, seconds) = match (t.next(), t.next(), t.next(), t.next()) {
        (Some(h), Some(m), Some(s), None) => (h, m, s),
        _ => {
            bail!(
                "Unable to parse {} as a duration. Expected format is 14:35:12",
                string_duration
            );
        }
    };
    let hours: u32 = hours.parse()?;
    let minutes: u32 = minutes.parse()?;
    let seconds: u32 = seconds.parse()?;
    if minutes > 59 || seconds > 59 {
        bail!(
            "Unable to parse {} as a duration. Expected format is 14:35:12",
            string_duration
        );
    }
    Ok(PositiveDuration::from_hms(hours, minutes, seconds))
}

fn run() -> Result<(), Error> {
    let options = Options::from_args();
    let input_dir = options.input;

    let model = transit_model::ntfs::read(input_dir)?;
    info!("Transit model loaded");
    info!(
        "Number of vehicle journeys : {}",
        model.vehicle_journeys.len()
    );
    info!("Number of routes : {}", model.routes.len());

    let data_timer = SystemTime::now();
    let default_transfer_duration = PositiveDuration::from_hms(0, 0, 60);
    let laxatips_data = LaxatipsData::new(model, default_transfer_duration);
    let transit_data = &laxatips_data.transit_data;
    let data_build_time = data_timer.elapsed().unwrap().as_millis();
    info!("Data constructed");
    info!("Number of pattern {} ", transit_data.nb_of_patterns());
    info!("Number of timetables {} ", transit_data.nb_of_timetables());
    info!("Number of vehicles {} ", transit_data.nb_of_vehicles());
    info!(
        "Validity dates between {} and {}",
        transit_data.calendar.first_date(),
        transit_data.calendar.last_date()
    );

    let nb_of_stops = transit_data.nb_of_stops();

    let mut raptor = MultiCriteriaRaptor::<DepartAfterRequest>::new(nb_of_stops);

    let departure_datetime = match options.departure_datetime {
            Some(string_datetime) => parse_datetime(&string_datetime)?,
            None => {
                let naive_date = transit_data.calendar.first_date();
                naive_date.and_hms(8, 0, 0)
            }
        };


    let leg_arrival_penalty = parse_duration(&options.leg_arrival_penalty).unwrap();
    let leg_walking_penalty = parse_duration(&options.leg_walking_penalty).unwrap();
    let max_journey_duration = parse_duration(&options.max_journey_duration).unwrap();
    let max_nb_of_legs: u8 = options.max_nb_of_legs;

    let compute_timer = SystemTime::now();

    match options.command {
        Command::Random(random) => {
            let mut total_nb_of_rounds = 0;
            let nb_queries = random.nb_queries;
            use rand::prelude::{SeedableRng, IteratorRandom};
            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(1);
            for _ in 1..nb_queries {
                       
                let start_stop_area_uri = &laxatips_data.model.stop_areas.values().choose(&mut rng).unwrap().id;
                let end_stop_area_uri = &laxatips_data.model.stop_areas.values().choose(&mut rng).unwrap().id;

                let nb_of_rounds = solve(
                    start_stop_area_uri,
                    end_stop_area_uri,
                    &mut raptor,
                    &laxatips_data,
                    &departure_datetime,
                    &leg_arrival_penalty,
                    &leg_walking_penalty,
                    &max_journey_duration,
                    max_nb_of_legs,
                )?;

                total_nb_of_rounds += nb_of_rounds;
            }
            let duration = compute_timer.elapsed().unwrap().as_millis();
            info!("Data build duration {} ms", data_build_time);
            info!(
                "Average duration per request : {} ms",
                (duration as f64) / (nb_queries as f64)
            );
            info!(
                "Average nb of rounds : {}",
                (total_nb_of_rounds as f64) / (nb_queries as f64)
            );
        }
        Command::StopAreas(stop_areas) => {
            let start_stop_area_uri = &stop_areas.start;
            let end_stop_area_uri = &stop_areas.end;

            solve(
                start_stop_area_uri,
                end_stop_area_uri,
                &mut raptor,
                &laxatips_data,
                &departure_datetime,
                &leg_arrival_penalty,
                &leg_walking_penalty,
                &max_journey_duration,
                max_nb_of_legs,
            )?;
        }
    };

    Ok(())
}

fn solve<'data>(
    start_stop_area_uri: &str,
    end_stop_area_uri: &str,
    engine: &mut MultiCriteriaRaptor<DepartAfterRequest<'data>>,
    laxatips_data : & 'data LaxatipsData,
    departure_datetime: & NaiveDateTime,
    leg_arrival_penalty: &PositiveDuration,
    leg_walking_penalty: &PositiveDuration,
    max_duration_to_arrival: &PositiveDuration,
    max_nb_of_legs: u8,
) -> Result<usize, Error> {
    trace!(
        "Request start stop area : {}, end stop_area : {}",
        start_stop_area_uri,
        end_stop_area_uri
    );
    let (start_stop_point_uris, end_stop_point_uris) =
        make_query_stop_area(&laxatips_data.model, start_stop_area_uri, end_stop_area_uri);
    let start_stops = start_stop_point_uris
        .iter()
        .map(|uri| {
            (uri.as_str(), PositiveDuration::zero())
        });

    let end_stops = end_stop_point_uris
        .iter()
        .map(|uri| {
            (uri.as_str(), PositiveDuration::zero())
        });

    let request = DepartAfterRequest::<'data>::new(
        laxatips_data,
        departure_datetime.clone(),
        start_stops,
        end_stops,
        leg_arrival_penalty.clone(),
        leg_walking_penalty.clone(),
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
    debug!("Tree size : {}", engine.tree_size());
    for pt_journey in engine.responses() {
        let response = request
             .create_response(pt_journey)
             .unwrap();            
        trace!("{}", response.print(laxatips_data)?);

    }

    Ok(engine.nb_of_rounds())
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
