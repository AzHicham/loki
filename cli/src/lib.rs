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

use loki::config;
use loki::transit_model;
use loki::PositiveDuration;
use loki::{log::trace, response, traits::RequestInput, transit_model::Model,
};

use loki::traits;

use slog::slog_o;
use slog::Drain;
use slog_async::OverflowStrategy;

use std::{
    fmt::{Debug, },
};

use chrono::NaiveDateTime;
use failure::{bail, Error};

// pub mod stop_areas;

pub mod random;

use serde::{Serialize, Deserialize};




#[derive(Serialize, Deserialize, Debug)]
pub struct BaseConfig {

    #[serde(flatten)]
    pub launch_params : config::LaunchParams,

    #[serde(flatten)]
    pub request_params: config::RequestParams,

    /// Departure datetime of the query, formatted like 20190628T163215
    /// If none is given, all queries will be made at 08:00:00 on the first
    /// valid day of the dataset
    pub departure_datetime: Option<String>,

    /// Which comparator to use for the request
    /// "basic" or "loads"
    #[serde(default)]
    pub comparator_type: config::ComparatorType,
}



pub fn init_logger() -> slog_scope::GlobalLoggerGuard {
    let decorator = slog_term::TermDecorator::new().stdout().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let mut builder = slog_envlogger::LogBuilder::new(drain).filter(None, slog::FilterLevel::Info);
    if let Ok(s) = std::env::var("RUST_LOG") {
        builder = builder.parse(&s);
    }
    let drain = slog_async::Async::new(builder.build())
        .chan_size(256) // Double the default size
        .overflow_strategy(OverflowStrategy::Block)
        .build()
        .fuse();
    let logger = slog::Logger::root(drain, slog_o!());

    let scope_guard = slog_scope::set_global_logger(logger);
    slog_stdlog::init().unwrap();
    scope_guard
}

pub fn make_query_stop_area(
    model: &transit_model::Model,
    from_stop_area: &str,
    to_stop_area: &str,
) -> Result<(Vec<String>, Vec<String>), Error> {
    use std::collections::BTreeSet;
    let mut start_sa_set = BTreeSet::new();
    let from_stop_area_idx = model
        .stop_areas
        .get_idx(from_stop_area)
        .ok_or_else(|| failure::format_err!("No stop area named `{}` found.", from_stop_area))?;
    start_sa_set.insert(from_stop_area_idx);
    let start_stop_points: Vec<String> = model
        .get_corresponding(&start_sa_set)
        .iter()
        .map(|idx| model.stop_points[*idx].id.clone())
        .collect();
    let mut end_sa_set = BTreeSet::new();

    let to_stop_area_idx = model
        .stop_areas
        .get_idx(to_stop_area)
        .ok_or_else(|| failure::format_err!("No stop area named `{}` found.", to_stop_area))?;
    end_sa_set.insert(to_stop_area_idx);
    let end_stop_points = model
        .get_corresponding(&end_sa_set)
        .iter()
        .map(|idx| model.stop_points[*idx].id.clone())
        .collect();
    Ok((start_stop_points, end_stop_points))
}

pub fn parse_datetime(string_datetime: &str) -> Result<NaiveDateTime, Error> {
    let try_datetime = NaiveDateTime::parse_from_str(string_datetime, "%Y%m%dT%H%M%S");
    match try_datetime {
        Ok(datetime) => Ok(datetime),
        Err(_) => bail!(
            "Unable to parse {} as a datetime. Expected format is 20190628T163215",
            string_datetime
        ),
    }
}

pub fn solve<'data, Data, Solver>(
    start_stop_area_uri: &str,
    end_stop_area_uri: &str,
    solver: &mut Solver, // &mut MultiCriteriaRaptor<DepartAfter<'data, Data>>,
    model: &Model,
    data: &'data Data,
    departure_datetime: &NaiveDateTime,
    config: &BaseConfig,
) -> Result<Vec<response::Response>, Error>
where
    Solver: traits::Solver<'data, Data>,
    Data: traits::DataWithIters,
{
    trace!(
        "Request start stop area : {}, end stop_area : {}",
        start_stop_area_uri,
        end_stop_area_uri
    );
    let (start_stop_point_uris, end_stop_point_uris) =
        make_query_stop_area(model, start_stop_area_uri, end_stop_area_uri)?;
    let departures_stop_point_and_fallback_duration = start_stop_point_uris
        .iter()
        .map(|uri| (uri.as_str(), PositiveDuration::zero()));

    let arrivals_stop_point_and_fallback_duration = end_stop_point_uris
        .iter()
        .map(|uri| (uri.as_str(), PositiveDuration::zero()));


    let request_input = RequestInput {
        departure_datetime: *departure_datetime,
        departures_stop_point_and_fallback_duration,
        arrivals_stop_point_and_fallback_duration,
        params : config.request_params.clone(),
    };

    let responses = solver.solve_request(data, model, request_input, &config.comparator_type)?;

    Ok(responses)
}
