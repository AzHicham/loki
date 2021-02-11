use failure::Error;
use laxatips::LoadsPeriodicData;

use laxatips_cli::stop_areas::{launch, Options};
use laxatips_cli::{BaseOptions, RequestConfig};

// The data consists of  a single line from `massy` to `paris`
// with three trips. The first and last trip area heavily loaded
//  while the second trip has a light load.
//
// trips                 | `matin`   | `midi`   |  `soir`
// leave `massy` at      | 08:00:00  | 12:00:00 | 18:00:00
// arrives at `paris` at | 09:00:00  | 13:00:00 | 19:00:00
// load                  |  80%      |  20%     | 80%

#[test]
fn test_loads_matin() -> Result<(), Error> {
    // Here we make a request from `massy` to `paris` at 08:00:00
    // We use the loads as criteria.
    // We should obtain two journeys :
    //  - one with `matin` as it arrives the earliest in `paris`
    //  - one with `midi` as it has a lighter load than `matin`
    // The `soir` trip arrives later and has a high load, and thus should
    //  not be present.
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
fn test_loads_midi() -> Result<(), Error> {
    // Here we make a request from `massy` to `paris` at 10:00:00
    // We use the loads as criteria.
    // We should obtain only one journey with the `midi` trip.
    // Indeed, `matin` cannot be boarded, and `soir` arrives
    // later than `midi` with a higher load
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
fn test_without_loads_matin() -> Result<(), Error> {
    // Here we make a request from `massy` to `paris` at 08:00:00
    // We do NOT use the loads as criteria.
    // We should obtain only one journey with the `matin` trip.
    // Indeed, `midi` and `soir` arrives later than `matin`.
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
