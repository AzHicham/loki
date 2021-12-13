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

use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

pub use loki_server;
use loki_server::{
    chaos_proto, master_worker::MasterWorker, navitia_proto, server_config::ServerConfig,
};
use prost::Message;
use protobuf::Message as ProtobuMessage;

use launch::loki::{chrono::Utc, tracing::info, NaiveDateTime, PositiveDuration};
use shiplift::builder::PullOptionsBuilder;

mod subtests;

#[test]
fn main() {
    launch::logger::init_global_test_logger();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(run())
}

async fn run() {
    let start_test_datetime = Utc::now().naive_utc();

    let working_dir = tempfile::tempdir().unwrap();
    let working_dir_path = working_dir.path();

    let data_dir_path = PathBuf::from_str(env!("CARGO_MANIFEST_DIR"))
        .unwrap()
        .join("tests")
        .join("a_small_ntfs");

    copy_ntfs(&data_dir_path, working_dir_path);

    let rabbitmq_endpoint = "amqp://guest:guest@localhost:5673";
    let input_data_path = working_dir_path.to_path_buf();
    let instance_name = "my_test_instance";
    let zmq_endpoint = "tcp://127.0.0.1:30001";

    let container_id = start_rabbitmq_docker().await;

    let mut config = ServerConfig::new(input_data_path, zmq_endpoint, instance_name);
    config.rabbitmq_params.rabbitmq_endpoint = rabbitmq_endpoint.to_string();
    config.rabbitmq_params.reload_kirin_timeout = PositiveDuration::from_hms(0, 0, 1);
    config
        .rabbitmq_params
        .rabbitmq_real_time_topics
        .push("test_realtime_topic".to_string());

    let _master_worker = MasterWorker::new(config.clone()).unwrap();

    wait_until_data_loaded_after(zmq_endpoint, &start_test_datetime).await;
    wait_until_connected_to_rabbitmq(zmq_endpoint).await;

    subtests::kirin_delete_vj_test::delete_vj_test(&config).await;

    // subtests::reload_test::reload_test(&config, &data_dir_path).await;

    info!("Everything went Ok ! Now stopping.");

    stop_rabbitmq_docker(&container_id).await;
    working_dir.close().unwrap();
}

fn copy_ntfs(from_dir: &Path, to_dir: &Path) {
    let files = vec![
        "calendar.txt",
        "commercial_modes.txt",
        "companies.txt",
        "contributors.txt",
        "datasets.txt",
        "feed_infos.txt",
        "lines.txt",
        "networks.txt",
        "physical_modes.txt",
        "routes.txt",
        "stop_times.txt",
        "stops.txt",
        "transfers.txt",
        "trips.txt",
    ];
    for file in &files {
        std::fs::copy(from_dir.join(file), to_dir.join(file)).unwrap();
    }
}

// launch a rabbitmq docker as
//
//   docker run -p 5673:5672 -p 15673:15672 rabbitmq:3-management
//
// management is available on http://localhost:15673
async fn start_rabbitmq_docker() -> String {
    let docker_image = "rabbitmq:3-management";

    let container_name = "rabbitmq_test_reload";

    let docker = shiplift::Docker::new();

    // let's pull the image from dockerhub
    {
        use futures::StreamExt;

        let pull_options = PullOptionsBuilder::default().image(docker_image).build();

        let mut stream = docker.images().pull(&pull_options);

        while let Some(pull_result) = stream.next().await {
            match pull_result {
                Ok(output) => {
                    info!("Pulled {:?} from docker hub.", output)
                }
                Err(e) => {
                    panic!("Error while pulling from dockerhub: {}", e);
                }
            }
        }
    }

    // if there was a problem at previous run, the docker container may still be running
    // so let's stop it if some is found
    {
        let old_container = docker.containers().get(container_name);
        let _ = old_container.stop(None).await;
        let _ = old_container.delete().await;
    }

    let options = shiplift::ContainerOptions::builder("rabbitmq:3-management")
        .expose(5672, "tcp", 5673)
        .expose(15672, "tcp", 15673)
        .name(container_name)
        .build();
    let id = docker.containers().create(&options).await.unwrap().id;

    docker.containers().get(&id).start().await.unwrap();

    id
}

async fn wait_until_connected_to_rabbitmq(zmq_endpoint: &str) {
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(60));
    tokio::pin!(timeout);
    let mut retry_interval = tokio::time::interval(std::time::Duration::from_secs(2));

    loop {
        retry_interval.tick().await;
        tokio::select! {
            status_response = send_status_request_and_wait_for_response(zmq_endpoint) => {
                if status_response.is_connected_to_rabbitmq.unwrap() {
                    return;
                }
            }
            _ = & mut timeout => {
                panic!("Not connected to rabbitmq before timeout.");
            }
        }
    }
}

async fn stop_rabbitmq_docker(container_id: &str) {
    let docker = shiplift::Docker::new();
    let container = docker.containers().get(container_id);
    container.stop(None).await.unwrap();
    container.delete().await.unwrap();
}

async fn wait_until_data_loaded_after(zmq_endpoint: &str, after_datetime: &NaiveDateTime) {
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(60));
    tokio::pin!(timeout);
    let mut retry_interval = tokio::time::interval(std::time::Duration::from_secs(2));

    loop {
        retry_interval.tick().await;
        tokio::select! {
            status_response = send_status_request_and_wait_for_response(zmq_endpoint) => {
                let has_datetime = status_response.last_load_at
                        .map(|datetime_str : String|
                            NaiveDateTime::parse_from_str(&datetime_str, "%Y%m%dT%H%M%S.%f").unwrap()
                        );
                // info!("Status request responded with last_load_at : {:?}. Reload should be after {}", has_datetime, after_datetime);
                if let Some(datetime) = has_datetime {
                    if datetime > *after_datetime {
                        return ;
                    }
                }
            }
            _ = & mut timeout => {
                panic!("Data not reloaded before timeout.");
            }
        }
    }
}

async fn wait_until_realtime_updated_after(zmq_endpoint: &str, after_datetime: &NaiveDateTime) {
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(60));
    tokio::pin!(timeout);
    let mut retry_interval = tokio::time::interval(std::time::Duration::from_secs(2));

    loop {
        retry_interval.tick().await;
        tokio::select! {
            status_response = send_status_request_and_wait_for_response(zmq_endpoint) => {
                let has_datetime = status_response.last_rt_data_loaded
                        .map(|datetime_str : String|
                            NaiveDateTime::parse_from_str(&datetime_str, "%Y%m%dT%H%M%S.%f").unwrap()
                        );
                // info!("Status request responded with last_load_at : {:?}. Reload should be after {}", has_datetime, after_datetime);
                if let Some(datetime) = has_datetime {
                    if datetime > *after_datetime {
                        return ;
                    }
                }
            }
            _ = & mut timeout => {
                panic!("Data not reloaded before timeout.");
            }
        }
    }
}

async fn send_realtime_message(
    config: &ServerConfig,
    realtime_message: chaos_proto::gtfs_realtime::FeedMessage,
) {
    // connect to rabbitmq
    let connection = lapin::Connection::connect(
        &config.rabbitmq_params.rabbitmq_endpoint,
        lapin::ConnectionProperties::default(),
    )
    .await
    .unwrap();
    let channel = connection.create_channel().await.unwrap();

    let mut payload = Vec::new();
    realtime_message.write_to_vec(&mut payload).unwrap();

    let routing_key = &config.rabbitmq_params.rabbitmq_real_time_topics[0];
    channel
        .basic_publish(
            &config.rabbitmq_params.rabbitmq_exchange,
            &routing_key,
            lapin::options::BasicPublishOptions::default(),
            payload,
            lapin::BasicProperties::default(),
        )
        .await
        .unwrap()
        .await
        .unwrap();

    info!("Sent realtime message with routing key {}.", routing_key);
}

async fn send_status_request_and_wait_for_response(zmq_endpoint: &str) -> navitia_proto::Status {
    let mut status_request = navitia_proto::Request::default();
    status_request.set_requested_api(navitia_proto::Api::Status);

    let proto_response = send_request_and_wait_for_response(zmq_endpoint, status_request).await;
    proto_response.status.unwrap()
}

async fn send_request_and_wait_for_response(
    zmq_endpoint: &str,
    request: navitia_proto::Request,
) -> navitia_proto::Response {
    let context = tmq::Context::new();
    let zmq_socket = tmq::request(&context).connect(zmq_endpoint).unwrap();

    // cf https://github.com/cetra3/tmq/blob/master/examples/request.rs
    let zmq_message = tmq::Message::from(request.encode_to_vec());

    let recv_socket = zmq_socket.send(zmq_message.into()).await.unwrap();
    let (mut reply, _) = recv_socket.recv().await.unwrap();
    let reply_payload = reply.pop_back().unwrap();

    navitia_proto::Response::decode(&*reply_payload).unwrap()
}

fn make_journeys_request(
    from_stop_point: &str,
    to_stop_point: &str,
    from_datetime: NaiveDateTime,
) -> navitia_proto::Request {
    let origin = navitia_proto::LocationContext {
        place: from_stop_point.to_string(),
        ..Default::default()
    };
    let destination = navitia_proto::LocationContext {
        place: to_stop_point.to_string(),
        ..Default::default()
    };

    let journeys = navitia_proto::JourneysRequest {
        origin: vec![origin],
        destination: vec![destination],
        datetimes: vec![from_datetime.timestamp() as u64],
        clockwise: true,
        max_duration: 24 * 60 * 60, // 1 day
        ..Default::default()
    };

    let mut request = navitia_proto::Request {
        journeys: Some(journeys),
        ..Default::default()
    };
    request.set_requested_api(navitia_proto::Api::PtPlanner);
    request
}
