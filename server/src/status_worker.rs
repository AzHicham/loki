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

use crate::{
    http_worker::HttpToStatusChannel,
    zmq_worker::{RequestMessage, ResponseMessage, StatusWorkerToZmqChannels},
};
use serde::{Deserialize, Serialize};

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
pub const LOKI_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct StatusWorker {
    status: Status,

    zmq_channels: StatusWorkerToZmqChannels,

    http_channel: HttpToStatusChannel,

    status_update_receiver: mpsc::UnboundedReceiver<StatusUpdate>,

    shutdown_sender: mpsc::Sender<()>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Status {
    pub base_data_info: Option<BaseDataInfo>,
    pub config_info: ConfigInfo,
    pub last_load_succeeded: bool, // last reload was successful
    pub is_realtime_loaded: bool,  // is_realtime_loaded for the last reload
    pub is_connected_to_rabbitmq: bool,
    pub realtime_queue_created: bool,
    pub reload_queue_created: bool,
    pub last_kirin_reload: Option<NaiveDateTime>,
    pub last_chaos_reload: Option<NaiveDateTime>,
    pub last_real_time_update: Option<NaiveDateTime>,
    pub loki_version: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BaseDataInfo {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub last_load_at: NaiveDateTime,
    pub dataset_created_at: Option<NaiveDateTime>,
    pub timezone: chrono_tz::Tz,
    pub contributors: Vec<String>,
    pub publisher_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigInfo {
    pub instance_name: String,
    pub real_time_contributors: Vec<String>,
    pub nb_workers: u16,
}

#[derive(Debug)]
pub enum StatusUpdate {
    BaseDataLoadFailed,
    BaseDataLoad(BaseDataInfo),
    RabbitMqConnected,
    RealTimeQueueCreated,
    ReloadQueueCreated,
    RabbitMqDisconnected,
    ChaosReload(NaiveDateTime),
    KirinReload(NaiveDateTime),
    RealTimeUpdate(NaiveDateTime),
}

impl StatusWorker {
    pub fn new(
        zmq_channels: StatusWorkerToZmqChannels,
        http_channel: HttpToStatusChannel,
        shutdown_sender: mpsc::Sender<()>,
        server_config: &ServerConfig,
    ) -> (Self, mpsc::UnboundedSender<StatusUpdate>) {
        let (status_update_sender, status_update_receiver) = mpsc::unbounded_channel();
        let worker = Self {
            status: Status {
                base_data_info: None,
                config_info: ConfigInfo {
                    instance_name: server_config.instance_name.clone(),
                    real_time_contributors: server_config.rabbitmq.real_time_topics.clone(),
                    nb_workers: server_config.nb_workers,
                },
                last_load_succeeded: false,
                is_realtime_loaded: false,
                is_connected_to_rabbitmq: false,
                realtime_queue_created: false,
                reload_queue_created: false,
                last_chaos_reload: None,
                last_kirin_reload: None,
                last_real_time_update: None,
                loki_version: LOKI_VERSION.to_string(),
            },
            zmq_channels,
            http_channel,
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
                has_zmq_request = self.zmq_channels.status_requests_receiver.recv() => {
                    let request_message = has_zmq_request.ok_or_else(||
                        format_err!("StatusWorker : channel to receive zmq status requests is closed.")
                    )?;
                    self.handle_zmq_request(request_message)?;
                }
                has_http_request = self.http_channel.status_request_receiver.recv() => {
                    let response_chan = has_http_request.ok_or_else(||
                        format_err!("StatusWorker : channel to receive http status requests is closed.")
                    )?;
                    let send_result = response_chan.send(self.status.clone());
                    if send_result.is_err() {
                        error!("Error while sending status response to http worker : the receiver is closed.");
                    }
                }

            }
        }
    }

    fn handle_zmq_request(&self, request_message: RequestMessage) -> Result<(), Error> {
        let handle_request_start_time = SystemTime::now();
        let requested_api = request_message.payload.requested_api();
        let request_id = request_message.payload.request_id.unwrap_or_default();
        debug!(
            "Status worker received request on api {:?} with id '{}'",
            requested_api, request_id
        );
        let response_payload = match requested_api {
            navitia_proto::Api::Status => navitia_proto::Response {
                status: Some(self.status_proto_response()),
                metadatas: Some(self.metadatas_proto_response()),
                ..Default::default()
            },
            navitia_proto::Api::Metadatas => navitia_proto::Response {
                metadatas: Some(self.metadatas_proto_response()),
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

    fn status_proto_response(&self) -> navitia_proto::Status {
        let mut proto_status = navitia_proto::Status::default();
        let date_format = DATE_FORMAT;
        let datetime_format = DATETIME_FORMAT;
        if let Some(info) = &self.status.base_data_info {
            proto_status.start_production_date = info.start_date.format(date_format).to_string();
            proto_status.end_production_date = info.end_date.format(date_format).to_string();
            proto_status.last_load_at = Some(info.last_load_at.format(datetime_format).to_string());
            proto_status.dataset_created_at = info
                .dataset_created_at
                .map(|dt| dt.format(datetime_format).to_string());
            proto_status.is_realtime_loaded = Some(self.status.is_realtime_loaded);
            proto_status.loaded = Some(true);
            proto_status.status = Some("running".to_string());
        } else {
            proto_status.loaded = Some(false);
            proto_status.status = Some("no_data".to_string());
        }
        proto_status.last_load_status = Some(self.status.last_load_succeeded);

        proto_status.is_connected_to_rabbitmq = Some(self.status.is_connected_to_rabbitmq);

        proto_status.navitia_version = Some(self.status.loki_version.clone());
        proto_status.nb_threads = Some(i32::from(self.status.config_info.nb_workers));
        for rt_contributors in &self.status.config_info.real_time_contributors {
            proto_status.rt_contributors.push(rt_contributors.clone());
        }

        if let Some(date) = &self.status.last_real_time_update {
            proto_status.last_rt_data_loaded = Some(date.format(datetime_format).to_string());
        }

        proto_status
    }

    fn metadatas_proto_response(&self) -> navitia_proto::Metadatas {
        let date_format = DATE_FORMAT;
        let datetime_format = DATETIME_FORMAT;

        match &self.status.base_data_info {
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
                self.status.last_load_succeeded = false;
            }
            StatusUpdate::BaseDataLoad(base_data_info) => {
                self.status.base_data_info = Some(base_data_info);
                self.status.last_load_succeeded = true;
            }
            StatusUpdate::RabbitMqConnected => {
                if self.status.is_connected_to_rabbitmq {
                    warn!("StatusWorker : received RabbitMqConnected update while I should already be connected to rabbitmq");
                }
                self.status.is_connected_to_rabbitmq = true;
            }
            StatusUpdate::RealTimeQueueCreated => {
                if self.status.realtime_queue_created {
                    warn!("StatusWorker : received RealTimeQueueCreated update while it should already be created");
                }
                self.status.realtime_queue_created = true;
            }
            StatusUpdate::ReloadQueueCreated => {
                if self.status.reload_queue_created {
                    warn!("StatusWorker : received ReloadQueueCreated update while it should already be created");
                }
                self.status.reload_queue_created = true;
            }
            StatusUpdate::RabbitMqDisconnected => {
                if !self.status.is_connected_to_rabbitmq {
                    warn!("StatusWorker : received RabbitMqDisconnected update while I should already be disconnected to rabbitmq");
                }
                self.status.is_connected_to_rabbitmq = false;
            }
            StatusUpdate::ChaosReload(datetime) => {
                self.status.last_chaos_reload = Some(datetime);
                self.status.last_real_time_update = Some(datetime);
            }
            StatusUpdate::KirinReload(datetime) => {
                self.status.last_kirin_reload = Some(datetime);
                self.status.last_real_time_update = Some(datetime);
                self.status.is_realtime_loaded = true;
            }
            StatusUpdate::RealTimeUpdate(datetime) => {
                self.status.last_real_time_update = Some(datetime);
            }
        }
    }
}
