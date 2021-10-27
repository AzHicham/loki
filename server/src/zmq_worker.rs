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

use std::{iter::FromIterator, thread};

use failure::{format_err, Error};

use launch::loki::tracing::{debug, error, info};
use prost::Message;
use tmq;

use tokio::{runtime::Builder, sync::mpsc};

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
    // to send request to master_worker
    requests_sender: mpsc::UnboundedSender<RequestMessage>,
    // to receive responses from master_worker
    responses_receiver: mpsc::UnboundedReceiver<ResponseMessage>,
    // to send response to itself, useful when an incoming zmq message is not a valid navitia_proto::Request
    responses_sender: mpsc::UnboundedSender<ResponseMessage>,
}

pub struct ZmqWorkerChannels {
    pub requests_receiver: mpsc::UnboundedReceiver<RequestMessage>,
    pub responses_sender: mpsc::UnboundedSender<ResponseMessage>,
}

impl ZmqWorker {
    pub fn new(endpoint: &str) -> (Self, ZmqWorkerChannels) {
        let (requests_sender, requests_receiver) = mpsc::unbounded_channel();
        let (responses_sender, responses_receiver) = mpsc::unbounded_channel();

        let actor = Self {
            endpoint: endpoint.to_string(),
            requests_sender,
            responses_receiver,
            responses_sender: responses_sender.clone(),
        };

        let handle = ZmqWorkerChannels {
            requests_receiver,
            responses_sender,
        };

        (actor, handle)
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
        let context = tmq::Context::new();
        let zmq_socket_result = tmq::router(&context).bind(&self.endpoint);
        info!("Zmq worker bound to endpoint {}", self.endpoint);

        match zmq_socket_result {
            Ok(zmq_socket) => self.run_loop(zmq_socket).await,
            Err(err) => {
                error!(
                    "Could not connect zmq to endpoint {}. Error : {}",
                    self.endpoint, err
                );
            }
        }
    }

    async fn run_loop(&mut self, mut zmq_socket: tmq::router::Router) {
        use futures::StreamExt;
        loop {
            debug!("Zmq worker is waiting.");
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
                    if let Some(response) = has_response {
                        debug!("Received a response");
                        send_response_to_zmq(& mut zmq_socket, response).await
                    }
                    else {
                        error!("The response channel is closed. I'll stop.");
                        break;
                    }
                }
                // receive requests from the zmq socket, and send them to the main thread for dispatch to workers
                has_zmq_message = zmq_socket.next() => {
                    if let Some(zmq_message_result) = has_zmq_message {
                        match zmq_message_result {
                            Ok(zmq_message) => {
                                debug!("Received a zmq request");
                                let result = handle_incoming_request(
                                    zmq_message,
                                    &mut self.requests_sender,
                                    &mut self.responses_sender,
                                );
                                if let Err(()) = result {
                                    error!("The response channel is closed. I'll stop.");
                                    break;
                                }

                            },
                            Err(err) => {
                                error!("Error while reading zmq socket : {}", err);
                            }
                        }
                    }
                    else {
                        error!("The zmq socket has been closed. I'll stop.");
                        break;
                    }

                }

            }
        }
    }
}

async fn send_response_to_zmq(zmq_socket: &mut tmq::router::Router, response: ResponseMessage) {
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

    let multipart_msg = tmq::Multipart::from_iter(iter);
    use futures::SinkExt;
    let send_result = zmq_socket.send(multipart_msg).await;
    if let Err(err) = send_result {
        error!("Error while sending response to zmq socket : {}", err);
    }
}

fn handle_incoming_request(
    mut zmq_message: tmq::Multipart,
    requests_sender: &mut mpsc::UnboundedSender<RequestMessage>,
    responses_sender: &mut mpsc::UnboundedSender<ResponseMessage>,
) -> Result<(), ()> {
    // The Router socket should always provides 3 parts messages with an empty second part.
    // see https://zguide.zeromq.org/docs/chapter3/#The-Extended-Reply-Envelope
    let nb_parts = zmq_message.len();
    if nb_parts != 3 {
        error!("Received a zmq message with {} parts. I only know how to handle messages with 3 parts. I'll ignore it", nb_parts);
        return Ok(());
    }
    // the 3 unwraps are safe since we just checked that the message has lenght 3
    let client_id_message = zmq_message.pop_front().unwrap();
    let empty_message = zmq_message.pop_front().unwrap();
    let payload_message = zmq_message.pop_front().unwrap();

    if empty_message.len() > 0 {
        error!("Received a zmq message with a non empty second part. Since this is invalid, I'll skip this message");
        return Ok(());
    }

    use std::ops::Deref;
    let proto_request_result = navitia_proto::Request::decode(payload_message.deref());
    // TODO : if deadline is expired, do not send to master
    match proto_request_result {
        Ok(proto_request) => {
            let request_message = RequestMessage {
                client_id: client_id_message,
                payload: proto_request,
            };
            let send_result = requests_sender.send(request_message);
            if let Err(err) = send_result {
                error!("Error while forwarding request to master_worker : {}", err);
                // TODO : what to do here ?
                // if an error occurs while sending
                // it means that the receiver of the channel is closed
                // so we won't be able to send messages anywhere
                // We could panic, or gracefully shutdown this thread
                // For now, we keep going, as we may still receive responses and send them to zmq
            }
        }
        Err(err) => {
            error!(
                "Could not decode zmq message into protobuf. Error : {}",
                err
            );
            // let's send back a response to our zmq client that we received an invalid protobuf

            let response_proto = make_error_response(format_err!(
                "Could not decode zmq message into protobuf. Error : {}",
                err
            ));
            let response_message = ResponseMessage {
                client_id: client_id_message,
                payload: response_proto,
            };

            let send_result = responses_sender.send(response_message);

            if let Err(err) = send_result {
                error!("Error while sending error response to zmq_worker : {}", err);
                // if an error occurs while sending
                // it means that the receiver of the channel is closed
                // so we won't be able to receive response messages anywhere
                // The only sensible thing is to shutdown.
                // TODO : can we do something else ?
                return Err(());
            }
        }
    }
    Ok(())
}

fn make_error_response(error: Error) -> navitia_proto::Response {
    let mut proto_response = navitia_proto::Response::default();
    proto_response.set_response_type(navitia_proto::ResponseType::NoSolution);
    let mut proto_error = navitia_proto::Error::default();
    proto_error.set_id(navitia_proto::error::ErrorId::InternalError);
    proto_error.message = Some(format!("{}", error));
    proto_response.error = Some(proto_error);
    proto_response
}
