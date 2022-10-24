// Copyright  (C) 2021, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Hove (www.kisio.com).
// Help us simplify mobility and open public transport:
// a non ending quest to the responsive locomotion way of traveling!
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

use std::str::FromStr;

use hyper::{StatusCode, Uri};
pub use loki_server;
use loki_server::server_config::http_params::HttpParams;

pub async fn status_test(http_params: &HttpParams) {
    let client = hyper::client::Client::new();
    let address = http_params.http_address.to_string();
    let uri_string = format!("http://{}/status", address);
    let uri = Uri::from_str(&uri_string).unwrap();

    let response = client.get(uri).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

pub async fn health_test(http_params: &HttpParams) {
    let client = hyper::client::Client::new();
    let address = http_params.http_address.to_string();
    let uri_string = format!("http://{}/health", address);
    let uri = Uri::from_str(&uri_string).unwrap();

    let response = client.get(uri).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
