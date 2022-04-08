// Copyright  (C) 2021, Hove and/or its affiliates. All rights reserved.
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
use loki_server::{navitia_proto, server_config::ServerConfig};

use launch::loki::{chrono::Utc, NaiveDateTime};
use loki_server::status_worker::{DATETIME_FORMAT, PKG_VERSION};

pub async fn status_test(config: &ServerConfig) {
    let status_request = make_status_request();
    let response =
        crate::send_request_and_wait_for_response(&config.requests_socket, status_request).await;
    let now_datetime = Utc::now().naive_utc();

    // We expect a Status response and also a Metadata Response
    let status = response.status.unwrap();
    assert_eq!(status.start_production_date, "20210101");
    assert_eq!(status.end_production_date, "20210103");
    assert_eq!(status.navitia_version, Some(PKG_VERSION.to_string()));
    assert_eq!(status.loaded, Some(true));
    assert_eq!(status.nb_threads, Some(1));
    assert_eq!(status.is_connected_to_rabbitmq, Some(true));
    let last_load_at_str = status.last_load_at.unwrap();
    let last_load_at = NaiveDateTime::parse_from_str(&last_load_at_str, DATETIME_FORMAT).unwrap();
    assert!(now_datetime > last_load_at);
    assert_eq!(status.status, Some("running".to_string()));
    let last_rt_loaded_str = status.last_rt_data_loaded.unwrap();
    let last_rt_loaded =
        NaiveDateTime::parse_from_str(&last_rt_loaded_str, DATETIME_FORMAT).unwrap();
    assert!(now_datetime > last_rt_loaded);
    // we don't have kirin for providing us a full real_time disruption reload
    assert_eq!(status.is_realtime_loaded, Some(false));
    assert_eq!(
        status.dataset_created_at,
        Some("20210101T120000.000000000".to_string())
    );
    assert_eq!(
        status.rt_contributors,
        vec!["test_realtime_topic".to_string()]
    );

    let metadatas = response.metadatas.unwrap();
    assert_eq!(metadatas.start_production_date, "20210101");
    assert_eq!(metadatas.end_production_date, "20210103");
    assert_eq!(metadatas.status, "running".to_string());
    assert_eq!(metadatas.timezone, Some("UTC".to_string()));
    assert_eq!(metadatas.name, Some("France - Ile-de-France".to_string()));
    assert_eq!(
        metadatas.dataset_created_at,
        Some("20210101T120000.000000000".to_string())
    );
    assert_eq!(metadatas.contributors, vec!["my_contributor".to_string()]);
    let last_load_at = NaiveDateTime::from_timestamp(metadatas.last_load_at.unwrap() as i64, 0);
    assert!(now_datetime > last_load_at);
}

pub async fn metadata_test(config: &ServerConfig) {
    let metadata_request = make_metadata_request();
    let response =
        crate::send_request_and_wait_for_response(&config.requests_socket, metadata_request).await;

    let now_datetime = Utc::now().naive_utc();

    let metadatas = response.metadatas.unwrap();
    assert_eq!(metadatas.start_production_date, "20210101");
    assert_eq!(metadatas.end_production_date, "20210103");
    assert_eq!(metadatas.status, "running".to_string());
    assert_eq!(metadatas.timezone, Some("UTC".to_string()));
    assert_eq!(metadatas.name, Some("France - Ile-de-France".to_string()));
    assert_eq!(metadatas.contributors, vec!["my_contributor".to_string()]);
    let last_load_at = NaiveDateTime::from_timestamp(metadatas.last_load_at.unwrap() as i64, 0);
    assert!(now_datetime > last_load_at);
}

fn make_status_request() -> navitia_proto::Request {
    let mut request = navitia_proto::Request::default();
    request.set_requested_api(navitia_proto::Api::Status);
    request
}

fn make_metadata_request() -> navitia_proto::Request {
    let mut request = navitia_proto::Request::default();
    request.set_requested_api(navitia_proto::Api::Metadatas);
    request
}
