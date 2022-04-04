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

fn check_access_point(
    access_points: &Vec<navitia_proto::AccessPoint>,
    uri: &str,
    name: &str,
    lon: f64,
    lat: f64,
    is_entrance: bool,
    is_exit: bool,
    length: i32,
    traversal_time: i32,
    stair_count: i32,
    max_slope: i32,
    min_width: i32,
    signposted_as: &str,
    reversed_signposted_as: &str,
    parent_station: &str,
) {
    let ap = access_points
        .iter()
        .find(|ap| ap.uri == Some(uri.to_string()))
        .unwrap();

    assert_eq!(ap.name, Some(name.to_string()));
    assert_eq!(
        ap.coord,
        Some(navitia_proto::GeographicalCoord { lon, lat })
    );
    assert_eq!(ap.is_entrance, Some(is_entrance));
    assert_eq!(ap.is_exit, Some(is_exit));
    assert_eq!(ap.length, Some(length));
    assert_eq!(ap.traversal_time, Some(traversal_time));
    assert_eq!(ap.stair_count, Some(stair_count));
    assert_eq!(ap.max_slope, Some(max_slope));
    assert_eq!(ap.min_width, Some(min_width));
    assert_eq!(ap.signposted_as, Some(signposted_as.to_string()));
    assert_eq!(
        ap.reversed_signposted_as,
        Some(reversed_signposted_as.to_string())
    );
    assert_eq!(
        ap.parent_station.as_ref().map(|p| p.uri.as_ref()),
        Some(Some(&parent_station.to_string()))
    );
}

pub async fn places_nearby_test(config: &ServerConfig) {
    let places_nearby_request = make_places_nearby_request("coord:2.260:48.725", 500_f64, 0, 10, 2);
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

    // With depth = 3, we are supposed to have access points under stop points.
    let places_nearby_request = make_places_nearby_request("coord:2.260:48.725", 500_f64, 0, 10, 3);
    let places_nearby_response =
        crate::send_request_and_wait_for_response(&config.requests_socket, places_nearby_request)
            .await;
    assert!(!places_nearby_response.places_nearby.is_empty());
    let pt_object = &places_nearby_response.places_nearby[0];
    assert_eq!(pt_object.uri, "stop_point:massy");
    let access_points = &pt_object.stop_point.as_ref().unwrap().access_points;
    assert_eq!(access_points.len(), 4);

    check_access_point(
        access_points,
        "access_point:massy_entrance_only_exit",
        "Massy Only Exit",
        2.2601,
        48.7251,
        false,
        true,
        42,
        43,
        44,
        45,
        46,
        "47",
        "48",
        "stop_area:massy_area",
    );

    check_access_point(
        access_points,
        "access_point:massy_entrance_only_entrance",
        "Massy Only Entrance",
        2.2602,
        48.7252,
        true,
        false,
        21,
        22,
        23,
        24,
        25,
        "26",
        "27",
        "stop_area:massy_area",
    );

    check_access_point(
        access_points,
        "access_point:massy_entrance_both_entrance_and_exit_1",
        "Massy Entrance And Exit 1",
        2.2603,
        48.7253,
        true,
        true,
        84,
        85,
        86,
        87,
        88,
        "89",
        "90",
        "stop_area:massy_area",
    );

    check_access_point(
        access_points,
        "access_point:massy_entrance_both_entrance_and_exit_2",
        "Massy Entrance And Exit 2",
        2.2604,
        48.7254,
        true,
        true,
        100,
        101,
        102,
        103,
        104,
        "105",
        "106",
        "stop_area:massy_area",
    );

    let places_nearby_request = make_places_nearby_request("coord:2.260:48.725", 500_f64, 1, 10, 2);
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
    depth: i32,
) -> navitia_proto::Request {
    let places_nearby_request = navitia_proto::PlacesNearbyRequest {
        distance,
        uri: uri.to_string(),
        count: item_per_page,
        start_page,
        depth,
        ..Default::default()
    };
    let mut request = navitia_proto::Request {
        places_nearby: Some(places_nearby_request),
        ..Default::default()
    };
    request.set_requested_api(navitia_proto::Api::PlacesNearby);
    request
}
