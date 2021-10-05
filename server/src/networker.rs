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

use std::iter::FromIterator;

use failure::{format_err, Error};

use launch::loki::tracing::{error, info, warn};
use tmq;

use tokio::runtime::Builder;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct RequestBlob {
    pub payload: Vec<u8>,   // the actual data received from zmq
    pub client_id: Vec<u8>, // the identifer of the client in the zmq socket
}

#[derive(Debug)]
pub struct ResponseBlob {
    pub payload: Vec<u8>,
    pub client_id: Vec<u8>, // the identifer of the client in the zmq socket
}

pub struct Networker {
    endpoint: String,
    requests_channel: mpsc::Sender<RequestBlob>,
    responses_channel: mpsc::Receiver<ResponseBlob>,
}

pub struct NetworkerHandle {
    pub requests_channel: mpsc::Receiver<RequestBlob>,
    pub responses_channel: mpsc::Sender<ResponseBlob>,
}

impl Networker {
    pub fn new(endpoint: String) -> Result<NetworkerHandle, Error> {
        let context = tmq::Context::new();

        let (requests_channel_sender, requests_channel_receiver) = mpsc::channel(1);
        let (reponses_channel_sender, reponses_channel_receiver) = mpsc::channel(1);

        let actor = Self {
            endpoint,
            requests_channel: requests_channel_sender,
            responses_channel: reponses_channel_receiver,
        };

        let handle = NetworkerHandle {
            requests_channel: requests_channel_receiver,
            responses_channel: reponses_channel_sender,
        };

        // copied from https://tokio.rs/tokio/topics/bridging#sending-messages

        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| format_err!("Failed to build tokio runtime. Error : {}", err))?;

        std::thread::spawn(move || runtime.block_on({ actor.run(&context) }));

        Ok(handle)
    }

    async fn run(mut self, context: &tmq::Context) {
        let zmq_socket_result = tmq::router(&context).bind(&self.endpoint);

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
        use futures::{SinkExt, StreamExt};
        loop {
            tokio::select! {
                // receive responses from worker threads
                // and send them back to the zmq socket
                has_response = self.responses_channel.recv() => {
                    if let Some(response) = has_response {
                        info!("Received a response");
                        let zmq_message = response.to_zmq_message();
                        let send_result = zmq_socket.send(zmq_message).await;
                        if let Err(err) = send_result {
                            warn!("Error while sending response to zmq socket : {}", err);
                        }
                    }
                    else {
                        warn!("The response channel has been closed. I'll stop.");
                        break;
                    }
                }
                // receive requests from the zmq socket, and send them to the main thread for dispatch to workers
                has_zmq_message = zmq_socket.next() => {
                    if let Some(zmq_message_result) = has_zmq_message {
                        match zmq_message_result {
                            Ok(zmq_message) => {
                                info!("Received a zmq request");
                                let request_blob_result = RequestBlob::from_zmq(zmq_message);
                                if let Ok(request_blob) = request_blob_result {
                                    let send_result = self.requests_channel.send(request_blob).await;
                                    if let Err(err) = send_result {
                                        warn!("Error while sending request to the main thread : {}", err);
                                    }
                                }

                            },
                            Err(err) => {
                                warn!("Error while reading zmq socket : {}", err);
                            }
                        }
                    }
                    else {
                        warn!("The zmq socket has been closed. I'll stop.");
                        break;
                    }

                }

            }
        }
    }
}

impl ResponseBlob {
    pub fn to_zmq_message(self) -> tmq::Multipart {
        let client_id_message = tmq::Message::from(self.client_id);
        let payload_message = tmq::Message::from(self.payload);
        let iter = std::iter::once(client_id_message).chain(std::iter::once(payload_message));
        tmq::Multipart::from_iter(iter)
    }
}

impl RequestBlob {
    pub fn from_zmq(multipart_message: tmq::Multipart) -> Result<Self, ()> {
        let nb_parts = multipart_message.len();
        if nb_parts != 3 {
            warn!("Received a zmq message with {} parts. I only know how to handle messages with 3 parts. I'll ignore it", nb_parts);
            return Err(());
        }
        let client_id_message = &multipart_message[0];
        let payload_message = &multipart_message[2];

        let result = Self {
            client_id: client_id_message.to_vec(),
            payload: payload_message.to_vec(),
        };

        Ok(result)
    }
}
