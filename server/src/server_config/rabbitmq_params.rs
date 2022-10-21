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

use loki_launch::loki::PositiveDuration;

use serde::{Deserialize, Serialize};
use std::{fmt::Debug, str::FromStr};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct RabbitMqParams {
    #[serde(default = "default_endpoint")]
    pub endpoint: String,

    #[serde(default = "default_exchange")]
    pub exchange: String,

    #[serde(default = "default_real_time_topics")]
    pub realtime_topics: Vec<String>,

    #[serde(default = "default_real_time_update_interval")]
    pub realtime_update_interval: PositiveDuration,

    #[serde(default = "default_connect_retry_interval")]
    pub connect_retry_interval: PositiveDuration,

    #[serde(default = "default_reload_kirin_request_time_to_live")]
    pub reload_kirin_request_time_to_live: PositiveDuration,

    #[serde(default = "default_reload_kirin_timeout")]
    pub reload_kirin_timeout: PositiveDuration,

    #[serde(default = "default_reload_queue_expires")]
    pub reload_queue_expires: PositiveDuration,

    #[serde(default = "default_realtime_queue_expires")]
    pub realtime_queue_expires: PositiveDuration,
}

pub fn default_endpoint() -> String {
    "amqp://guest:guest@rabbitmq:5672".to_string()
}

pub fn default_exchange() -> String {
    "navitia".to_string()
}

pub fn default_real_time_topics() -> Vec<String> {
    Vec::new()
}

pub fn default_real_time_update_interval() -> PositiveDuration {
    PositiveDuration::from_str("00:00:30").unwrap()
}

pub fn default_connect_retry_interval() -> PositiveDuration {
    PositiveDuration::from_str("00:00:10").unwrap()
}

pub fn default_reload_kirin_request_time_to_live() -> PositiveDuration {
    PositiveDuration::from_str("00:00:02").unwrap()
}

pub fn default_reload_kirin_timeout() -> PositiveDuration {
    PositiveDuration::from_str("00:00:10").unwrap()
}

pub fn default_reload_queue_expires() -> PositiveDuration {
    PositiveDuration::from_str("02:00:00").unwrap()
}

pub fn default_realtime_queue_expires() -> PositiveDuration {
    PositiveDuration::from_str("02:00:00").unwrap()
}

impl Default for RabbitMqParams {
    fn default() -> Self {
        Self {
            endpoint: default_endpoint(),
            exchange: default_exchange(),
            realtime_topics: default_real_time_topics(),
            realtime_update_interval: default_real_time_update_interval(),
            connect_retry_interval: default_connect_retry_interval(),
            reload_kirin_request_time_to_live: default_reload_kirin_request_time_to_live(),
            reload_kirin_timeout: default_reload_kirin_timeout(),
            reload_queue_expires: default_reload_queue_expires(),
            realtime_queue_expires: default_realtime_queue_expires(),
        }
    }
}

impl RabbitMqParams {
    pub fn new_from_env_vars() -> Self {
        let endpoint =
            std::env::var("LOKI_RABBITMQ_ENDPOINT").unwrap_or_else(|_| default_endpoint());
        let exchange =
            std::env::var("LOKI_RABBITMQ_EXCHANGE").unwrap_or_else(|_| default_exchange());
        let realtime_topics: Vec<String> = {
            let string = std::env::var("LOKI_REALTIME_TOPICS").unwrap_or_default();
            // split at ";" characters
            let iter = string.split_terminator(';');
            iter.map(|substring| substring.trim().to_string()).collect()
        };
        let realtime_update_interval = {
            let s = std::env::var("LOKI_REALTIME_UPDATE_INTERVAL").unwrap_or_default();
            PositiveDuration::from_str(&s).unwrap_or_else(|_| default_real_time_update_interval())
        };
        let connect_retry_interval = {
            let s = std::env::var("LOKI_RABBITMQ_CONNECT_RETRY_INTERVAL").unwrap_or_default();
            PositiveDuration::from_str(&s).unwrap_or_else(|_| default_connect_retry_interval())
        };
        let reload_kirin_request_time_to_live = {
            let s = std::env::var("LOKI_RELOAD_KIRIN_REQUEST_TIME_TO_LIVE").unwrap_or_default();
            PositiveDuration::from_str(&s)
                .unwrap_or_else(|_| default_reload_kirin_request_time_to_live())
        };
        let reload_kirin_timeout = {
            let s = std::env::var("LOKI_RELOAD_KIRIN_TIMEOUT").unwrap_or_default();
            PositiveDuration::from_str(&s).unwrap_or_else(|_| default_reload_kirin_timeout())
        };
        let reload_queue_expires = {
            let s = std::env::var("LOKI_RELOAD_QUEUE_EXPIRES").unwrap_or_default();
            PositiveDuration::from_str(&s).unwrap_or_else(|_| default_reload_queue_expires())
        };
        let realtime_queue_expires = {
            let s = std::env::var("LOKI_REALTIME_QUEUE_EXPIRES").unwrap_or_default();
            PositiveDuration::from_str(&s).unwrap_or_else(|_| default_realtime_queue_expires())
        };
        Self {
            endpoint,
            exchange,
            realtime_topics,
            realtime_update_interval,
            connect_retry_interval,
            reload_kirin_request_time_to_live,
            reload_kirin_timeout,
            reload_queue_expires,
            realtime_queue_expires,
        }
    }
}
