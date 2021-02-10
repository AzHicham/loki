use failure::Error;
use laxatips::LoadsPeriodicData;

use laxatips_cli::stop_areas::{launch, Options};
use laxatips_cli::{BaseOptions, RequestConfig};

#[test]
fn test_1() -> Result<(), Error> {
    let request_config = RequestConfig::default();

    let base = BaseOptions {
        ntfs_path: "tests/one_line".to_string(),
        loads_data_path: "tests/one_line/loads.csv".to_string(),
        departure_datetime: Some("20210101T080000".to_string()),
        request_config,
        implem: "loads_daily".to_string(),
        request_type: "loads".to_string(),
    };

    let options = Options {
        base,
        start: "stop_area:massy".to_string(),
        end: "stop_area:paris".to_string(),
    };
    println!("Launching : \n {}", options);
    let (model, mut responses) = launch::<LoadsPeriodicData>(options)?;

    assert!(responses.len() == 2);
    responses.sort_by_key(|resp| resp.first_vehicle.from_datetime);
    assert!(responses[0].first_vj_uri(&model) == "matin");
    assert!(responses[1].first_vj_uri(&model) == "midi");

    Ok(())
}

#[test]
fn test_2() -> Result<(), Error> {
    let request_config = RequestConfig::default();

    let base = BaseOptions {
        ntfs_path: "tests/one_line".to_string(),
        loads_data_path: "tests/one_line/loads.csv".to_string(),
        departure_datetime: Some("20210101T100000".to_string()),
        request_config,
        implem: "loads_daily".to_string(),
        request_type: "loads".to_string(),
    };

    let options = Options {
        base,
        start: "stop_area:massy".to_string(),
        end: "stop_area:paris".to_string(),
    };
    println!("Launching : \n {}", options);
    let (model, mut responses) = launch::<LoadsPeriodicData>(options)?;

    assert!(responses.len() == 1);
    responses.sort_by_key(|resp| resp.first_vehicle.from_datetime);
    assert!(responses[0].first_vj_uri(&model) == "midi");

    Ok(())
}

#[test]
fn test_3() -> Result<(), Error> {
    let request_config = RequestConfig::default();

    let base = BaseOptions {
        ntfs_path: "tests/one_line".to_string(),
        loads_data_path: "tests/one_line/loads.csv".to_string(),
        departure_datetime: Some("20210101T080000".to_string()),
        request_config,
        implem: "loads_daily".to_string(),
        request_type: "classic".to_string(),
    };

    let options = Options {
        base,
        start: "stop_area:massy".to_string(),
        end: "stop_area:paris".to_string(),
    };
    println!("Launching : \n {}", options);
    let (model, mut responses) = launch::<LoadsPeriodicData>(options)?;

    assert!(responses.len() == 1);
    responses.sort_by_key(|resp| resp.first_vehicle.from_datetime);
    assert!(responses[0].first_vj_uri(&model) == "matin");

    Ok(())
}
