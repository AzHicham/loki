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
use launch::{
    config,
    loki::{
        models::{base_model::BaseModel, real_time_model::RealTimeModel},
        tracing::{debug, error, info, warn},
        TransitData,
    },
};
use std::{
    sync::{Arc, RwLock},
    thread::{self},
};
use tokio::{runtime::Builder, sync::mpsc};

use crate::{
    compute_worker::ComputeWorker,
    master_worker::{BaseTimetable, LoadBalancerChannels, LoadBalancerOrder, RealTimeTimetable},
    zmq_worker::{RequestMessage, ResponseMessage, ZmqWorker, ZmqWorkerChannels},
};

#[derive(Debug, PartialEq, Eq)]
pub enum WorkerState {
    Available,
    Busy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadBalancerState {
    Online,   // accepting requests
    Stopping, // not accepting new requests, but there is still some requests being processed by workers
    Stopped,  // not accepting new requests, and all workers are idle
}

#[derive(Debug, Clone, Copy)]
pub struct WorkerId {
    pub id: usize,
}

pub struct LoadBalancer {
    worker_request_senders: Vec<mpsc::Sender<RequestMessage>>,
    workers_response_receiver: mpsc::Receiver<(WorkerId, ResponseMessage)>,
    worker_states: Vec<WorkerState>,

    load_balancer_order_receiver: mpsc::Receiver<LoadBalancerOrder>,
    load_balancer_state_sender: mpsc::Sender<LoadBalancerState>,
    load_balancer_error_sender: mpsc::Sender<()>,
    state: LoadBalancerState,

    zmq_worker_handle: ZmqWorkerChannels,
}

impl LoadBalancer {
    pub fn new(
        base_data_and_model: Arc<RwLock<(TransitData<BaseTimetable>, BaseModel)>>,
        real_time_data_and_model: Arc<RwLock<(TransitData<RealTimeTimetable>, RealTimeModel)>>,
        nb_workers: usize,
        zmq_endpoint: &str,
        request_default_params: &config::RequestParams,
    ) -> Result<(Self, LoadBalancerChannels), Error> {
        let mut worker_request_senders = Vec::new();
        let mut worker_states = Vec::new();

        // Compute workers
        let (workers_response_sender, workers_response_receiver) = mpsc::channel(1);
        for id in 0..nb_workers {
            let builder = thread::Builder::new().name(format!("loki_worker_{}", id));

            let worker_id = WorkerId { id };

            let (worker, request_channel) = ComputeWorker::new(
                worker_id,
                base_data_and_model.clone(),
                real_time_data_and_model.clone(),
                request_default_params.clone(),
                workers_response_sender.clone(),
            );
            let _thread_handle = builder.spawn(move || worker.run())?;
            worker_request_senders.push(request_channel);
            worker_states.push(WorkerState::Available);
        }

        // ZMQ worker
        let (zmq_worker, zmq_worker_handle) = ZmqWorker::new(zmq_endpoint);
        let _zmq_thread_handle = zmq_worker.run_in_a_thread()?;

        let (load_balancer_order_sender, load_balancer_order_receiver) = mpsc::channel(1);
        let (load_balancer_state_sender, load_balancer_state_receiver) = mpsc::channel(1);
        let (load_balancer_error_sender, load_balancer_error_receiver) = mpsc::channel(1);

        // Master worker
        let load_balancer_handle = LoadBalancerChannels {
            load_balancer_order_sender,
            load_balancer_state_receiver,
            load_balancer_error_receiver,
        };

        let result = Self {
            worker_request_senders,
            workers_response_receiver,
            worker_states,
            load_balancer_order_receiver,
            load_balancer_state_sender,
            load_balancer_error_sender,
            state: LoadBalancerState::Online,
            zmq_worker_handle,
        };
        Ok((result, load_balancer_handle))
    }

    // run in a spawned thread
    pub fn run_in_a_thread(self) -> Result<std::thread::JoinHandle<()>, Error> {
        // copied from https://tokio.rs/tokio/topics/bridging#sending-messages

        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| format_err!("Failed to build tokio runtime. Error : {}", err))?;

        let thread_builder = thread::Builder::new().name("loki_zmq_worker".to_string());
        let handle = thread_builder.spawn(move || runtime.block_on(self.run()))?;
        Ok(handle)
    }

    async fn run(mut self) {
        info!("Starting LoadBalancer worker");
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

            // if state is Stopping and all workers are available
            // change state to Stopped and
            // we inform master that we (LoadBalancer) stopped successfully
            let all_workers_available = self
                .worker_states
                .iter()
                .all(|state| *state == WorkerState::Available);
            if all_workers_available && self.state == LoadBalancerState::Stopping {
                self.state = LoadBalancerState::Stopped;
                debug!("LoadBalancer new state : {:?}", self.state);
                let res = self.load_balancer_state_sender.send(self.state).await;
                if let Err(err) = res {
                    error!(
                        "Channel to send LoadBalancer state to Master has closed : {}",
                        err
                    );
                }
            }

            info!("LoadBalancer worker is waiting {:?}", has_available_worker);
            tokio::select! {
                // this indicates to tokio to poll the futures in the order they appears below
                // see https://docs.rs/tokio/1.12.0/tokio/macro.select.html#fairness
                // here use this give priority to forwarding responses from workers to zmq
                // receiving new requests from zmq has a lower priority
                biased;

                // receive responses from worker threads
                has_response = self.workers_response_receiver.recv() => {
                    if let Some((worker_id, response)) = has_response {
                        info!("LoadBalancer received response from worker {:?}", worker_id);
                        let forward_response_result = self.forward_worker_response(worker_id, response).await;
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
                has_order = self.load_balancer_order_receiver.recv() => {
                    if let Some(order) = has_order {
                        if order == LoadBalancerOrder::Start && self.state != LoadBalancerState::Online {
                            self.state = LoadBalancerState::Online;
                            debug!("LoadBalancer new state : {:?}", self.state);
                        }
                        else if order == LoadBalancerOrder::Stop && self.state == LoadBalancerState::Online {
                            self.state = LoadBalancerState::Stopping;
                            debug!("LoadBalancer new state : {:?}", self.state);
                        }
                        else{
                            warn!("We received an order to {:?}, but we already are in this state : {:?}", order, self.state);
                        }
                    }
                    else {
                        error!("Channel to receive Master Worker' order has closed. I'll stop.");
                        break;
                    }
                }
                //receive requests from the zmq socket, and dispatch them to an available worker
                has_request = self.zmq_worker_handle.requests_receiver.recv(),
                if has_available_worker.is_some() && self.state == LoadBalancerState::Online => {
                    if let Some(request) = has_request {

                        // unwrap is safe here, because we enter this block only if has_available_worker.is_some()
                        let worker_id = has_available_worker.unwrap();
                        info!("LoadBalancer is sending request to worker {:?}", worker_id);
                        let sender = &self.worker_request_senders[worker_id];
                        let forward_request_result = sender.send(request).await;
                        if let Err(err) = forward_request_result {
                            error!("Channel to forward request to worker {} has closed. I'll stop using this worker.", err);
                            break;
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

        // If we exited the loop it means we got an error
        // We need to warn Master worker that we are broken
        // So MasterWorker can shutdown the program
        debug!("LoadBalancer : Send error to MasterWorker & exiting thread");
        let res = self.load_balancer_error_sender.send(()).await;
        if let Err(err) = res {
            error!(
                "Channel to send LoadBalancer state to Master has closed : {}",
                err
            );
        }
    }

    async fn forward_worker_response(
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
            WorkerState::Busy => {
                *worker_state = WorkerState::Available;
            }
            WorkerState::Available => {
                let error = format_err!(
                    "I received a response from worker {}, but it is marked as {:?}. I'll stop using this worker.", worker_id.id, worker_state
                );
                return Err(error);
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
