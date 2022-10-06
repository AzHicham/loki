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

use super::{navitia_proto, response};
use crate::{
    load_balancer::WorkerId,
    master_worker::DataAndModels,
    zmq_worker::{RequestMessage, ResponseMessage},
};
use anyhow::{format_err, Context, Error};
use launch::{
    config::{self, ComparatorType},
    datetime::DateTimeRepresent,
    loki::{
        self,
        chrono::{Duration, Utc},
        filters::{parse_filter, Filters},
        models::{base_model::PREFIX_ID_STOP_POINT, ModelRefs},
        schedule::{self, ScheduleOn, ScheduleRequestInput},
        tracing::{debug, error, info, trace, warn},
        DataTrait, NaiveDateTime, PositiveDuration, RealTimeLevel, RequestInput, TransitData,
    },
    solver::Solver,
    timer,
};
use std::{
    convert::TryFrom,
    ops::Deref,
    sync::{Arc, RwLock},
    time::SystemTime,
};
use tokio::sync::mpsc;

pub struct ComputeWorker {
    data_and_models: Arc<RwLock<DataAndModels>>,
    solver: Solver,
    worker_id: WorkerId,
    default_request_params: config::RequestParams,
    request_channel: mpsc::Receiver<RequestMessage>,
    responses_channel: mpsc::Sender<(WorkerId, ResponseMessage)>,
}

impl ComputeWorker {
    pub fn new(
        worker_id: WorkerId,
        data_and_models: Arc<RwLock<DataAndModels>>,
        default_request_params: config::RequestParams,
        responses_channel: mpsc::Sender<(WorkerId, ResponseMessage)>,
    ) -> (Self, mpsc::Sender<RequestMessage>) {
        let solver = Solver::new(0, 0);

        let (requests_channel_sender, requests_channel_receiver) = mpsc::channel(1);

        let result = Self {
            data_and_models,
            solver,
            worker_id,
            default_request_params,
            responses_channel,
            request_channel: requests_channel_receiver,
        };

        (result, requests_channel_sender)
    }

    pub fn run(mut self) {
        let run_result = self.run_loop();
        error!("Worker {} stopped : {:?}", &self.worker_id.id, run_result);
    }

    pub fn run_loop(&mut self) -> Result<(), Error> {
        info!("Worker {} has launched.", self.worker_id.id);
        loop {
            // block on receiving message

            let has_request = self.request_channel.blocking_recv();

            let request_message = has_request.ok_or_else(|| {
                format_err!(
                    "Compute worker {} request channel is closed. This worker will stop.",
                    self.worker_id.id
                )
            })?;

            let reponse_result = self.handle_request(request_message.payload);
            let proto_response = match reponse_result {
                Err(err) => {
                    error!("An error occured while solving a request : {:#?}", err);
                    make_error_response(&err)
                }
                Ok(proto_response) => proto_response,
            };
            let response_message = ResponseMessage {
                payload: proto_response,
                client_id: request_message.client_id,
            };

            // block until the response is sent
            self.responses_channel
                .blocking_send((self.worker_id, response_message))
                .with_context(|| {
                    format!(
                        "Compute worker {} could not send response. This worker will stop.",
                        self.worker_id.id
                    )
                })?;

            debug!("Worker {} sent his response.", self.worker_id.id);
        }
    }

    fn handle_request(
        &mut self,
        proto_request: navitia_proto::Request,
    ) -> Result<navitia_proto::Response, Error> {
        check_deadline(&proto_request)?;
        let request_id = proto_request.request_id.clone().unwrap_or_default();
        let requested_api = proto_request.requested_api();

        let start_request_time = SystemTime::now();
        debug!(
            "Worker {} received request on api {:?} with id '{}'",
            self.worker_id.id, requested_api, request_id
        );

        let result = match requested_api {
            navitia_proto::Api::PtPlanner => {
                let journey_request = proto_request.journeys.ok_or_else(|| {
                    format_err!("request.journey should not be empty for api PtPlanner.")
                });
                self.handle_journey_request(journey_request)
            }
            navitia_proto::Api::PlacesNearby => {
                let places_nearby_request = proto_request.places_nearby.ok_or_else(|| {
                    format_err!("request.places_nearby should not be empty for api PlacesNearby.")
                });
                self.handle_places_nearby(places_nearby_request)
            }
            navitia_proto::Api::NextDepartures => {
                let request = proto_request.next_stop_times.ok_or_else(|| {
                    format_err!(
                        "request.next_stop_times should not be empty for api NextDepartures."
                    )
                });
                self.handle_schedule(request, ScheduleOn::BoardTimes)
            }
            navitia_proto::Api::NextArrivals => {
                let request = proto_request.next_stop_times.ok_or_else(|| {
                    format_err!("request.next_stop_times should not be empty for api NextArrivals.")
                });
                self.handle_schedule(request, ScheduleOn::DebarkTimes)
            }
            _ => {
                error!("I can't handle the requested api : {:?}", requested_api);
                Err(format_err!(
                    "I can't handle the requested api : {:?}",
                    requested_api
                ))
            }
        };
        let duration = timer::duration_since(start_request_time);
        info!(
            "Worker {} took {} ms on api {:?} for request with id '{}'",
            self.worker_id.id, duration, requested_api, request_id
        );
        result
    }

    fn handle_journey_request(
        &mut self,
        proto_request: Result<navitia_proto::JourneysRequest, Error>,
    ) -> Result<navitia_proto::Response, Error> {
        match proto_request {
            Err(err) => {
                // send a response saying that the journey request could not be handled
                warn!("Could not handle journey request : {}", err);
                Ok(make_error_response(&err))
            }
            Ok(journey_request) => {
                let rw_lock_read_guard = self.data_and_models.read().map_err(|err| {
                    format_err!(
                        "Compute worker {} failed to acquire read lock on data_and_models. {}",
                        self.worker_id.id,
                        err
                    )
                })?;

                let data_and_models = rw_lock_read_guard.deref();
                match data_and_models {
                    Some((data, base_model, real_time_model)) => {
                        let model_refs = ModelRefs::new(base_model, real_time_model);
                        let solve_result = solve(
                            &journey_request,
                            data,
                            &model_refs,
                            &mut self.solver,
                            &self.default_request_params,
                        );
                        Ok(make_proto_response(solve_result, &model_refs))
                    }
                    None => Ok(make_error_response(&format_err!("No data loaded."))),
                }
                // RwLock is released
            }
        }
    }

    fn handle_places_nearby(
        &mut self,
        proto_request: Result<navitia_proto::PlacesNearbyRequest, Error>,
    ) -> Result<navitia_proto::Response, Error> {
        match proto_request {
            Err(err) => {
                // send a response saying that the journey request could not be handled
                warn!("Could not handle places nearby request : {}", err);
                Ok(make_error_response(&err))
            }
            Ok(places_nearby_request) => {
                let rw_lock_read_guard = self.data_and_models.read().map_err(|err| {
                    format_err!(
                        "Compute worker {} failed to acquire read lock on data_and_models. {}",
                        self.worker_id.id,
                        err
                    )
                })?;

                let data_and_models = rw_lock_read_guard.deref();
                match data_and_models {
                    Some((_, base_model, real_time_model)) => {
                        let model_refs = ModelRefs::new(base_model, real_time_model);
                        let radius = places_nearby_request.distance;
                        let uri = places_nearby_request.uri;
                        let start_page =
                            usize::try_from(places_nearby_request.start_page).unwrap_or(0);
                        let count = usize::try_from(places_nearby_request.count).unwrap_or(0);
                        let depth = usize::try_from(places_nearby_request.depth).unwrap_or(2);

                        match self.solver.solve_places_nearby(&model_refs, &uri, radius) {
                            Ok(mut places_nearby_iter) => {
                                Ok(response::make_places_nearby_proto_response(
                                    &model_refs,
                                    &mut places_nearby_iter,
                                    start_page,
                                    count,
                                    depth,
                                ))
                            }
                            Err(err) => Ok(make_error_response(&format_err!("{}", err))),
                        }
                    }
                    None => Ok(make_error_response(&format_err!("No data loaded."))),
                }
                // RwLock is released
            }
        }
    }

    fn handle_schedule(
        &mut self,
        proto_request: Result<navitia_proto::NextStopTimeRequest, Error>,
        schedule_on: ScheduleOn,
    ) -> Result<navitia_proto::Response, Error> {
        match proto_request {
            Err(err) => {
                // send a response saying that the journey request could not be handled
                warn!("Could not handle schedule request : {}", err);
                Ok(make_error_response(&err))
            }
            Ok(request) => {
                let rw_lock_read_guard = self.data_and_models.read().map_err(|err| {
                    format_err!(
                        "Compute worker {} failed to acquire read lock on data_and_models. {}",
                        self.worker_id.id,
                        err
                    )
                })?;

                let data_and_models = rw_lock_read_guard.deref();
                match data_and_models {
                    None => Ok(make_error_response(&format_err!("No data loaded."))),
                    Some((data, base_model, real_time_model)) => {
                        let model_refs = ModelRefs::new(base_model, real_time_model);

                        let schedule_request =
                            make_schedule_request(&request, schedule_on, &model_refs, data);

                        match schedule_request {
                            Err(err) => {
                                error!(
                                    "Error while creating request input for next_departure : {:?}",
                                    err
                                );
                                Ok(make_error_response(&format_err!("{}", err)))
                            }
                            Ok(request_input) => {
                                let has_filters =
                                    schedule::generate_vehicle_filters_for_schedule_request(
                                        &request.forbidden_uri,
                                        &model_refs,
                                    );
                                let response = self.solver.solve_schedule(
                                    data,
                                    &model_refs,
                                    &request_input,
                                    has_filters,
                                );
                                match response {
                                    Result::Err(err) => {
                                        error!("Error while solving schedule request : {:?}", err);
                                        Ok(make_error_response(&format_err!("{}", err)))
                                    }
                                    Ok(response) => {
                                        let start_page =
                                            usize::try_from(request.start_page).unwrap_or(0);
                                        let count = usize::try_from(request.count).unwrap_or(0);

                                        let proto_response = response::make_schedule_proto_response(
                                            &request_input,
                                            response,
                                            &model_refs,
                                            start_page,
                                            count,
                                        );
                                        Ok(proto_response)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn check_deadline(proto_request: &navitia_proto::Request) -> Result<(), Error> {
    if let Some(deadline_str) = &proto_request.deadline {
        let datetime_result = NaiveDateTime::parse_from_str(deadline_str, "%Y%m%dT%H%M%S,%f");
        match datetime_result {
            Ok(datetime) => {
                let now = Utc::now().naive_utc();
                if now > datetime {
                    return Err(format_err!("Deadline reached."));
                }
            }
            Err(err) => {
                warn!(
                    "Could not parse deadline string {}. Error : {}",
                    deadline_str, err
                );
            }
        }
    }
    Ok(())
}

fn solve(
    journey_request: &navitia_proto::JourneysRequest,
    data: &TransitData,
    model: &ModelRefs<'_>,
    solver: &mut Solver,
    default_request_params: &config::RequestParams,
) -> Result<(RequestInput, Vec<loki::response::Response>), Error> {
    // println!("{:#?}", journey_request);
    let departures_stop_point_and_fallback_duration = journey_request
        .origin
        .iter()
        .enumerate()
        .filter_map(|(idx, location_context)| {
            let duration = u32::try_from(location_context.access_duration)
                .map(|duration_u32| PositiveDuration::from_hms(0, 0, duration_u32))
                .ok()
                .or_else(|| {
                    warn!(
                        "The {}th departure stop point {} has a fallback duration {} \
                        that cannot be converted to u32. I ignore it",
                        idx, location_context.place, location_context.access_duration
                    );
                    None
                })?;
            let stop_point_uri = location_context
                .place
                .strip_prefix(PREFIX_ID_STOP_POINT)
                .map(ToString::to_string)
                .or_else(|| {
                    warn!(
                        "The {}th arrival stop point has an uri {} \
                        that doesn't start with `stop_point:`. I ignore it",
                        idx, location_context.place,
                    );
                    None
                })?;
            // let trimmed = location_context.place.trim_start_matches("stop_point:");
            // let stop_point_uri = format!("StopPoint:{}", trimmed);
            // let stop_point_uri = location_context.place.clone();
            Some((stop_point_uri, duration))
        })
        .collect();

    let arrivals_stop_point_and_fallback_duration = journey_request
        .destination
        .iter()
        .enumerate()
        .filter_map(|(idx, location_context)| {
            let duration = u32::try_from(location_context.access_duration)
                .map(|duration_u32| PositiveDuration::from_hms(0, 0, duration_u32))
                .ok()
                .or_else(|| {
                    warn!(
                        "The {}th arrival stop point {} has a fallback duration {}\
                        that cannot be converted to u32. I ignore it",
                        idx, location_context.place, location_context.access_duration
                    );
                    None
                })?;
            let stop_point_uri = location_context
                .place
                .strip_prefix(PREFIX_ID_STOP_POINT)
                .map(ToString::to_string)
                .or_else(|| {
                    warn!(
                        "The {}th arrival stop point has an uri {} \
                        that doesn't start with `stop_point:`. I ignore it",
                        idx, location_context.place,
                    );
                    None
                })?;
            // let trimmed = location_context.place.trim_start_matches("stop_point:");
            // let stop_point_uri = format!("StopPoint:{}", trimmed);
            // let stop_point_uri = location_context.place.clone();
            Some((stop_point_uri, duration))
        })
        .collect();

    let departure_timestamp_u64 = journey_request
        .datetimes
        .first()
        .ok_or_else(|| format_err!("No departure datetime provided."))?;
    let departure_timestamp_i64 = i64::try_from(*departure_timestamp_u64).with_context(|| {
        format!(
            "The departure datetime {} cannot be converted to a valid i64 timestamp.",
            departure_timestamp_u64
        )
    })?;
    let departure_datetime = loki::NaiveDateTime::from_timestamp(departure_timestamp_i64, 0);

    debug!(
        "Requested timestamp {}, datetime {}",
        departure_timestamp_u64, departure_datetime
    );

    let max_journey_duration = u32::try_from(journey_request.max_duration)
        .map(|duration| PositiveDuration::from_hms(0, 0, duration))
        .unwrap_or_else(|_| {
            warn!(
                "The max duration {} cannot be converted to a u32.\
                I'm gonna use the default {} as max duration",
                journey_request.max_duration, default_request_params.max_journey_duration
            );
            default_request_params.max_journey_duration
        });

    let max_nb_of_legs = u8::try_from(journey_request.max_transfers + 1).unwrap_or_else(|_| {
        warn!(
            "The max nb of transfers {} cannot be converted to a u8.\
                    I'm gonna use the default {} as the max nb of legs",
            journey_request.max_transfers, default_request_params.max_nb_of_legs
        );
        default_request_params.max_nb_of_legs
    });

    let must_be_wheelchair_accessible = journey_request.wheelchair.unwrap_or(false);
    let must_be_bike_accessible = journey_request.bike_in_pt.unwrap_or(false);

    let forbidden_filters = journey_request
        .forbidden_uris
        .iter()
        .filter_map(|forbidden_uri| parse_filter(model, forbidden_uri, "forbidden_uri[]"));

    let allowed_filters = journey_request
        .allowed_id
        .iter()
        .filter_map(|forbidden_uri| parse_filter(model, forbidden_uri, "forbidden_uri[]"));

    let data_filters = Filters::new(
        forbidden_filters,
        allowed_filters,
        must_be_wheelchair_accessible,
        must_be_bike_accessible,
    );

    let real_time_level = if journey_request.realtime_level.is_some() {
        match journey_request.realtime_level() {
            navitia_proto::RtLevel::BaseSchedule => RealTimeLevel::Base,
            navitia_proto::RtLevel::Realtime | navitia_proto::RtLevel::AdaptedSchedule => {
                RealTimeLevel::RealTime
            }
        }
    } else {
        RealTimeLevel::RealTime
    };

    let comparator_type = if journey_request.criteria.is_some() {
        match journey_request.criteria() {
            navitia_proto::Criteria::Classic => ComparatorType::Basic,
            navitia_proto::Criteria::Robustness => ComparatorType::Robustness,
        }
    } else {
        ComparatorType::Basic
    };

    let leg_arrival_penalty = journey_request
        .arrival_transfer_penalty
        .and_then(|seconds_i32| PositiveDuration::try_from(seconds_i32).ok())
        .unwrap_or(default_request_params.leg_arrival_penalty);

    let leg_walking_penalty = journey_request
        .walking_transfer_penalty
        .and_then(|seconds_i32| PositiveDuration::try_from(seconds_i32).ok())
        .unwrap_or(default_request_params.leg_walking_penalty);

    let request_input = RequestInput {
        datetime: departure_datetime,
        departures_stop_point_and_fallback_duration,
        arrivals_stop_point_and_fallback_duration,
        leg_arrival_penalty,
        leg_walking_penalty,
        max_nb_of_legs,
        max_journey_duration,
        too_late_threshold: default_request_params.too_late_threshold,
        real_time_level,
    };

    let datetime_represent = match journey_request.clockwise {
        true => DateTimeRepresent::Departure,
        false => DateTimeRepresent::Arrival,
    };
    trace!("{:#?}", request_input);

    let responses = solver.solve_journey_request(
        data,
        model,
        &request_input,
        data_filters,
        comparator_type,
        datetime_represent,
    )?;
    for response in &responses {
        debug!("{}", response.print(model)?);
    }
    Ok((request_input, responses))
}

fn make_proto_response(
    solve_result: Result<(RequestInput, Vec<loki::Response>), Error>,
    model: &ModelRefs<'_>,
) -> navitia_proto::Response {
    match solve_result {
        Result::Err(err) => {
            error!("Error while solving request : {:?}", err);
            make_error_response(&err)
        }
        Ok((request_input, journeys)) => {
            let response_result = response::make_response(&request_input, journeys, model);
            match response_result {
                Result::Err(err) => {
                    error!(
                        "Error while encoding protobuf response for request : {:?}",
                        err
                    );
                    make_error_response(&err)
                }
                Ok(resp) => {
                    // trace!("{:#?}", resp);
                    resp
                }
            }
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

fn make_schedule_request<'a>(
    proto: &'a navitia_proto::NextStopTimeRequest,
    schedule_on: ScheduleOn,
    model: &ModelRefs,
    data: &TransitData,
) -> Result<ScheduleRequestInput, Error> {
    let input_filter = match schedule_on {
        ScheduleOn::BoardTimes => &proto.departure_filter,
        ScheduleOn::DebarkTimes => &proto.arrival_filter,
    };

    let from_datetime = {
        let proto_datetime = proto
            .from_datetime
            .ok_or(format_err!("No from_datetime provided."))?;

        let timestamp = i64::try_from(proto_datetime).with_context(|| {
            format!(
                "Datetime {} cannot be converted to a valid i64 timestamp.",
                proto_datetime
            )
        })?;
        loki::NaiveDateTime::from_timestamp(timestamp, 0)
    };

    let until_datetime = from_datetime + Duration::seconds(i64::from(proto.duration));
    let until_datetime = std::cmp::min(until_datetime, data.calendar().last_datetime());

    let nb_max_responses = usize::try_from(proto.nb_stoptimes).with_context(|| {
        format!(
            "nb_stoptimes {} cannot be converted to usize.",
            proto.nb_stoptimes
        )
    })?;

    let real_time_level = match proto.realtime_level() {
        navitia_proto::RtLevel::BaseSchedule => RealTimeLevel::Base,
        navitia_proto::RtLevel::Realtime | navitia_proto::RtLevel::AdaptedSchedule => {
            RealTimeLevel::RealTime
        }
    };

    let input_stop_points = schedule::generate_stops_for_schedule_request(
        input_filter,
        proto.forbidden_uri.as_slice(),
        model,
    );

    Ok(ScheduleRequestInput {
        input_stop_points,
        from_datetime,
        until_datetime,
        nb_max_responses,
        real_time_level,
        schedule_on,
    })
}
