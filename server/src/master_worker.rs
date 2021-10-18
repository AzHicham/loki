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

use super::chaos_proto::gtfs_realtime;
use failure::{format_err, Error};
use launch::{
    config,
    loki::{
        timetables::PeriodicSplitVjByTzTimetables,
        tracing::{info, log::error},
        transit_model::Model,
        TransitData,
    },
};
use std::sync::{Arc, RwLock};
use tokio::{runtime::Builder, sync::mpsc};

use crate::load_balancer::LoadBalancer;
use crate::load_balancer::LoadBalancerState;
use crate::rabbitmq_worker::{BrokerConfig, RabbitMqWorker};

#[derive(Debug, PartialEq, Eq)]
pub enum LoadBalancerOrder {
    Start,
    Stop,
}

pub type MyTimetable = PeriodicSplitVjByTzTimetables;

pub struct LoadBalancerChannels {
    pub load_balancer_order_sender: mpsc::Sender<LoadBalancerOrder>,
    pub load_balancer_state_receiver: mpsc::Receiver<LoadBalancerState>,
}

pub struct MasterWorker {
    data_and_model: Arc<RwLock<(TransitData<MyTimetable>, Model)>>,
    amqp_message_receiver: mpsc::Receiver<Vec<gtfs_realtime::FeedMessage>>,
    load_balancer_handle: LoadBalancerChannels,
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

        // LoadBalancer worker
        let (load_balancer, load_balancer_handle) = LoadBalancer::new(
            data_and_model.clone(),
            nb_workers,
            zmq_endpoint,
            request_default_params,
        )?;
        let _load_balancer_thread_handle = load_balancer.run_in_a_thread()?;

        // AMQP worker
        let (amqp_message_sender, amqp_message_receiver) = mpsc::channel(1);
        let broker_config = BrokerConfig::default();
        let amqp_worker = RabbitMqWorker::new(&broker_config, amqp_message_sender)?;
        let _amqp_thread_handle = amqp_worker.run_in_a_thread()?;

        // Master worker
        let result = Self {
            data_and_model,
            amqp_message_receiver,
            load_balancer_handle,
        };
        Ok(result)
    }

    async fn run(mut self) {
        info!("Starting Master worker");
        loop {
            tokio::select! {

                // We receive protobuf from amqp broker
                has_proto_vec = self.amqp_message_receiver.recv() => {
                    if let Some(vec_protobuf) = has_proto_vec {
                        info!("Master received response from AmqpWorker");
                        // convert_realtime & stop the load balancer from receiving more request
                        let res = self.load_balancer_handle
                                    .load_balancer_order_sender
                                    .blocking_send(LoadBalancerOrder::Stop);
                        if let Err(err) = res {
                            error!("Could not sent Stop order to LoadBalancer : {}. I'll stop.", err);
                            break;
                        }
                    } else {
                        error!("Channel to receive realtime protobuf' responses has closed. I'll stop.");
                        break;
                    }
                }

                // We receive load balancer status, if load balancer is stopped
                // we apply realtime disruption on data
                has_load_balancer_status = self
                                            .load_balancer_handle
                                            .load_balancer_state_receiver
                                            .recv() => {
                    if let Some(load_balancer_status) = has_load_balancer_status {
                        info!("Master received LoadBalancer status");
                        // apply_realtime
                    } else {
                        error!("Channel to receive LoadBalancer status' responses has closed. I'll stop.");
                        break;
                    }
                }
            }
        }
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
