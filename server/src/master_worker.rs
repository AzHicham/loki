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
    thread::{self},
};
use tokio::{runtime::Builder, sync::mpsc};

use launch::{
    config,
    loki::{
        timetables::PeriodicSplitVjByTzTimetables,
        tracing::{info, log::error},
        transit_model::Model,
        TransitData,
    },
};

use crate::{
    compute_worker::ComputeWorker,
    zmq_worker::{RequestMessage, ResponseMessage, ZmqWorker, ZmqWorkerChannels},
};

pub type MyTimetable = PeriodicSplitVjByTzTimetables;

#[derive(Debug, PartialEq, Eq)]
pub enum WorkerState {
    Available,
    Busy,
    Error,
}

#[derive(Debug, Clone, Copy)]
pub struct WorkerId {
    pub id: usize,
}

pub struct MasterWorker {
    data_and_model: Arc<RwLock<(TransitData<MyTimetable>, Model)>>,
    worker_request_senders: Vec<mpsc::Sender<RequestMessage>>,
    workers_response_receiver: mpsc::Receiver<(WorkerId, ResponseMessage)>,
    worker_states: Vec<WorkerState>,

    zmq_worker_handle: ZmqWorkerChannels,
}

impl MasterWorker {
    pub fn new(
        model: Model,
        data: TransitData<MyTimetable>,
        nb_workers: usize,
        zmq_endpoint: String,
        request_default_params: &config::RequestParams,
    ) -> Result<Self, Error> {
        let data_and_model = Arc::new(RwLock::new((data, model)));
        let mut worker_request_senders = Vec::new();
        let mut worker_states = Vec::new();

        let (workers_response_sender, workers_response_receiver) = mpsc::channel(1);

        for id in 0..nb_workers {
            let builder = thread::Builder::new().name(format!("loki_worker_{}", id));

            let worker_id = WorkerId { id };

            let (worker, request_channel) = ComputeWorker::new(
                worker_id,
                data_and_model.clone(),
                request_default_params.clone(),
                workers_response_sender.clone(),
            );
            let _thread_handle = builder.spawn(move || worker.run())?;
            worker_request_senders.push(request_channel);
            worker_states.push(WorkerState::Available);
        }

        let (zmq_worker, zmq_worker_handle) = ZmqWorker::new(zmq_endpoint);

        let _zmq_thread_handle = zmq_worker.run_in_a_thread()?;

        let result = Self {
            data_and_model,
            worker_request_senders,
            workers_response_receiver,
            worker_states,
            zmq_worker_handle,
        };
        Ok(result)
    }

    async fn run(mut self) {
        info!("Starting master worker");
        loop {
            let has_available_worker =
                self.worker_states
                    .iter()
                    .enumerate()
                    .find_map(|(id, state)| {
                        if *state == WorkerState::Available {
                            Some(id)
                        } else {
                            None
                        }
                    });

            info!("Master worker is waiting {:?}", has_available_worker);
            tokio::select! {
                // this indicates to tokio to poll the futures in the order they appears below
                // see https://docs.rs/tokio/1.12.0/tokio/macro.select.html#fairness
                // here use this give priority to forwarding responses from workers to zmq
                // receiving new requests from zmq has a lower priority
                //
                biased;

                // receive responses from worker threads
                has_response = self.workers_response_receiver.recv() => {
                    if let Some((worker_id, response)) = has_response {
                        info!("Master received response from worker {:?}", worker_id);
                        let forward_response_result = self.forward_worker_response(worker_id, response);
                        if let Err(err) = forward_response_result {
                            error!("Could not sent response to zmq worker : {}. I'll stop.", err);
                            break;
                        }
                    }
                    else {
                        error!("Channel to receive workers' responses has closed. I'll stop.");
                        break;
                    }

                }
                has_realtime = realtime_receiver.recv() => {
                    handle_realtime()
                }
                // receive requests from the zmq socket, and dispatch them to an available worker
                has_request = self.zmq_worker_handle.requests_receiver.recv(), if has_available_worker.is_some() => {
                    if let Some(request) = has_request {

                        // unwrap is safe here, because we enter this block only if has_available_worker.is_some()
                        let worker_id = has_available_worker.unwrap();
                        info!("Master is sending request to worker {:?}", worker_id);
                        let sender = &self.worker_request_senders[worker_id];
                        let forward_request_result = sender.send(request).await;
                        if let Err(err) = forward_request_result {
                            error!("Channel to forward request to worker {} has closed. I'll stop using this worker.", err);
                            self.worker_states[worker_id] = WorkerState::Error;
                            // TODO : here the request is lost and won't be answered.
                            // should we do something about it ?
                            // like try to find another available worker ?
                        }
                        else {
                            self.worker_states[worker_id] = WorkerState::Busy;
                        }
                    }
                    else {
                        error!("Channel to receive zmq requests has closed. I'll stop.");
                        break;
                    }



                }

            }
        }
    }

    fn forward_worker_response(
        &mut self,
        worker_id: WorkerId,
        response: ResponseMessage,
    ) -> Result<(), Error> {
        // let's forward to response to the zmq worker
        // who will forward it to the client
        let send_result = self.zmq_worker_handle.responses_sender.send(response);
        if let Err(err) = send_result {
            return Err(format_err!(
                "Channel to send responses to zmq worker has closed : {}",
                err
            ));
        }

        // let's mark the worker as available
        let worker_state = &mut self.worker_states[worker_id.id];
        match worker_state {
            WorkerState::Available | WorkerState::Error => {
                error!("I received a response from worker {}, but it is marked as {:?}. I'll stop using this worker.", worker_id.id, worker_state );
                *worker_state = WorkerState::Error;
            }
            WorkerState::Busy => {
                *worker_state = WorkerState::Available;
            }
        }
        Ok(())
    }

    // run by blocking the current thread
    pub fn run_blocking(self) -> Result<(), Error> {
        // copied from https://tokio.rs/tokio/topics/bridging#sending-messages

        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| format_err!("Failed to build tokio runtime. Error : {}", err))?;

        runtime.block_on(self.run());

        Ok(())
    }
}
