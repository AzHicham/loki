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

use launch::loki::chrono::NaiveDate;
use launch::loki::schedule::ScheduleOn;
use launch::loki::{NaiveDateTime, RealTimeLevel};
use loki_server::{navitia_proto, server_config::ServerConfig};

pub async fn simple_next_departure_test(config: &ServerConfig) {
    let date = NaiveDate::from_ymd(2021, 1, 1);
    let from_datetime = date.and_hms(8, 0, 0);

    let schedule_request = make_schedule_request(
        ScheduleOn::BoardTimes,
        "stop_area:massy_area",
        from_datetime,
        2 * 60 * 60, // duration 2h
        10,
        RealTimeLevel::Base,
        &[],
    );

    let response = crate::send_request_and_wait_for_response(
        &config.requests_socket,
        schedule_request.clone(),
    )
    .await;

    assert_eq!(response.next_departures.len(), 1);
    let passage = &response.next_departures[0];

    assert_eq!(
        passage.stop_date_time.departure_date_time.unwrap(),
        date.and_hms(8, 0, 0).timestamp() as u64
    );
    assert_eq!(
        passage.stop_date_time.stop_point.as_ref().unwrap().name,
        Some("Massy".to_string())
    );
    assert_eq!(
        passage.route.as_ref().unwrap().name,
        Some("Nord".to_string())
    );
}

pub async fn simple_next_arrival_test(config: &ServerConfig) {
    let date = NaiveDate::from_ymd(2021, 1, 1);
    let from_datetime = date.and_hms(8, 0, 0);

    let schedule_request = make_schedule_request(
        ScheduleOn::DebarkTimes,
        "stop_area:cdg_area",
        from_datetime,
        2 * 60 * 60, // duration 2h
        10,
        RealTimeLevel::Base,
        &[],
    );

    let response = crate::send_request_and_wait_for_response(
        &config.requests_socket,
        schedule_request.clone(),
    )
    .await;

    assert_eq!(response.next_arrivals.len(), 1);
    let passage = &response.next_arrivals[0];

    assert_eq!(
        passage.stop_date_time.arrival_date_time.unwrap(),
        date.and_hms(9, 30, 0).timestamp() as u64
    );
    assert_eq!(
        passage.stop_date_time.stop_point.as_ref().unwrap().name,
        Some("CDG".to_string())
    );
    assert_eq!(
        passage.route.as_ref().unwrap().name,
        Some("Nord".to_string())
    );
}

fn make_schedule_request(
    schedule_on: ScheduleOn,
    input_filter: &str,
    from_datetime: NaiveDateTime,
    duration: i32,
    nb_max_responses: i32,
    real_time_level: RealTimeLevel,
    forbidden_uri: &[&str],
) -> navitia_proto::Request {
    let mut next_stop_request = navitia_proto::NextStopTimeRequest {
        from_datetime: u64::try_from(from_datetime.timestamp()).ok(),
        duration,
        nb_stoptimes: nb_max_responses,
        count: nb_max_responses,
        start_page: 0,
        forbidden_uri: forbidden_uri.iter().map(|str| str.to_string()).collect(),
        ..Default::default()
    };
    match schedule_on {
        ScheduleOn::BoardTimes => {
            next_stop_request.departure_filter = input_filter.to_string();
        }
        ScheduleOn::DebarkTimes => {
            next_stop_request.arrival_filter = input_filter.to_string();
        }
    };
    match real_time_level {
        RealTimeLevel::Base => {
            next_stop_request.set_realtime_level(navitia_proto::RtLevel::BaseSchedule);
        }
        RealTimeLevel::RealTime => {
            next_stop_request.set_realtime_level(navitia_proto::RtLevel::Realtime);
        }
    };
    let mut request = navitia_proto::Request {
        next_stop_times: Some(next_stop_request),
        ..Default::default()
    };
    match schedule_on {
        ScheduleOn::BoardTimes => {
            request.set_requested_api(navitia_proto::Api::NextDepartures);
        }
        ScheduleOn::DebarkTimes => {
            request.set_requested_api(navitia_proto::Api::NextArrivals);
        }
    };
    request
}
