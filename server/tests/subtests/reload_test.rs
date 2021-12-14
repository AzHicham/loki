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

use std::path::Path;

pub use loki_server;
use loki_server::{navitia_proto, server_config::ServerConfig};
use prost::Message;

use lapin::{options::BasicPublishOptions, BasicProperties};
use launch::loki::{chrono::Utc, tracing::info, NaiveDateTime};

// changes the ntfs file on disk, send a reload order, and check
// that the new data is indeed loaded
pub async fn reload_test(config: &ServerConfig, data_dir_path: &Path) {
    let datetime =
        NaiveDateTime::parse_from_str("2021-01-01 08:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

    let journeys_request =
        crate::make_journeys_request("stop_point:massy", "stop_point:paris", datetime);

    let journeys_response = crate::send_request_and_wait_for_response(
        &config.requests_socket,
        journeys_request.clone(),
    )
    .await;
    // info!("{:#?}", journeys_response);
    // check that we have a journey, that uses the only trip in the ntfs, with headsign "Hello"
    assert_eq!(
        journeys_response.journeys[0].sections[0]
            .pt_display_informations
            .as_ref()
            .unwrap()
            .headsign
            .as_ref()
            .unwrap()
            .as_str(),
        "Hello"
    );

    crate::wait_until_connected_to_rabbitmq(&config.requests_socket).await;

    // copy the modified trips.txt into working dir
    std::fs::copy(
        data_dir_path.join("trips_renamed.txt"),
        config.launch_params.input_data_path.join("trips.txt"),
    )
    .unwrap();

    let before_reload_datetime = Utc::now().naive_utc();
    send_reload_order(&config).await;

    crate::wait_until_data_loaded_after(&config.requests_socket, &before_reload_datetime).await;

    let journeys_response =
        crate::send_request_and_wait_for_response(&config.requests_socket, journeys_request).await;
    // check that we have a journey, that uses the only trip in the ntfs,  now with headsign "Hello Renamed"
    assert_eq!(
        journeys_response.journeys[0].sections[0]
            .pt_display_informations
            .as_ref()
            .unwrap()
            .headsign
            .as_ref()
            .unwrap()
            .as_str(),
        "Hello Renamed"
    );
}

async fn send_reload_order(config: &ServerConfig) {
    // connect to rabbitmq
    let connection = lapin::Connection::connect(
        &config.rabbitmq_params.rabbitmq_endpoint,
        lapin::ConnectionProperties::default(),
    )
    .await
    .unwrap();
    let channel = connection.create_channel().await.unwrap();

    let mut task = navitia_proto::Task::default();
    task.set_action(navitia_proto::Action::Reload);
    let payload = task.encode_to_vec();

    let routing_key = format!("{}.task.reload", &config.instance_name);
    channel
        .basic_publish(
            &config.rabbitmq_params.rabbitmq_exchange,
            &routing_key,
            BasicPublishOptions::default(),
            payload,
            BasicProperties::default(),
        )
        .await
        .unwrap()
        .await
        .unwrap();

    info!("Reload message published with routing key {}.", routing_key);
}
