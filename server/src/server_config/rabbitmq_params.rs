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

use loki_launch::{
    config::{parse_env_var, read_env_var},
    loki::PositiveDuration,
};

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
        let endpoint = read_env_var("LOKI_RABBITMQ_ENDPOINT", default_endpoint(), |s| {
            s.to_string()
        });

        let exchange = read_env_var("LOKI_RABBITMQ_EXCHANGE", default_exchange(), |s| {
            s.to_string()
        });

        let realtime_topics =
            read_env_var("LOKI_REALTIME_TOPICS", default_real_time_topics(), |s| {
                // split at ";" characters
                let iter = s.split_terminator(';');
                iter.map(|substring| substring.trim().to_string())
                    .filter(|s| !s.is_empty()) // remove empty strings
                    .collect::<Vec<String>>()
            });

        let realtime_update_interval = parse_env_var(
            "LOKI_REALTIME_UPDATE_INTERVAL",
            default_real_time_update_interval(),
            PositiveDuration::from_str,
        );

        let connect_retry_interval = parse_env_var(
            "LOKI_RABBITMQ_CONNECT_RETRY_INTERVAL",
            default_connect_retry_interval(),
            PositiveDuration::from_str,
        );

        let reload_kirin_request_time_to_live = parse_env_var(
            "LOKI_RELOAD_KIRIN_REQUEST_TIME_TO_LIVE",
            default_reload_kirin_request_time_to_live(),
            PositiveDuration::from_str,
        );

        let reload_kirin_timeout = parse_env_var(
            "LOKI_RELOAD_KIRIN_TIMEOUT",
            default_reload_kirin_timeout(),
            PositiveDuration::from_str,
        );

        let reload_queue_expires = parse_env_var(
            "LOKI_RELOAD_QUEUE_EXPIRES",
            default_reload_queue_expires(),
            PositiveDuration::from_str,
        );

        let realtime_queue_expires = parse_env_var(
            "LOKI_REALTIME_QUEUE_EXPIRES",
            default_realtime_queue_expires(),
            PositiveDuration::from_str,
        );

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

#[cfg(test)]
mod tests {
    use crate::server_config::rabbitmq_params::RabbitMqParams;

    #[test]
    fn test_single_topic() {
        temp_env::with_var("LOKI_REALTIME_TOPICS", Some("my-topic"), || {
            let params = RabbitMqParams::new_from_env_vars();

            assert_eq!(params.realtime_topics, vec!["my-topic"]);
        })
    }

    #[test]
    fn test_two_topics() {
        temp_env::with_var(
            "LOKI_REALTIME_TOPICS",
            Some("my-topic; my.other.topic"),
            || {
                let params = RabbitMqParams::new_from_env_vars();

                assert_eq!(params.realtime_topics, vec!["my-topic", "my.other.topic"]);
            },
        )
    }

    #[test]
    fn test_middle_topic_empty() {
        temp_env::with_var(
            "LOKI_REALTIME_TOPICS",
            Some("my-topic; ; my.other.topic"),
            || {
                let params = RabbitMqParams::new_from_env_vars();

                assert_eq!(params.realtime_topics, vec!["my-topic", "my.other.topic"]);
            },
        )
    }

    #[test]
    fn test_empty_topics() {
        temp_env::with_var("LOKI_REALTIME_TOPICS", Some(" ; ;"), || {
            let params = RabbitMqParams::new_from_env_vars();

            assert!(params.realtime_topics.is_empty());
        })
    }
}
