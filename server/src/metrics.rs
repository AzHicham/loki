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
use anyhow::{bail, Context};
use std::time::SystemTime;
use tracing::{error, info};

use prometheus::{self, process_collector::ProcessCollector, Histogram, HistogramOpts, Registry};

use lazy_static::lazy_static;

lazy_static! {
    static ref METRICS: Option<Metrics> = create_metrics();
}

struct Metrics {
    registry: Registry,
    journeys_durations: Histogram,
    places_nearby_durations: Histogram,
    next_departures_arrivals_durations: Histogram,
    zmq_status_durations: Histogram,
    http_status_durations: Histogram,
    reload_durations: Histogram,
    realtime_ingestion_durations: Histogram,
}

pub enum Metric {
    Journeys,
    PlacesNearby,
    NextDeparturesArrivals,
    ZmqStatus,
    HttpStatus,
    Reload,
    RealtimeIngestion,
}

pub fn initialize_metrics() {
    lazy_static::initialize(&METRICS);
}

fn create_metrics() -> Option<Metrics> {
    let registry = Registry::new_custom(Some("loki".to_string()), None)
        .map_err(|err| error!("Failed to create prometheus registry {:?}", err))
        .ok()?;
    let journeys_durations = create_journeys_durations_histogram(&registry)?;
    let places_nearby_durations = create_places_nearby_durations_histogram(&registry)?;
    let next_departures_arrivals_durations =
        create_next_departures_arrivals_durations_histogram(&registry)?;
    let zmq_status_durations = create_zmq_status_histogram(&registry)?;
    let http_status_durations = create_http_status_histogram(&registry)?;
    let reload_durations = create_reload_histogram(&registry)?;
    let realtime_ingestion_durations = create_realtime_ingestion_histogram(&registry)?;

    let process_metrics = ProcessCollector::for_self();
    registry
        .register(Box::new(process_metrics))
        .map_err(|err| error!("Failed to register process metrics {:?}", err))
        .ok()?;

    info!("Metrics created");
    Some(Metrics {
        registry,
        journeys_durations,
        places_nearby_durations,
        next_departures_arrivals_durations,
        zmq_status_durations,
        http_status_durations,
        reload_durations,
        realtime_ingestion_durations,
    })
}

fn register_histogram(
    registry: &Registry,
    name: &str,
    help: &str,
    buckets: Vec<f64>,
) -> Option<Histogram> {
    let opts = HistogramOpts::new(name, help).buckets(buckets);
    let histogram = Histogram::with_opts(opts)
        .map_err(|err| error!("Failed to create {} histogram {:?}", name, err))
        .ok()?;
    registry
        .register(Box::new(histogram.clone()))
        .map_err(|err| error!("Failed to register {} histogram {:?}", name, err))
        .ok()?;
    Some(histogram)
}

fn create_journeys_durations_histogram(registry: &Registry) -> Option<Histogram> {
    let name = "journeys_durations";
    let help = "durations (in seconds) for handling journeys requests";
    let buckets = vec![0.01, 0.05, 0.1, 0.2, 0.4, 1.0, 5.0];
    register_histogram(registry, name, help, buckets)
}

fn create_places_nearby_durations_histogram(registry: &Registry) -> Option<Histogram> {
    let name = "places_nearby_durations";
    let help = "durations (in seconds) for handling places nearby requests";
    let buckets = vec![0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0];
    register_histogram(registry, name, help, buckets)
}

fn create_next_departures_arrivals_durations_histogram(registry: &Registry) -> Option<Histogram> {
    let name = "next_departures_arrivals_durations";
    let help = "durations (in seconds) for handling next departures and next arrivals requests";
    let buckets = vec![0.01, 0.05, 0.1, 0.2, 0.4, 1.0, 5.0];
    register_histogram(registry, name, help, buckets)
}

fn create_zmq_status_histogram(registry: &Registry) -> Option<Histogram> {
    let name = "zmq_status_durations";
    let help = "durations (in seconds) for handling zmq status requests";
    let buckets = vec![0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0];
    register_histogram(registry, name, help, buckets)
}

fn create_http_status_histogram(registry: &Registry) -> Option<Histogram> {
    let name = "http_status_durations";
    let help = "durations (in seconds) for handling http status requests";
    let buckets = vec![0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0];
    register_histogram(registry, name, help, buckets)
}

fn create_reload_histogram(registry: &Registry) -> Option<Histogram> {
    let name = "reload_durations";
    let help = "durations (in seconds) for reloads";
    let buckets = vec![1.0, 5.0, 20.0, 60.0, 120.0, 300.0];
    register_histogram(registry, name, help, buckets)
}

fn create_realtime_ingestion_histogram(registry: &Registry) -> Option<Histogram> {
    let name = "realtime_ingestion_durations";
    let help = "durations (in seconds) for realtime ingestion";
    let buckets = vec![0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 60.0];
    register_histogram(registry, name, help, buckets)
}

pub fn observe(metric: Metric, time: SystemTime) {
    let metrics: &Metrics = match *METRICS {
        Some(ref metrics) => metrics,
        None => {
            return;
        }
    };
    let Ok(duration) = time.elapsed() else {
        return;
    };

    let duration_f64 = duration.as_secs_f64();
    let histogram = match metric {
        Metric::Journeys => &metrics.journeys_durations,
        Metric::PlacesNearby => &metrics.places_nearby_durations,
        Metric::NextDeparturesArrivals => &metrics.next_departures_arrivals_durations,
        Metric::ZmqStatus => &metrics.zmq_status_durations,
        Metric::HttpStatus => &metrics.http_status_durations,
        Metric::Reload => &metrics.reload_durations,
        Metric::RealtimeIngestion => &metrics.realtime_ingestion_durations,
    };
    histogram.observe(duration_f64);
}

pub fn export_metrics() -> Result<String, anyhow::Error> {
    let metrics: &Metrics = match *METRICS {
        Some(ref metrics) => metrics,
        None => {
            bail!("Cannot export uninitalized metrics");
        }
    };

    let metric_families = metrics.registry.gather();
    let encoder = prometheus::TextEncoder::new();
    encoder
        .encode_to_string(&metric_families)
        .context("Failed to encode metrics to String")
}
