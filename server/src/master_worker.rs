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

use anyhow::{bail, Context, Error};
use launch::loki::{
    models::{base_model::BaseModel, real_time_model::RealTimeModel},
    tracing::{error, info},
    TransitData,
};
use std::sync::{Arc, RwLock};
use tokio::{runtime::Builder, signal, sync::mpsc};

use crate::{
    data_worker::DataWorker, http_worker::HttpWorker, load_balancer::LoadBalancer,
    status_worker::StatusWorker, zmq_worker::ZmqWorker, ServerConfig,
};

pub type DataAndModels = Option<(TransitData, BaseModel, RealTimeModel)>;

pub struct MasterWorker {
    shutdown_receiver: mpsc::Receiver<()>,
}

impl MasterWorker {
    pub fn new(config: ServerConfig) -> Result<Self, Error> {
        // Initialize models and data.
        // We init everything with empty data.
        // DataWorker will take care of reading data from disk
        let base_model = BaseModel::empty();
        let data = TransitData::new(&base_model);
        let real_time_model = RealTimeModel::new();
        let data_and_models = Arc::new(RwLock::new(None));

        let (shutdown_sender, shutdown_receiver) = mpsc::channel(1);

        // Zmq worker
        let (zmq_worker, load_balancer_to_zmq_channels, status_worker_to_zmq_channels) =
            ZmqWorker::new(&config.requests_socket, shutdown_sender.clone());

        let _zmq_handle = zmq_worker.run_in_a_thread()?;

        // http worker
        let (http_worker, http_to_status_channel) =
            HttpWorker::new(config.http.clone(), shutdown_sender.clone());
        let _http_worker_handle = http_worker.run_in_a_thread()?;

        // LoadBalancer
        let (load_balancer, load_balancer_channels) = LoadBalancer::new(
            data_and_models.clone(),
            config.nb_workers,
            &config.default_request_params,
            load_balancer_to_zmq_channels,
            shutdown_sender.clone(),
        )?;
        let _load_balancer_handle = load_balancer.run_in_a_thread()?;

        // Status worker
        let (status_worker, status_update_sender) = StatusWorker::new(
            status_worker_to_zmq_channels,
            http_to_status_channel,
            shutdown_sender.clone(),
            &config,
        );
        let _status_worker_handle = status_worker.run_in_a_thread()?;

        // Data worker
        let data_worker = DataWorker::new(
            config,
            data_and_models,
            load_balancer_channels,
            status_update_sender,
            shutdown_sender,
        )?;
        let _data_worker_handle = data_worker.run_in_a_thread()?;

        // Master worker
        let result = Self { shutdown_receiver };
        info!("Master worker created.");
        Ok(result)
    }

    pub fn run_blocking(self) -> Result<(), Error> {
        // copied from https://tokio.rs/tokio/topics/bridging#sending-messages

        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Failed to build tokio runtime.")?;

        runtime.block_on(self.run())
    }

    pub fn run_in_a_thread(self) -> Result<(), Error> {
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Failed to build tokio runtime.")?;

        let thread_builder = std::thread::Builder::new().name("loki_master_worker".to_string());
        let _handle = thread_builder.spawn(move || runtime.block_on(self.run()))?;
        Ok(())
    }

    async fn run(mut self) -> Result<(), Error> {
        tokio::select! {
            _ = self.shutdown_receiver.recv() => {
                error!("One of the worker sent the shutdown signal.");
                bail!("One of the worker sent the shutdown signal.")

            }
            _ = signal::ctrl_c() => {
                info!("Receive Ctrl+C signal. I'm gonna shut down");
                Ok(())
            }
        }
    }
}
