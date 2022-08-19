// Copyright  (C) 2022, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Hove (www.kisio.com).
// Help us simplify mobility and open public transport:
// a non ending quest to the responsive locomotion way of traveling!
//
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
use anyhow::{format_err, Context, Error};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, StatusCode};
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info};

use crate::status_worker::Status;

pub struct HttpToStatusChannel {
    // http worker will send a oneshot::Sender thought `chan`
    // to status worker, and will wait for the response
    // on the oneshot::Receiver
    pub status_request_receiver: mpsc::Receiver<oneshot::Sender<Status>>,
}

pub struct HttpWorker {
    http_address: std::net::SocketAddr,
    status_request_sender: mpsc::Sender<oneshot::Sender<Status>>,
    shutdown_sender: mpsc::Sender<()>,
}

impl HttpWorker {
    pub fn new(
        http_address: std::net::SocketAddr,
        shutdown_sender: mpsc::Sender<()>,
    ) -> (Self, HttpToStatusChannel) {
        let (status_request_sender, status_request_receiver) =
            mpsc::channel::<oneshot::Sender<Status>>(1);

        let worker = Self {
            http_address,
            status_request_sender,
            shutdown_sender,
        };
        let chan = HttpToStatusChannel {
            status_request_receiver,
        };

        (worker, chan)
    }

    // run in a spawned thread
    pub fn run_in_a_thread(self) -> Result<std::thread::JoinHandle<()>, anyhow::Error> {
        // copied from https://tokio.rs/tokio/topics/bridging#sending-messages

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Failed to build tokio runtime.")?;

        let thread_builder = std::thread::Builder::new().name("loki_http_worker".to_string());
        let handle = thread_builder.spawn(move || runtime.block_on(self.run()))?;
        Ok(handle)
    }

    async fn run(self) {
        // The closure inside `make_service_fn` is run for each connection,
        // creating a 'service' to handle requests for that specific connection.
        let make_service = make_service_fn(move |_| {
            // While the status_request_sender was moved into the make_service closure,
            // we need to clone it here because this closure is called
            // once for every connection.
            //
            // Each connection could send multiple requests, so
            // the `Service` needs a clone to handle later requests.
            let status_request_sender = self.status_request_sender.clone();

            async move {
                // This is the `Service` that will handle the connection.
                // `service_fn` is a helper to convert a function that
                // returns a Response into a `Service`.
                Ok::<_, hyper::Error>(service_fn(move |http_request| {
                    handle_http_request(http_request, status_request_sender.clone())
                }))
            }
        });

        let server = hyper::Server::bind(&self.http_address).serve(make_service);

        info!("Http worker is listening on http://{}", self.http_address);

        if let Err(e) = server.await {
            error!("Http worker error: {}", e);
            let _ = self.shutdown_sender.send(()).await;
        }
    }
}

async fn handle_http_request(
    http_request: Request<Body>,
    status_request_sender: mpsc::Sender<oneshot::Sender<Status>>,
) -> Result<Response<Body>, hyper::http::Error> {
    match (http_request.method(), http_request.uri().path()) {
        // Serve some instructions at GET /status
        (&Method::GET, "/status") => match handle_status_request(status_request_sender).await {
            Ok(bytes) => Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(bytes)),
            Err(err) => {
                error!("{:#}", err);
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
            }
        },

        // Return the 404 Not Found for other routes.
        _ => {
            info!(
                "Received http request with invalid (method, path) : ({}, {})",
                http_request.method(),
                http_request.uri().path()
            );
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty());
        }
    }
}

async fn handle_status_request(
    status_request_sender: mpsc::Sender<oneshot::Sender<Status>>,
) -> Result<Vec<u8>, Error> {
    let (status_response_sender, status_response_receiver) = oneshot::channel();
    let timeout = tokio::time::Duration::from_secs(3);

    // send a request to the status worker
    let _ = status_request_sender
        .send_timeout(status_response_sender, timeout)
        .await
        .map_err(|_| format_err!("Could not send request to status worker"))?;

    let status = status_response_receiver
        .await
        .context("Could not receive response from status worker")?;

    serde_json::to_vec_pretty(&status).context("Could not serialize status to json")
}
