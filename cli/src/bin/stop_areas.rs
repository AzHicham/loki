use laxatips::{DailyData, PeriodicData};
use laxatips::{LoadsDailyData, LoadsPeriodicData};
use laxatips::config;

use failure::Error;

use structopt::StructOpt;

use laxatips_cli::{
    init_logger,
    stop_areas::{launch, Options},
};

fn main() {
    let _log_guard = init_logger();
    if let Err(err) = run() {
        for cause in err.iter_chain() {
            eprintln!("{}", cause);
        }
        std::process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    let options = Options::from_args();
    match options.base.implem {
        config::Implem::Periodic => {
            launch::<PeriodicData>(options)?;
        }
        config::Implem::Daily => {
            launch::<DailyData>(options)?;
        }
        config::Implem::LoadsPeriodic => {
            launch::<LoadsPeriodicData>(options)?;
        }
        config::Implem::LoadsDaily => {
            launch::<LoadsDailyData>(options)?;
        }
    };
    Ok(())
}
