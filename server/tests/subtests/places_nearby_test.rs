// Copyright  (C) 2021, Kisio Digital and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Kisio Digital (www.kisio.com).
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

use loki_server::{navitia_proto, server_config::ServerConfig};

pub async fn places_nearby_test(config: &ServerConfig) {
    let places_nearby_request = make_places_nearby_request("coord:2.260:48.725", 500_f64, 0, 10);
    let places_nearby_response =
        crate::send_request_and_wait_for_response(&config.requests_socket, places_nearby_request)
            .await;
    assert!(!places_nearby_response.places_nearby.is_empty());
    let pt_object = &places_nearby_response.places_nearby[0];
    assert_eq!(pt_object.uri, "stop_point:massy");
    assert_eq!(pt_object.distance, Some(62));
    assert!(places_nearby_response.pagination.is_some());
    let pagination = places_nearby_response.pagination.unwrap();
    assert_eq!(pagination.start_page, 0);
    assert_eq!(pagination.items_on_page, 1);
    assert_eq!(pagination.items_per_page, 10);
    assert_eq!(pagination.total_result, 1);

    let places_nearby_request = make_places_nearby_request("coord:2.260:48.725", 500_f64, 1, 10);
    let places_nearby_response =
        crate::send_request_and_wait_for_response(&config.requests_socket, places_nearby_request)
            .await;
    assert!(places_nearby_response.places_nearby.is_empty());
    assert!(places_nearby_response.pagination.is_some());
    let pagination = places_nearby_response.pagination.unwrap();
    assert_eq!(pagination.start_page, 1);
    assert_eq!(pagination.items_on_page, 0);
    assert_eq!(pagination.items_per_page, 10);
    assert_eq!(pagination.total_result, 1);
}

fn make_places_nearby_request(
    uri: &str,
    distance: f64,
    start_page: i32,
    item_per_page: i32,
) -> navitia_proto::Request {
    let places_nearby_request = navitia_proto::PlacesNearbyRequest {
        distance,
        uri: uri.to_string(),
        count: item_per_page,
        start_page,
        ..Default::default()
    };
    let mut request = navitia_proto::Request {
        places_nearby: Some(places_nearby_request),
        ..Default::default()
    };
    request.set_requested_api(navitia_proto::Api::PlacesNearby);
    request
}
