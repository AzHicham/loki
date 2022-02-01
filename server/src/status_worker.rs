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

use crate::zmq_worker::{RequestMessage, ResponseMessage, StatusWorkerToZmqChannels};

use super::navitia_proto;

use anyhow::{format_err, Context, Error};

use launch::loki::{
    chrono::NaiveDate,
    tracing::{error, log::warn},
    NaiveDateTime,
};

use std::thread;

use tokio::{runtime::Builder, sync::mpsc};

pub struct StatusWorker {
    base_data_info: Option<BaseDataInfo>,
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
}

pub enum StatusUpdate {
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
    ) -> (Self, mpsc::UnboundedSender<StatusUpdate>) {
        let (status_update_sender, status_update_receiver) = mpsc::unbounded_channel();
        let worker = Self {
            base_data_info: None,
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
                    self.handle_status_request(request_message)?;
                }

            }
        }
    }

    fn handle_status_request(&self, request_message: RequestMessage) -> Result<(), Error> {
        // check that the request api is indeed "status"
        // form navitia_proto::Response with Some(status)
        // send it to self.response_sender
        let requested_api = request_message.payload.requested_api();
        if requested_api != navitia_proto::Api::Status {
            warn!("StatusWorker : received a request with api {:?} while I can only handle Status api.", requested_api);
            return Ok(());
        }

        let mut status = navitia_proto::Status::default();
        let date_format = "%Y%m%d";
        let datetime_format = "%Y%m%dT%H%M%S.%f";
        if let Some(info) = &self.base_data_info {
            status.start_production_date = info.start_date.format(date_format).to_string();
            status.end_production_date = info.end_date.format(date_format).to_string();
            status.last_load_at = Some(info.last_load_at.format(datetime_format).to_string());
        }

        status.is_connected_to_rabbitmq = Some(self.is_connected_to_rabbitmq);

        if let Some(date) = &self.last_real_time_update {
            status.last_rt_data_loaded = Some(date.format(datetime_format).to_string());
        }

        let payload = navitia_proto::Response {
            status: Some(status),
            ..Default::default()
        };

        let response = ResponseMessage {
            client_id: request_message.client_id,
            payload,
        };

        self.zmq_channels
            .status_responses_sender
            .send(response)
            .map_err(|err| {
                format_err!(
                    "StatusWorker : channel to send status response is closed : {}",
                    err
                )
            })
    }

    fn handle_status_update(&mut self, status_update: StatusUpdate) {
        match status_update {
            StatusUpdate::BaseDataLoad(base_data_info) => {
                self.base_data_info = Some(base_data_info);
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
            }
            StatusUpdate::RealTimeUpdate(datetime) => {
                self.last_real_time_update = Some(datetime);
            }
        }
    }
}
