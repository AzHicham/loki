// Copyright  (C) 2020, Kisio Digital and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Kisio Digital (www.kisio.com).
// Help us simplify mobility and open public transport:
// a non ending quest to the responsive locomotion way of traveling!
//
// This contribution is a part of the research and development work of the
// IVA Project which aims to enhance traveler information and is carried out
// under the leadership of the Technological Research Institute SystemX,
// with the partnership and support of the transport organization authority
// Ile-De-France Mobilités (IDFM), SNCF, and public funds
// under the scope of the French Program "Investissements d’Avenir".
//
// LICENCE: This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// Stay tuned using
// twitter @navitia
// channel `#navitia` on riot https://riot.im/app/#/room/#navitia:matrix.org
// https://groups.google.com/d/forum/navitia
// www.navitia.io



use failure::Error;

use stop_areas::{launch, Config, };

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

    let config : Config = serde_json::from_str(r#" {
            "input_data_path" : "tests/one_line",
            "input_data_type" : "ntfs",
            "loads_data_path" : "tests/one_line/loads.csv",
            "criteria_implem" : "loads",
            "data_implem" : "loads_periodic",
            "departure_datetime" : "20210101T080000",
            "comparator_type" : "loads",
            "start" : "stop_area:massy",
            "end" : "stop_area:paris"
          } "# )?;

    let (model, mut responses) = launch(config)?;

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

    let config : Config = serde_json::from_str(r#" {
        "input_data_path": "tests/one_line",
        "input_data_type": "ntfs",
        "loads_data_path": "tests/one_line/loads.csv",
        "criteria_implem": "loads",
        "data_implem": "loads_periodic",
        "departure_datetime": "20210101T100000",
        "comparator_type": "loads",
        "start" : "stop_area:massy",
        "end" : "stop_area:paris"
      } "# )?;



    let (model, mut responses) = launch(config)?;

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
    let config : Config = serde_json::from_str(r#" {
        "input_data_path": "tests/one_line",
        "input_data_type": "ntfs",
        "loads_data_path": "tests/one_line/loads.csv",
        "criteria_implem": "loads",
        "data_implem": "loads_periodic",
        "departure_datetime": "20210101T080000",
        "comparator_type": "basic",
        "start" : "stop_area:massy",
        "end" : "stop_area:paris"
      } "# )?;



    let (model, mut responses) = launch(config)?;

    assert!(responses.len() == 1);
    responses.sort_by_key(|resp| resp.first_vehicle.from_datetime);
    assert!(responses[0].first_vj_uri(&model) == "matin");

    Ok(())
}
