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

use loki_launch::{config::parse_env_var, loki::PositiveDuration};

use serde::{Deserialize, Serialize};
use std::{fmt::Debug, str::FromStr};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct HttpParams {
    /// http endpoint for health and status checks
    /// Something like 0.0.0.0:3000
    /// will provide two routes
    /// - http://0.0.0.0:3000/status
    /// - http://0.0.0.0:3000/health
    #[serde(default = "default_http_address")]
    pub http_address: std::net::SocketAddr,

    /// How long to wait before deciding that the request failed
    #[serde(default = "default_http_request_timeout")]
    pub http_request_timeout: PositiveDuration,
}

pub fn default_http_address() -> std::net::SocketAddr {
    // default to 0.0.0.0 to be reachable from within a docker container
    // see
    // https://stackoverflow.com/questions/59179831/docker-app-server-ip-address-127-0-0-1-difference-of-0-0-0-0-ip
    ([0, 0, 0, 0], 3000).into()
}

pub fn default_http_request_timeout() -> PositiveDuration {
    PositiveDuration::from_hms(0, 0, 10)
}

impl Default for HttpParams {
    fn default() -> Self {
        Self {
            http_address: default_http_address(),
            http_request_timeout: default_http_request_timeout(),
        }
    }
}

impl HttpParams {
    pub fn new_from_env_vars() -> Self {
        let http_address = parse_env_var(
            "LOKI_HTTP_ADDRESS",
            default_http_address(),
            std::net::SocketAddr::from_str,
        );

        let http_request_timeout = parse_env_var(
            "LOKI_HTTP_REQUEST_TIMEOUT",
            default_http_request_timeout(),
            PositiveDuration::from_str,
        );

        Self {
            http_address,
            http_request_timeout,
        }
    }
}
