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

use std::thread;

use anyhow::{format_err, Context, Error};

use launch::loki::{
    chrono::Utc,
    tracing::{error, info, log::trace, warn},
    NaiveDateTime,
};
use prost::Message;
use tmq;

use tokio::{runtime::Builder, sync::mpsc};

use futures::SinkExt;
use std::ops::Deref;

use crate::navitia_proto;

#[derive(Debug)]
pub struct RequestMessage {
    pub payload: navitia_proto::Request, // the actual data received from zmq
    pub client_id: tmq::Message,         // the identifer of the client in the zmq socket
}

#[derive(Debug)]
pub struct ResponseMessage {
    pub payload: navitia_proto::Response,
    pub client_id: tmq::Message, // the identifer of the client in the zmq socket
}

pub struct ZmqWorker {
    endpoint: String,
    // to send requests to load_balancer
    requests_sender: mpsc::UnboundedSender<RequestMessage>,
    // to receive responses from load_balancer
    responses_receiver: mpsc::UnboundedReceiver<ResponseMessage>,

    status_requests_sender: mpsc::UnboundedSender<RequestMessage>,
    status_responses_receiver: mpsc::UnboundedReceiver<ResponseMessage>,

    // to send shutdown signal to Master when an error occurs insider ZmqWorker
    shutdown_sender: mpsc::Sender<()>,
}

pub struct LoadBalancerToZmqChannels {
    pub requests_receiver: mpsc::UnboundedReceiver<RequestMessage>,
    pub responses_sender: mpsc::UnboundedSender<ResponseMessage>,
}

pub struct StatusWorkerToZmqChannels {
    pub status_requests_receiver: mpsc::UnboundedReceiver<RequestMessage>,
    pub status_responses_sender: mpsc::UnboundedSender<ResponseMessage>,
}

impl ZmqWorker {
    pub fn new(
        endpoint: &str,
        shutdown_sender: mpsc::Sender<()>,
    ) -> (Self, LoadBalancerToZmqChannels, StatusWorkerToZmqChannels) {
        let (requests_sender, requests_receiver) = mpsc::unbounded_channel();
        let (responses_sender, responses_receiver) = mpsc::unbounded_channel();
        let (status_requests_sender, status_requests_receiver) = mpsc::unbounded_channel();
        let (status_responses_sender, status_responses_receiver) = mpsc::unbounded_channel();

        let worker = Self {
            endpoint: endpoint.to_string(),
            requests_sender,
            responses_receiver,
            shutdown_sender,
            status_requests_sender,
            status_responses_receiver,
        };

        let load_balancer_channels = LoadBalancerToZmqChannels {
            requests_receiver,
            responses_sender,
        };

        let status_channels = StatusWorkerToZmqChannels {
            status_requests_receiver,
            status_responses_sender,
        };

        (worker, load_balancer_channels, status_channels)
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

    // run in a spawned thread
    pub fn run_in_a_thread(self) -> Result<std::thread::JoinHandle<()>, Error> {
        // copied from https://tokio.rs/tokio/topics/bridging#sending-messages

        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Failed to build tokio runtime.")?;

        let thread_builder = thread::Builder::new().name("loki_zmq_worker".to_string());
        let handle = thread_builder.spawn(move || runtime.block_on(self.run()))?;
        Ok(handle)
    }

    async fn run(mut self) {
        let context = tmq::Context::new();
        let zmq_socket_result = tmq::router(&context).bind(&self.endpoint);

        match zmq_socket_result {
            Ok(zmq_socket) => {
                info!("Zmq worker bound to endpoint {}", self.endpoint);
                let run_err = self.run_loop(zmq_socket).await;
                error!("ZmqWorker stopped : {:?}", run_err);
            }
            Err(err) => {
                error!(
                    "Could not connect zmq to endpoint {}. {:?}",
                    self.endpoint, err
                );
            }
        }

        // send shutdown signal
        let _ = self.shutdown_sender.send(()).await;
    }

    async fn run_loop(&mut self, mut zmq_socket: tmq::router::Router) -> Result<(), Error> {
        use futures::StreamExt;
        loop {
            trace!("Zmq worker is waiting.");
            tokio::select! {
                // this indicates to tokio to poll the futures in the order they appears below
                // see https://docs.rs/tokio/1.12.0/tokio/macro.select.html#fairness
                // here use this give priority to sending responses
                // thus, receiving new requests has a lower priority
                //
                biased;

                // receive responses from worker threads
                // and send them back to the zmq socket
                has_response = self.responses_receiver.recv() => {
                    let response = has_response.ok_or_else(||
                        format_err!("ZmqWorker : channel to receive responses is closed.")
                    )?;
                    trace!("ZmqWorker received a response.");
                    send_response_to_zmq(& mut zmq_socket, response).await?;

                }
                has_status_response = self.status_responses_receiver.recv() => {
                    let response = has_status_response.ok_or_else(||
                        format_err!("ZmqWorker : channel to receive status responses is closed.")
                    )?;
                    trace!("ZmqWorker received a status response.");
                    send_response_to_zmq(& mut zmq_socket, response).await?;
                }
                // receive requests from the zmq socket, and send them to the main thread for dispatch to workers
                has_zmq_message = zmq_socket.next() => {
                    let zmq_message_result = has_zmq_message
                        .ok_or_else(|| format_err!("ZmqWorker : the zmq socket is closed.")
                    )?;

                    match zmq_message_result {
                        Ok(zmq_message) => {
                            trace!("Received a zmq request");
                            handle_incoming_request(
                                &mut zmq_socket,
                                zmq_message,
                                &mut self.requests_sender,
                                &mut self.status_requests_sender,
                            ).await?;

                        },
                        Err(err) => {
                            error!("Error while reading zmq socket. {:?}", err);
                        }
                    }

                }

            }
        }
    }
}

async fn send_response_to_zmq(
    zmq_socket: &mut tmq::router::Router,
    response: ResponseMessage,
) -> Result<(), Error> {
    let response_bytes = response.payload.encode_to_vec();
    let payload_message = tmq::Message::from(response_bytes);

    // The Router socket requires sending 3 parts messages as responses, where :
    //  - the first part is an identifier or the client
    //  - the second part is empty
    //  - the third part is the actual message
    // see https://zguide.zeromq.org/docs/chapter3/#The-Extended-Reply-Envelope
    let client_id_message = response.client_id;
    let empty_message = tmq::Message::new();
    let iter = std::iter::once(client_id_message)
        .chain(std::iter::once(empty_message))
        .chain(std::iter::once(payload_message));

    let multipart_msg: tmq::Multipart = iter.collect();

    zmq_socket
        .send(multipart_msg)
        .await
        .context("ZmqWorker, error while sending response to zmq socket.")
}

async fn handle_incoming_request(
    zmq_socket: &mut tmq::router::Router,
    mut zmq_message: tmq::Multipart,
    requests_sender: &mut mpsc::UnboundedSender<RequestMessage>,
    status_request_sender: &mut mpsc::UnboundedSender<RequestMessage>,
) -> Result<(), Error> {
    // The Router socket should always provides 3 parts messages with an empty second part.
    // see https://zguide.zeromq.org/docs/chapter3/#The-Extended-Reply-Envelope
    let nb_parts = zmq_message.len();
    if nb_parts != 3 {
        error!("ZmqWorker received a zmq message with {} parts. I only know how to handle messages with 3 parts. I'll ignore it", nb_parts);
        return Ok(());
    }
    // the 3 unwraps are safe since we just checked that the message has lenght 3
    let client_id_message = zmq_message.pop_front().unwrap();
    let empty_message = zmq_message.pop_front().unwrap();
    let payload_message = zmq_message.pop_front().unwrap();

    if empty_message.len() > 0 {
        error!("ZmqWorker received a zmq message with a non empty second part. Since this is invalid, I'll skip this message");
        return Ok(());
    }

    let proto_request_result = navitia_proto::Request::decode(payload_message.deref());
    // TODO ? : if deadline is expired, do not forward request
    match proto_request_result {
        Ok(proto_request) => {
            if is_deadline_expired(&proto_request) {
                info!("ZmqWorker received an expired request. I'll ignore it.");
                return Ok(());
            }

            let requested_api = proto_request.requested_api();

            let request_message = RequestMessage {
                client_id: client_id_message,
                payload: proto_request,
            };
            use navitia_proto::Api;

            match requested_api {
                Api::Status | Api::Metadatas => status_request_sender
                    .send(request_message)
                    .context("ZmqWorker error while forwarding request to status worker."),
                Api::PtPlanner | Api::PlacesNearby | Api::NextDepartures | Api::NextArrivals => {
                    requests_sender
                        .send(request_message)
                        .context("ZmqWorker error while forwarding request to load balancer.")
                }
                _ => {
                    error!(
                        "ZmqWorker received a request with api {:?} while I can only handle \
                    Status/Metadatas/PtPlanner/PlacesNearby/NextDepartures/NextArrivals api.",
                        requested_api
                    );
                    Ok(())
                }
            }
        }
        Err(err) => {
            let err_str = format!("Could not decode zmq message into protobuf. {:?}", err);
            error!("{}", err_str);

            // let's send back a response to our zmq client that we received an invalid protobuf
            let response_proto = make_error_response(&format_err!("{}", err_str));
            let response_message = ResponseMessage {
                client_id: client_id_message,
                payload: response_proto,
            };
            send_response_to_zmq(zmq_socket, response_message).await
        }
    }
}

fn make_error_response(error: &Error) -> navitia_proto::Response {
    let mut proto_response = navitia_proto::Response::default();
    proto_response.set_response_type(navitia_proto::ResponseType::NoSolution);
    let mut proto_error = navitia_proto::Error::default();
    proto_error.set_id(navitia_proto::error::ErrorId::InternalError);
    proto_error.message = Some(format!("{}", error));
    proto_response.error = Some(proto_error);
    proto_response
}

fn is_deadline_expired(proto_request: &navitia_proto::Request) -> bool {
    if let Some(deadline_str) = &proto_request.deadline {
        let datetime_result = NaiveDateTime::parse_from_str(deadline_str, "%Y%m%dT%H%M%S,%f");
        match datetime_result {
            Ok(datetime) => {
                let now = Utc::now().naive_utc();
                now > datetime
            }
            Err(err) => {
                warn!(
                    "Could not parse deadline string {}. Error : {}",
                    deadline_str, err
                );
                // deadline could not be parsed, so let's say that the deadline is not expired
                false
            }
        }
    } else {
        // there is no deadline, so it is not expired...
        false
    }
}
