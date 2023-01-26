// Copyright  (C) 2022, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Hove (www.kisio.com).
// Help us simplify mobility and open public transport:
// a non ending quest to the responsive locomotion way of traveling!
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

pub use loki_server;
use loki_server::server_config::ServerConfig;

use crate::{datetime, first_section_vj_name};

pub async fn massy_to_paris(config: &ServerConfig) {
    // the ntfs (in tests/a_small_ntfs) contains one trip
    // with a vehicle_journey named "matin"
    // departing from "massy" at 8h and arriving to "paris" at 9h
    // on day 2021-01-01
    let request_datetime = datetime("2021-01-01 08:00:00");

    // initial request, on base schedule
    let base_request =
        crate::make_journeys_request("stop_point:massy", "stop_point:paris", request_datetime);

    let journeys_response =
        crate::send_request_and_wait_for_response(&config.requests_socket, base_request.clone())
            .await;
    // info!("{:#?}", journeys_response);
    // check that we have a journey, that uses the only trip in the ntfs
    assert_eq!(
        first_section_vj_name(&journeys_response.journeys[0]),
        "vehicle_journey:matin"
    );
}

pub async fn massy_to_paris_stop_area(config: &ServerConfig) {
    // the ntfs (in tests/a_small_ntfs) contains one trip
    // with a vehicle_journey named "matin"
    // departing from "massy" at 8h and arriving to "paris" at 9h
    // on day 2021-01-01
    let request_datetime = datetime("2021-01-01 08:00:00");

    // initial request, on base schedule
    let base_request = crate::make_journeys_request(
        "stop_area:massy_area",
        "stop_area:paris_area",
        request_datetime,
    );

    let journeys_response =
        crate::send_request_and_wait_for_response(&config.requests_socket, base_request.clone())
            .await;
    // info!("{:#?}", journeys_response);
    // check that we have a journey, that uses the only trip in the ntfs
    assert_eq!(
        first_section_vj_name(&journeys_response.journeys[0]),
        "vehicle_journey:matin"
    );
}
