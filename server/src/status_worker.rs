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

use crate::zmq_worker::{RequestMessage, ResponseMessage, StatusWorkerToZmqChannels};

use super::navitia_proto;

use anyhow::{format_err, Context, Error};

use launch::{
    loki::{
        chrono::NaiveDate,
        chrono_tz,
        tracing::{debug, error, info, log::warn},
        NaiveDateTime,
    },
    timer,
};

use std::{thread, time::SystemTime};

use crate::ServerConfig;
use tokio::{runtime::Builder, sync::mpsc};

pub const DATE_FORMAT: &str = "%Y%m%d";
pub const DATETIME_FORMAT: &str = "%Y%m%dT%H%M%S.%f";
pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct StatusWorker {
    base_data_info: Option<BaseDataInfo>,
    config_info: ConfigInfo,
    last_load_succeeded: bool, // last reload was successful
    is_realtime_loaded: bool,  // is_realtime_loaded for the last reload
    is_connected_to_rabbitmq: bool,
    last_kirin_reload: Option<NaiveDateTime>,
    last_chaos_reload: Option<NaiveDateTime>,
    last_real_time_update: Option<NaiveDateTime>,

    zmq_channels: StatusWorkerToZmqChannels,

    status_update_receiver: mpsc::UnboundedReceiver<StatusUpdate>,

    shutdown_sender: mpsc::Sender<()>,
}

pub struct BaseDataInfo {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub last_load_at: NaiveDateTime,
    pub dataset_created_at: Option<NaiveDateTime>,
    pub timezone: chrono_tz::Tz,
    pub contributors: Vec<String>,
    pub publisher_name: Option<String>,
}

pub struct ConfigInfo {
    pub pkg_version: String,
    pub real_time_contributors: Vec<String>,
    pub nb_workers: u16,
}

pub enum StatusUpdate {
    BaseDataLoadFailed,
    BaseDataLoad(BaseDataInfo),
    RabbitMqConnected,
    RabbitMqDisconnected,
    ChaosReload(NaiveDateTime),
    KirinReload(NaiveDateTime),
    RealTimeUpdate(NaiveDateTime),
}

impl StatusWorker {
    pub fn new(
        zmq_channels: StatusWorkerToZmqChannels,
        shutdown_sender: mpsc::Sender<()>,
        server_config: &ServerConfig,
    ) -> (Self, mpsc::UnboundedSender<StatusUpdate>) {
        let (status_update_sender, status_update_receiver) = mpsc::unbounded_channel();
        let worker = Self {
            base_data_info: None,
            config_info: ConfigInfo {
                pkg_version: PKG_VERSION.to_string(),
                real_time_contributors: server_config.rabbitmq.real_time_topics.clone(),
                nb_workers: server_config.nb_workers,
            },
            last_load_succeeded: false,
            is_realtime_loaded: false,
            is_connected_to_rabbitmq: false,
            last_chaos_reload: None,
            last_kirin_reload: None,
            last_real_time_update: None,
            zmq_channels,
            status_update_receiver,
            shutdown_sender,
        };

        (worker, status_update_sender)
    }

    // run in a spawned thread
    pub fn run_in_a_thread(self) -> Result<std::thread::JoinHandle<()>, Error> {
        // copied from https://tokio.rs/tokio/topics/bridging#sending-messages

        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Failed to build tokio runtime.")?;

        let thread_builder = thread::Builder::new().name("loki_status_worker".to_string());
        let handle = thread_builder.spawn(move || runtime.block_on(self.run()))?;
        Ok(handle)
    }

    async fn run(mut self) {
        let err = self.run_loop().await;
        error!("StatusWorker failed : {:?}", err);
        let _ = self.shutdown_sender.send(()).await;
    }

    async fn run_loop(&mut self) -> Result<(), Error> {
        loop {
            tokio::select! {
                // we biase the select toward handling status updates
                // before answering to requests.
                // In this way, we will always send the latest info
                biased;

                has_update = self.status_update_receiver.recv() => {
                    let status_update = has_update.ok_or_else(||
                        format_err!("StatusWorker : channel to receive status updates is closed.")
                    )?;
                    self.handle_status_update(status_update);
                }
                has_request = self.zmq_channels.status_requests_receiver.recv() => {
                    let request_message = has_request.ok_or_else(||
                        format_err!("StatusWorker : channel to receive status requests is closed.")
                    )?;
                    self.handle_request(request_message)?;
                }

            }
        }
    }

    fn handle_request(&self, request_message: RequestMessage) -> Result<(), Error> {
        let handle_request_start_time = SystemTime::now();
        let requested_api = request_message.payload.requested_api();
        let request_id = request_message.payload.request_id.unwrap_or_default();
        debug!(
            "Status worker received request on api {:?} with id '{}'",
            requested_api, request_id
        );
        let response_payload = match requested_api {
            navitia_proto::Api::Status => navitia_proto::Response {
                status: Some(self.status_response()),
                metadatas: Some(self.metadatas_response()),
                ..Default::default()
            },
            navitia_proto::Api::Metadatas => navitia_proto::Response {
                metadatas: Some(self.metadatas_response()),
                ..Default::default()
            },
            _ => {
                error!("StatusWorker : received a request with api {:?} while I can only handle Status or Metadatas api.", requested_api);
                return Ok(());
            }
        };

        let response = ResponseMessage {
            client_id: request_message.client_id,
            payload: response_payload,
        };

        self.zmq_channels
            .status_responses_sender
            .send(response)
            .map_err(|err| {
                format_err!(
                    "StatusWorker : channel to send status response is closed : {}",
                    err
                )
            })?;

        let duration = timer::duration_since(handle_request_start_time);
        info!(
            "Status worker responded in {} ms to request on api {:?} with id '{}'",
            duration, requested_api, request_id
        );

        Ok(())
    }

    fn status_response(&self) -> navitia_proto::Status {
        let mut status = navitia_proto::Status::default();
        let date_format = DATE_FORMAT;
        let datetime_format = DATETIME_FORMAT;
        if let Some(info) = &self.base_data_info {
            status.start_production_date = info.start_date.format(date_format).to_string();
            status.end_production_date = info.end_date.format(date_format).to_string();
            status.last_load_at = Some(info.last_load_at.format(datetime_format).to_string());
            status.dataset_created_at = info
                .dataset_created_at
                .map(|dt| dt.format(datetime_format).to_string());
            status.is_realtime_loaded = Some(self.is_realtime_loaded);
            status.loaded = Some(true);
            status.status = Some("running".to_string());
        } else {
            status.loaded = Some(false);
            status.status = Some("no_data".to_string());
        }
        status.last_load_status = Some(self.last_load_succeeded);

        status.is_connected_to_rabbitmq = Some(self.is_connected_to_rabbitmq);

        status.navitia_version = Some(self.config_info.pkg_version.clone());
        status.nb_threads = Some(i32::from(self.config_info.nb_workers));
        for rt_contributors in &self.config_info.real_time_contributors {
            status.rt_contributors.push(rt_contributors.clone());
        }

        if let Some(date) = &self.last_real_time_update {
            status.last_rt_data_loaded = Some(date.format(datetime_format).to_string());
        }

        status
    }

    fn metadatas_response(&self) -> navitia_proto::Metadatas {
        let date_format = DATE_FORMAT;
        let datetime_format = DATETIME_FORMAT;

        match &self.base_data_info {
            None => navitia_proto::Metadatas {
                status: "no_data".to_string(),
                ..Default::default()
            },
            Some(base_data_info) => navitia_proto::Metadatas {
                start_production_date: base_data_info.start_date.format(date_format).to_string(),
                end_production_date: base_data_info.end_date.format(date_format).to_string(),
                dataset_created_at: base_data_info
                    .dataset_created_at
                    .map(|dt| dt.format(datetime_format).to_string()),
                last_load_at: u64::try_from(base_data_info.last_load_at.timestamp()).ok(),
                timezone: Some(base_data_info.timezone.name().to_string()),
                contributors: base_data_info.contributors.clone(),
                status: "running".to_string(),
                name: base_data_info.publisher_name.clone(),
                ..Default::default()
            },
        }
    }

    fn handle_status_update(&mut self, status_update: StatusUpdate) {
        match status_update {
            StatusUpdate::BaseDataLoadFailed => {
                self.last_load_succeeded = false;
            }
            StatusUpdate::BaseDataLoad(base_data_info) => {
                self.base_data_info = Some(base_data_info);
                self.last_load_succeeded = true;
            }
            StatusUpdate::RabbitMqConnected => {
                if self.is_connected_to_rabbitmq {
                    warn!("StatusWorker : received RabbitMqConnected update while I should already be connected to rabbitmq");
                }
                self.is_connected_to_rabbitmq = true;
            }
            StatusUpdate::RabbitMqDisconnected => {
                if !self.is_connected_to_rabbitmq {
                    warn!("StatusWorker : received RabbitMqDisconnected update while I should already be disconnected to rabbitmq");
                }
                self.is_connected_to_rabbitmq = false;
            }
            StatusUpdate::ChaosReload(datetime) => {
                self.last_chaos_reload = Some(datetime);
                self.last_real_time_update = Some(datetime);
            }
            StatusUpdate::KirinReload(datetime) => {
                self.last_kirin_reload = Some(datetime);
                self.last_real_time_update = Some(datetime);
                self.is_realtime_loaded = true;
            }
            StatusUpdate::RealTimeUpdate(datetime) => {
                self.last_real_time_update = Some(datetime);
            }
        }
    }
}
