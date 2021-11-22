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
use launch::loki::{
    chrono,
    models::{base_model::BaseModel, real_time_model::RealTimeModel},
    timetables::{DailyTimetables, PeriodicSplitVjByTzTimetables},
    tracing::{debug, error, info},
    DataTrait, LoadsData, TransitData,
};
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock},
};
use tokio::{
    runtime::Builder,
    sync::{mpsc, mpsc::error::SendError},
};

use crate::{
    handle_kirin_message::handle_kirin_protobuf,
    load_balancer::{LoadBalancer, LoadBalancerState},
    rabbitmq_worker::listen_amqp_in_a_thread,
    Config,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadBalancerOrder {
    Start,
    Stop,
}

pub type BaseTimetable = PeriodicSplitVjByTzTimetables;
pub type RealTimeTimetable = DailyTimetables;

pub struct LoadBalancerChannels {
    pub load_balancer_order_sender: mpsc::Sender<LoadBalancerOrder>,
    pub load_balancer_state_receiver: mpsc::Receiver<LoadBalancerState>,
    pub load_balancer_error_receiver: mpsc::Receiver<()>,
}

pub struct MasterWorker {
    base_data_and_model: Arc<RwLock<(TransitData<BaseTimetable>, BaseModel)>>,
    real_time_data_and_model: Arc<RwLock<(TransitData<RealTimeTimetable>, RealTimeModel)>>,
    loads_data: LoadsData,
    amqp_message_receiver: mpsc::Receiver<Vec<gtfs_realtime::FeedMessage>>,
    load_balancer_handle: LoadBalancerChannels,
    nb_of_realtime_days_to_keep: u16,
}

impl MasterWorker {
    pub fn new(config: &Config) -> Result<Self, Error> {
        let launch_params = &config.launch_params;
        let base_model = launch::read::read_model(launch_params)?;
        info!("Base model loaded");
        info!("Starting to build base data");
        let loads_data = launch::read::read_loads_data(launch_params, &base_model);
        let base_data = launch::read::build_transit_data::<BaseTimetable>(
            &base_model,
            &loads_data,
            &launch_params.default_transfer_duration,
            None,
        );
        info!("Base data loaded");
        let restrict_calendar_for_realtime = {
            let start_date = *base_data.calendar().first_date();
            let end_date = start_date + chrono::Duration::days(2);
            Some((start_date, end_date))
        };

        info!("Starting to build real time data");
        let real_time_model = RealTimeModel::new();
        let real_time_data = launch::read::build_transit_data::<RealTimeTimetable>(
            &base_model,
            &loads_data,
            &launch_params.default_transfer_duration,
            restrict_calendar_for_realtime,
        );
        info!("Real time data loaded");

        info!("Starting to build workers");

        let base_data_and_model = Arc::new(RwLock::new((base_data, base_model)));
        let real_time_data_and_model = Arc::new(RwLock::new((real_time_data, real_time_model)));

        // LoadBalancer worker
        let (load_balancer, load_balancer_handle) = LoadBalancer::new(
            base_data_and_model.clone(),
            real_time_data_and_model.clone(),
            config.nb_workers,
            &config.requests_socket,
            &config.request_default_params,
        )?;
        let _load_balancer_thread_handle = load_balancer.run_in_a_thread()?;

        // AMQP worker
        let (amqp_message_sender, amqp_message_receiver) = mpsc::channel(1);
        let _amqp_thread_handle =
            listen_amqp_in_a_thread(config.amqp_params.clone(), amqp_message_sender);

        info!("Workers built");

        // Master worker
        let result = Self {
            base_data_and_model,
            real_time_data_and_model,
            loads_data,
            amqp_message_receiver,
            load_balancer_handle,
            nb_of_realtime_days_to_keep: config.nb_of_realtime_days_to_keep,
        };
        Ok(result)
    }

    async fn run(mut self) -> Result<(), Error> {
        info!("Starting Master worker");
        loop {
            tokio::select! {
                has_proto_vec = self.amqp_message_receiver.recv() => {
                    let vec_protobuf = has_proto_vec
                        .ok_or_else(||
                            format_err!("Channel to receive realtime protobuf' responses has closed. I'll stop.")
                        )?;

                    info!("Master received response from AmqpWorker");

                    // stop the load balancer from receiving more request
                    debug!("Master ask LoadBalancer to Stop");
                    self.send_order_to_load_balancer(LoadBalancerOrder::Stop)
                        .await
                        .map_err(|err| format_err!("Master could not send Stop order to load balancer. {}", err))?;


                    // Wait for the LoadBalancer to Stop
                    debug!("MasterWork waiting for LoadBalancer to Stop");
                    let load_balancer_state = self
                        .load_balancer_handle
                        .load_balancer_state_receiver
                        .recv()
                        .await
                        .ok_or_else(|| format_err!("Channel to receive LoadBalancer status responses has closed."))?;

                    if load_balancer_state == LoadBalancerState::Stopped {

                        debug!("Master start handling real time messages.");
                        self.handle_realtime_messages(vec_protobuf)
                            .map_err(|err| format_err!("Error while handling real time messages : {}", err))?;

                        debug!("Master has finished handling real time messages.");
                        debug!("Master ask LoadBalancer to Start");
                        self.send_order_to_load_balancer(LoadBalancerOrder::Start)
                            .await
                            .map_err(|err| format_err!("Master could not send Start order to load balancer. {}", err))?;
                    } else {
                        error!("Master requested LoadBalancer to Stop, but it is in state {:?}.", load_balancer_state);
                        break;
                    }

                }
                _has_load_balancer_error = self
                    .load_balancer_handle
                    .load_balancer_error_receiver
                    .recv() => {
                        // We don't even need to know if _has_load_balancer_error is None or not
                        // is LoadBalancer send an Error we must shutdown
                        error!("Load Balancer is broken, exit program safely");
                        break;
                    }
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

        runtime.block_on(self.run())
    }

    async fn send_order_to_load_balancer(
        &mut self,
        order: LoadBalancerOrder,
    ) -> Result<(), SendError<LoadBalancerOrder>> {
        let res = self
            .load_balancer_handle
            .load_balancer_order_sender
            .send(order.clone())
            .await;
        if let Err(err) = &res {
            error!(
                "Could not sent {:?} order to LoadBalancer : {}. I'll stop.",
                order, err
            );
        };
        res
    }

    fn handle_realtime_messages(
        &mut self,
        messages: Vec<gtfs_realtime::FeedMessage>,
    ) -> Result<(), Error> {
        let base_lock_guard = self.base_data_and_model.read().map_err(|err| {
            format_err!(
                "Master worker failed to acquire read lock on base_time_data_and_model. {}.",
                err
            )
        })?;

        let mut real_time_lock_guard = self.real_time_data_and_model.write().map_err(|err| {
            format_err!(
                "Master worker failed to acquire write lock on real_time_data_and_model. {}.",
                err
            )
        })?;

        let (_, base_model) = base_lock_guard.deref();
        let (real_time_data, real_time_model) = real_time_lock_guard.deref_mut();

        for message in messages.into_iter() {
            for feed_entity in message.entity {
                let disruption_result = handle_kirin_protobuf(&feed_entity);
                match disruption_result {
                    Err(err) => {
                        error!("Could not handle a kirin message {}", err);
                    }
                    Ok(disruption) => {
                        real_time_model.apply_disruption(
                            &disruption,
                            base_model,
                            &self.loads_data,
                            real_time_data,
                            self.nb_of_realtime_days_to_keep,
                        );
                    }
                }
            }
        }

        Ok(())
        // RwLocks are released here
    }
}
