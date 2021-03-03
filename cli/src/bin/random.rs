use laxatips::config;
use laxatips::{DailyData, PeriodicData};
use laxatips::{LoadsDailyData, LoadsPeriodicData};

use failure::Error;

use structopt::StructOpt;

use laxatips_cli::{
    init_logger,
    random::{launch, Options},
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
    match options.base.data_implem {
        config::DataImplem::Periodic => {
            launch::<PeriodicData>(options)?;
        }
        config::DataImplem::Daily => {
            launch::<DailyData>(options)?;
        }
        config::DataImplem::LoadsPeriodic => {
            launch::<LoadsPeriodicData>(options)?;
        }
        config::DataImplem::LoadsDaily => {
            launch::<LoadsDailyData>(options)?;
        }
    };
    Ok(())
}
