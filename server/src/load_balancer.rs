// Copyright  (C) 2020, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Hove (www.kisio.com).
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

use anyhow::{format_err, Context, Error};
use launch::{
    config,
    loki::{
        models::{base_model::BaseModel, real_time_model::RealTimeModel},
        tracing::{error, info, log::trace},
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
    master_worker::DataAndModels,
    zmq_worker::{LoadBalancerToZmqChannels, RequestMessage, ResponseMessage},
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

    order_receiver: mpsc::Receiver<LoadBalancerOrder>,
    stopped_sender: mpsc::Sender<()>,

    shutdown_sender: mpsc::Sender<()>,

    state: LoadBalancerState,

    zmq_channels: LoadBalancerToZmqChannels,
}

pub struct LoadBalancerChannels {
    pub order_sender: mpsc::Sender<LoadBalancerOrder>,
    pub stopped_receiver: mpsc::Receiver<()>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadBalancerOrder {
    Start,
    Stop,
}

impl LoadBalancer {
    pub fn new(
        data_and_models: Arc<RwLock<DataAndModels>>,
        nb_workers: u16,
        default_request_params: &config::RequestParams,
        zmq_channels: LoadBalancerToZmqChannels,
        shutdown_sender: mpsc::Sender<()>,
    ) -> Result<(Self, LoadBalancerChannels), Error> {
        let mut worker_request_senders = Vec::new();
        let mut worker_states = Vec::new();

        // Compute workers
        let (workers_response_sender, workers_response_receiver) = mpsc::channel(1);
        for id in 0..nb_workers {
            let builder = thread::Builder::new().name(format!("loki_worker_{}", id));

            let worker_id = WorkerId {
                id: usize::from(id),
            };

            let (worker, request_channel) = ComputeWorker::new(
                worker_id,
                data_and_models.clone(),
                default_request_params.clone(),
                workers_response_sender.clone(),
            );
            let _thread_handle = builder.spawn(move || worker.run())?;
            worker_request_senders.push(request_channel);
            worker_states.push(WorkerState::Available);
        }

        // // ZMQ worker
        // let (zmq_worker, zmq_worker_handle) = ZmqWorker::new(zmq_endpoint, shutdown_sender.clone());
        // let _zmq_thread_handle = zmq_worker.run_in_a_thread()?;

        let (order_sender, order_receiver) = mpsc::channel(1);
        let (stopped_sender, stopped_receiver) = mpsc::channel(1);

        let load_balancer_handle = LoadBalancerChannels {
            order_sender,
            stopped_receiver,
        };

        let result = Self {
            worker_request_senders,
            workers_response_receiver,
            worker_states,
            order_receiver,
            stopped_sender,
            shutdown_sender,
            state: LoadBalancerState::Online,
            zmq_channels,
        };
        Ok((result, load_balancer_handle))
    }

    // run in a spawned thread
    pub fn run_in_a_thread(self) -> Result<std::thread::JoinHandle<()>, Error> {
        // copied from https://tokio.rs/tokio/topics/bridging#sending-messages

        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Failed to build tokio runtime.")?;

        let thread_builder = thread::Builder::new().name("loki_load_balancer".to_string());
        let handle = thread_builder.spawn(move || runtime.block_on(self.run()))?;
        Ok(handle)
    }

    async fn run(mut self) {
        let err = self.main_loop().await;

        error!(
            "Load balancer main loop exited. I'll tell Master Worker about it. {:?}",
            err
        );
        // If we exited the loop it means we got an error
        // We need to warn Master that we are broken
        // So Master can shutdown the program
        let res = self.shutdown_sender.send(()).await;
        if let Err(err) = res {
            error!(
                "Channel shutdown_sender to Master has closed. Load balancer will die, and Master will not known about it. {:?}",
                err
            );
        }
    }

    async fn main_loop(&mut self) -> Result<(), Error> {
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

            trace!(
                "LoadBalancer worker is waiting. Available worker : {:?}",
                has_available_worker
            );
            tokio::select! {
                // this indicates to tokio to poll the futures in the order they appears below
                // see https://docs.rs/tokio/1.12.0/tokio/macro.select.html#fairness
                // here this give priority to forwarding responses from workers to zmq
                // over receiving new requests from zmq
                biased;

                // receive responses from worker threads
                has_response = self.workers_response_receiver.recv() => {

                    let (worker_id, response) = has_response.ok_or_else(|| format_err!("Channel to receive workers' responses has closed."))?;

                    trace!("LoadBalancer received response from worker {:?}", worker_id);

                     self.forward_worker_response(worker_id, response)
                            .context("Could not send response to zmq worker")?;


                    if self.state == LoadBalancerState::Stopping {
                        self.stop_if_all_workers_available().await?;
                    }

                }
                // receive order from the Master
                has_order = self.order_receiver.recv() => {
                    let order = has_order.ok_or_else(|| format_err!("Channel to receive load balancer order has closed."))?;
                    trace!("LoadBalancer received order {:?}", order);
                    match (&order, self.state) {
                        (LoadBalancerOrder::Start, LoadBalancerState::Stopped) => {
                            self.state = LoadBalancerState::Online;
                            trace!("LoadBalancer is now back Online.");
                        },
                        (LoadBalancerOrder::Stop, LoadBalancerState::Online) => {
                            self.state =LoadBalancerState::Stopping;
                            trace!("LoadBalancer is now Stopping.");

                           self.stop_if_all_workers_available().await?;
                        },
                        _ => {
                           error!("Load balancer received order {:?} while being in state {:?}. \
                                    I don't know what to do, so I'll ignore this order.", order , &self.state);
                        }

                    }
                }
                //receive requests from the zmq socket, and dispatch them to an available worker
                has_request = self.zmq_channels.requests_receiver.recv(),
                if has_available_worker.is_some() && self.state == LoadBalancerState::Online => {
                    let request = has_request.ok_or_else(|| format_err!("Channel to receive zmq requests has closed."))?;
                    trace!("Load Balancer received a request.");
                    // unwrap is safe here, because we enter this block only if has_available_worker.is_some()
                    let worker_id = has_available_worker.unwrap();
                    trace!("LoadBalancer is sending request to worker {:?}", worker_id);
                    let sender = &self.worker_request_senders[worker_id];
                    sender.send(request).await
                                .with_context(||format!("Channel to forward request to worker {} has closed", worker_id))?;

                    self.worker_states[worker_id] = WorkerState::Busy;
                }
            }
        }
    }

    async fn stop_if_all_workers_available(&mut self) -> Result<(), Error> {
        let all_workers_available = self
            .worker_states
            .iter()
            .all(|state| *state == WorkerState::Available);
        if all_workers_available {
            self.state = LoadBalancerState::Stopped;
            self.stopped_sender
                .send(())
                .await
                .context("Channel load_balancer_stopped has closed")
        } else {
            Ok(())
        }
    }

    fn forward_worker_response(
        &mut self,
        worker_id: WorkerId,
        response: ResponseMessage,
    ) -> Result<(), Error> {
        // let's forward to response to the zmq worker
        // who will forward it to the client
        self.zmq_channels
            .responses_sender
            .send(response)
            .context("Channel to send responses to zmq worker has closed")?;

        // let's mark the worker as available
        let worker_state = &mut self.worker_states[worker_id.id];
        match worker_state {
            WorkerState::Busy => {
                *worker_state = WorkerState::Available;
            }
            WorkerState::Available => {
                error!(
                    "I received a response from worker {}, but it is marked as {:?}.",
                    worker_id.id, worker_state
                );
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
            .context("Failed to build tokio runtime.")?;

        runtime.block_on(self.run());

        Ok(())
    }
}
