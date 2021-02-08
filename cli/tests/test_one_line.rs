use laxatips::LoadsPeriodicData;
use laxatips_cli::{init_logger, parse_datetime, parse_duration, solve, build};
use failure::Error;

use laxatips_cli::stop_areas::{Options, launch};
use laxatips_cli::{BaseOptions, RequestConfig};


#[test]
fn test_1() -> Result<(), Error> {


    let request_config = RequestConfig::default();
    

    let base = BaseOptions {
        ntfs_path: "tests/one_line".to_string(),
        loads_data_path: "tests/one_line/loads.csv".to_string(),
        departure_datetime: Some("20210101T060000".to_string()),
        request_config,
        implem: "loads_daily".to_string(),
        request_type: "loads".to_string(),


    };


    let options = Options {
        base,
        start: "Navitia:massy".to_string(),
        end: "Navitia:paris".to_string(),

    };
    println!("{:#?}", options);
    let responses = launch::<LoadsPeriodicData>(options)?;

    assert!(responses.len() == 2);

    

    Ok(())
    
}
