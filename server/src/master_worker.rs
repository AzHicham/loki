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
use failure::{bail, format_err, Error};
use launch::loki::{
    models::{base_model::BaseModel, real_time_model::RealTimeModel},
    timetables::PeriodicSplitVjByTzTimetables,
    tracing::{debug, error, info},
    TransitData,
};
use std::sync::{Arc, RwLock};
use tokio::{
    runtime::Builder,
    sync::{mpsc, mpsc::error::SendError},
};

use crate::{
    handle_kirin_message::handle_kirin_protobuf, load_balancer::LoadBalancer,
    rabbitmq_worker::RabbitMqWorker, Config,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadBalancerOrder {
    Start,
    Stop,
}

pub type Timetable = PeriodicSplitVjByTzTimetables;

pub struct LoadBalancerChannels {
    pub order_sender: mpsc::Sender<LoadBalancerOrder>,
    pub stopped_receiver: mpsc::Receiver<()>,
    pub error_receiver: mpsc::Receiver<()>,
}

pub type DataAndModels = (TransitData<Timetable>, BaseModel, RealTimeModel);

pub struct MasterWorker {
    config: Config,
    data_and_models: Arc<RwLock<DataAndModels>>,
    kirin_messages_receiver: mpsc::Receiver<Vec<gtfs_realtime::FeedMessage>>,
    reload_data_receiver: mpsc::Receiver<()>,
    load_balancer_channels: LoadBalancerChannels,
}

impl MasterWorker {
    pub fn new(config: Config) -> Result<Self, Error> {
        let launch_params = &config.launch_params;

        // Initialize models and data.
        // We init everything with empty data.
        // We will read the data from disk when run() is called
        let base_model = BaseModel::empty();
        let data = launch::read::build_transit_data::<Timetable>(
            &base_model,
            &launch_params.default_transfer_duration,
        );
        let real_time_model = RealTimeModel::new();
        let data_and_models = Arc::new(RwLock::new((data, base_model, real_time_model)));

        // LoadBalancer worker
        let (load_balancer, load_balancer_channels) = LoadBalancer::new(
            data_and_models.clone(),
            config.nb_workers,
            &config.requests_socket,
            &config.request_default_params,
        )?;
        let _load_balancer_thread_handle = load_balancer.run_in_a_thread()?;

        // RabbitMq worker
        let (kirin_messages_sender, kirin_messages_receiver) = mpsc::channel(1);
        let (reload_data_sender, reload_data_receiver) = mpsc::channel(1);

        let rabbitmq_worker = RabbitMqWorker::new(
            config.rabbitmq_params.clone(),
            config.launch_params.instance_name.clone(),
            kirin_messages_sender,
            reload_data_sender,
        );
        let _rabbitmq_thread_handle = rabbitmq_worker.run_in_a_thread()?;

        // Master worker
        let result = Self {
            config,
            data_and_models,
            kirin_messages_receiver,
            load_balancer_channels,
            reload_data_receiver,
        };
        Ok(result)
    }

    async fn run(mut self) -> Result<(), Error> {
        self.load_data_from_disk().await?;
        self.main_loop().await
    }

    async fn load_data_from_disk(&mut self) -> Result<(), Error> {
        let launch_params = self.config.launch_params.clone();
        let updater = move |data_and_models: &mut DataAndModels| {
            let new_base_model = launch::read::read_model(&launch_params).map_err(|err| {
                format_err!(
                    "Could not read data from disk at {:?}, because {}",
                    &launch_params.input_data_path,
                    err
                )
            })?;
            info!("Model loaded");
            info!("Starting to build data");
            let new_data = launch::read::build_transit_data::<Timetable>(
                &new_base_model,
                &launch_params.default_transfer_duration,
            );
            info!("Data loaded");
            let new_real_time_model = RealTimeModel::new();
            data_and_models.0 = new_data;
            data_and_models.1 = new_base_model;
            data_and_models.2 = new_real_time_model;
            Ok(())
        };

        self.update_data_and_models(updater).await
    }

    async fn main_loop(mut self) -> Result<(), Error> {
        info!("Starting Master worker");
        loop {
            tokio::select! {
                // receive a kirin message
                has_proto_vec = self.kirin_messages_receiver.recv() => {
                    let vec_protobuf = has_proto_vec
                        .ok_or_else(||
                            format_err!("Channel to receive realtime messages has closed.")
                        )?;

                    info!("Master received real time messages from AmqpWorker");

                    self.handle_realtime_messages(vec_protobuf).await
                        .map_err(|err| format_err!("Error while handling real time messages : {}", err))?;

                }
                // receive reload order
                has_reload_order = self.reload_data_receiver.recv() => {
                    self.load_data_from_disk().await?;
                }
                // receive an error message from load balancer
                _ = self.load_balancer_channels
                    .error_receiver
                    .recv() => {
                        // We don't even need to know if _has_load_balancer_error is None or not
                        // is LoadBalancer send an Error we must shutdown
                        bail!("Load Balancer is broken, exit program safely.");
                    }
            }
        }
    }

    async fn update_data_and_models<Updater>(&mut self, updater: Updater) -> Result<(), Error>
    where
        Updater: FnOnce(&mut DataAndModels) -> Result<(), Error>,
    {
        // Wait for the LoadBalancer to Stop
        debug!("MasterWork waiting for LoadBalancer to Stop");
        self.load_balancer_channels
            .stopped_receiver
            .recv()
            .await
            .ok_or_else(|| format_err!("Channel load_balancer_stopped has closed."))?;

        {
            let mut lock_guard = self.data_and_models.write().map_err(|err| {
                format_err!(
                    "Master worker failed to acquire write lock on data_and_models. {}.",
                    err
                )
            })?;

            updater(&mut *lock_guard)?;
        } // lock_guard is now released

        debug!("Master ask LoadBalancer to Start");
        self.send_order_to_load_balancer(LoadBalancerOrder::Start)
            .await
            .map_err(|err| {
                format_err!(
                    "Master could not send Start order to load balancer. {}",
                    err
                )
            })
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
            .load_balancer_channels
            .order_sender
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

    async fn handle_realtime_messages(
        &mut self,
        messages: Vec<gtfs_realtime::FeedMessage>,
    ) -> Result<(), Error> {
        let updater = |data_and_models: &mut DataAndModels| {
            let data = &mut data_and_models.0;
            let base_model = &data_and_models.1;
            let real_time_model = &mut data_and_models.2;
            for message in messages.into_iter() {
                for feed_entity in message.entity {
                    let disruption_result = handle_kirin_protobuf(&feed_entity);
                    match disruption_result {
                        Err(err) => {
                            error!("Could not handle a kirin message {}", err);
                        }
                        Ok(disruption) => {
                            real_time_model.apply_disruption(&disruption, base_model, data);
                        }
                    }
                }
            }
            Ok(())
        };

        self.update_data_and_models(updater).await
    }
}
