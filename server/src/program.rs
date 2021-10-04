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

use failure::{format_err, Error};
use std::{
    sync::{Arc, RwLock},
    thread::{self, JoinHandle},
};

use launch::{
    loki::{timetables::PeriodicSplitVjByTzTimetables, transit_model::Model, TransitData},
    solver::Solver,
};

use super::worker::Worker;

pub type MyTimetable = PeriodicSplitVjByTzTimetables;

const WORKERS_SOCKET_ADDRESS: &str = "inproc://loki_workers";

pub struct Program {
    model: Model,
    data: Arc<RwLock<TransitData<MyTimetable>>>,
    worker_threads: Vec<JoinHandle<()>>,
    nb_available_workers: usize,
    nb_workers: usize,
    zmq_context: zmq::Context,
    basic_requests_socket: zmq::Socket,
    workers_socket: zmq::Socket,
}

impl Program {
    pub fn new(
        model: Model,
        data: TransitData<MyTimetable>,
        nb_workers: usize,
        basic_requests_socket_address: &str,
    ) -> Result<Self, Error> {
        let data = Arc::new(RwLock::new(data));
        let mut worker_threads = Vec::new();
        let zmq_context = zmq::Context::new();

        // socket used to :
        //  - listen to incoming requests from the outside
        //  - reply with the response computed by a worker
        let basic_requests_socket = zmq_context.socket(zmq::ROUTER).map_err(|err| {
            format_err!(
                "Main thread could not create incoming requests socket. Error : {}",
                err
            )
        })?;

        basic_requests_socket
            .bind(&basic_requests_socket_address)
            .map_err(|err| {
                format_err!(
                    "Main thread could not bind incoming requests socket {}. Error : {}",
                    basic_requests_socket_address,
                    err
                )
            })?;

        // socket used to
        //  - send requests to workers
        //  - read responses computed by the workers
        let workers_socket = zmq_context.socket(zmq::ROUTER).map_err(|err| {
            format_err!(
                "Main thread could not create the socket to communicate with workers. Error : {}",
                err
            )
        })?;

        basic_requests_socket
            .bind(&WORKERS_SOCKET_ADDRESS)
            .map_err(|err| {
                format_err!(
                    "Main thread could not bind socket to communicate with workers at {}. Error : {}",
                    WORKERS_SOCKET_ADDRESS,
                    err
                )
            })?;

        for worker_id in 0..nb_workers {
            let builder = thread::Builder::new().name(format!("loki_worker_{}", worker_id));

            let worker = Worker::new(
                data.clone(),
                &zmq_context,
                &WORKERS_SOCKET_ADDRESS,
                worker_id,
            )?;
            let handler = builder.spawn(|| {})?;
            worker_threads.push(handler);
        }

        let result = Self {
            model,
            data,
            nb_available_workers: nb_workers,
            nb_workers,
            worker_threads,
            zmq_context,
        };
        Ok(result)
    }
}
