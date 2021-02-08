
use laxatips::{ LoadsDailyData,  LoadsPeriodicData};
use laxatips::{
    DailyData,  PeriodicData, 
};

use failure::{bail, Error};


use structopt::StructOpt;

use laxatips_cli::{ stop_areas::{Options, launch}, init_logger};



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
    match options.base.implem.as_str() {
        "periodic" =>  { launch::<PeriodicData>(options)?; },
        "daily" => { launch::<DailyData>(options)?; },
        "loads_periodic" => {launch::<LoadsPeriodicData>(options)?; },
        "loads_daily" => { launch::<LoadsDailyData>(options)?; },
        _ => bail!(format!("Bad implem option : {}.", options.base.implem)),
    };
    Ok(())
}
