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
pub use loki_server;
use loki_server::master_worker::MasterWorker;
use loki_server::navitia_proto;
use loki_server::server_config::ServerConfig;
use prost::Message;

use failure::Error;
use lapin::options::BasicPublishOptions;
use lapin::BasicProperties;
use launch::loki::tracing::info;
use launch::loki::PositiveDuration;

fn main() -> Result<(), Error> {
    // first launch a rabbitmq docker with
    // docker run -p 5673:5672 -p 15673:15672 rabbitmq:3-management
    // management is available on http://localhost:15673

    let _log_guard = launch::logger::init_logger();
    let rabbitmq_endpoint = "amqp://guest:guest@localhost:5673";
    let input_data_path = "/home/pascal/loki/data/idfm/ntfs/";
    let instance_name = "my_test_instance";
    let zmq_socket = "tcp://*:30001";
    let mut config = ServerConfig::new(input_data_path, zmq_socket, instance_name);
    config.rabbitmq_params.rabbitmq_endpoint = rabbitmq_endpoint.to_string();
    config.rabbitmq_params.reload_kirin_timeout = PositiveDuration::from_hms(0, 0, 1);

    let _master_worker = MasterWorker::new(config.clone()).unwrap();

    std::thread::sleep(std::time::Duration::from_secs(30));

    send_reload_order(&config).unwrap();

    loop {}
}

pub fn send_reload_order(config: &ServerConfig) -> Result<(), Error> {
    // connect to rabbitmq
    let connection = lapin::Connection::connect(
        &config.rabbitmq_params.rabbitmq_endpoint,
        lapin::ConnectionProperties::default(),
    )
    .wait()?;
    let channel = connection.create_channel().wait()?;

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
        .wait()?
        .wait()?;

    info!("Reload message published with routing key {}.", routing_key);
    Ok(())
}
