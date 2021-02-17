use laxatips::{DailyData, PeriodicData};
use laxatips::{LoadsDailyData, LoadsPeriodicData};

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
    use laxatips_cli::Implem::*;
    match options.base.implem {
        Periodic => {
            launch::<PeriodicData>(options)?;
        }
        Daily => {
            launch::<DailyData>(options)?;
        }
        LoadsPeriodic => {
            launch::<LoadsPeriodicData>(options)?;
        }
        LoadsDaily => {
            launch::<LoadsDailyData>(options)?;
        }
    };
    Ok(())
}
