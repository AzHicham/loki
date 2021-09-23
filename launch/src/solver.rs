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

use std::{fmt::Debug, time::SystemTime};

use loki::tracing::{debug, info, trace};

use loki::{
    response, transit_model, BadRequest, MultiCriteriaRaptor, RequestDebug, RequestIO,
    RequestInput, RequestTypes, RequestWithIters,
};

use crate::{datetime::DateTimeRepresent, filters::Filters};

use super::config;
use crate::loki::{DataTrait, TransitData};
use loki::{
    request::{self, generic_request::Types},
    timetables::{Timetables as TimetablesTrait, TimetablesIter},
    transit_data_filtered::TransitDataFiltered,
};

pub struct Solver<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
{
    engine: MultiCriteriaRaptor<Types<TransitData<Timetables>>>,

    allowed_vehicle_journey_idxs: Vec<bool>, // memory used for filtered requests
    allowed_stop_point_idxs: Vec<bool>,      // memory used for filtered requests
}

impl<Timetables> Solver<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
    Timetables::Mission: 'static,
    Timetables::Position: 'static,
{
    pub fn new(nb_of_stops: usize, nb_of_missions: usize) -> Self {
        Self {
            engine: MultiCriteriaRaptor::new(nb_of_stops, nb_of_missions),
            allowed_stop_point_idxs: Vec::new(),
            allowed_vehicle_journey_idxs: Vec::new(),
        }
    }

    fn fill_allowed_stops_and_vehicles(&mut self, model: &transit_model::Model, filters: &Filters) {
        self.allowed_vehicle_journey_idxs
            .resize(model.vehicle_journeys.len(), true);
        for (idx, _) in model.vehicle_journeys.iter() {
            self.allowed_vehicle_journey_idxs[idx.get()] =
                filters.is_vehicle_journey_valid(&idx, model);
        }
        self.allowed_stop_point_idxs
            .resize(model.stop_points.len(), true);
        for (idx, _) in model.stop_points.iter() {
            self.allowed_stop_point_idxs[idx.get()] = filters.is_stop_point_valid(&idx, model);
        }
    }

    pub fn solve_request(
        &mut self,
        data: &TransitData<Timetables>,
        model: &transit_model::Model,
        request_input: &RequestInput,
        has_filters: Option<Filters>,
        comparator_type: &config::ComparatorType,
        datetime_represent: &DateTimeRepresent,
    ) -> Result<Vec<response::Response>, BadRequest>
    where
        Self: Sized,
    {
        use crate::datetime::DateTimeRepresent::*;
        use config::ComparatorType::*;

        if let Some(filters) = has_filters {
            self.fill_allowed_stops_and_vehicles(model, &filters);

            let data = TransitDataFiltered::new(
                data,
                &self.allowed_stop_point_idxs,
                &self.allowed_vehicle_journey_idxs,
            );

            let responses = match (datetime_represent, comparator_type) {
                (Arrival, Loads) => {
                    let request = request::arrive_before::loads_comparator::Request::new(
                        model,
                        &data,
                        request_input,
                    )?;
                    solve_request_inner(&mut self.engine, &request, &data)
                }
                (Departure, Loads) => {
                    let request = request::depart_after::loads_comparator::Request::new(
                        model,
                        &data,
                        request_input,
                    )?;
                    solve_request_inner(&mut self.engine, &request, &data)
                }
                (Arrival, Basic) => {
                    let request = request::arrive_before::basic_comparator::Request::new(
                        model,
                        &data,
                        request_input,
                    )?;
                    solve_request_inner(&mut self.engine, &request, &data)
                }
                (Departure, Basic) => {
                    let request = request::depart_after::basic_comparator::Request::new(
                        model,
                        &data,
                        request_input,
                    )?;
                    solve_request_inner(&mut self.engine, &request, &data)
                }
            };
            Ok(responses)
        } else {
            let responses = match (datetime_represent, comparator_type) {
                (Arrival, Loads) => {
                    let request = request::arrive_before::loads_comparator::Request::new(
                        model,
                        data,
                        request_input,
                    )?;
                    solve_request_inner(&mut self.engine, &request, data)
                }
                (Departure, Loads) => {
                    let request = request::depart_after::loads_comparator::Request::new(
                        model,
                        data,
                        request_input,
                    )?;
                    solve_request_inner(&mut self.engine, &request, data)
                }
                (Arrival, Basic) => {
                    let request = request::arrive_before::basic_comparator::Request::new(
                        model,
                        data,
                        request_input,
                    )?;
                    solve_request_inner(&mut self.engine, &request, data)
                }
                (Departure, Basic) => {
                    let request = request::depart_after::basic_comparator::Request::new(
                        model,
                        data,
                        request_input,
                    )?;
                    solve_request_inner(&mut self.engine, &request, data)
                }
            };
            Ok(responses)
        }
    }
}

fn solve_request_inner<'data, 'model, Data, Request, Types>(
    engine: &mut MultiCriteriaRaptor<Types>,
    request: &Request,
    data: &'data Data,
) -> Vec<response::Response>
where
    Request: RequestWithIters,
    Request: RequestIO<'data, 'model, Data> + RequestDebug,
    Data: DataTrait,
    Types: RequestTypes<
        Position = Request::Position,
        Mission = Request::Mission,
        Stop = Request::Stop,
        Trip = Request::Trip,
        Transfer = Request::Transfer,
        Departure = Request::Departure,
        Arrival = Request::Arrival,
        Criteria = Request::Criteria,
    >,
    Types::Criteria: Debug,
{
    debug!("Start computing journeys");
    let request_timer = SystemTime::now();
    engine.compute(request);
    info!(
        "Journeys computed in {} ms with {} rounds",
        request_timer.elapsed().unwrap().as_millis(),
        engine.nb_of_rounds()
    );
    info!("Nb of journeys found : {}", engine.nb_of_journeys());
    info!("Tree size : {}", engine.tree_size());

    let journeys_iter = engine.responses().filter_map(|pt_journey| {
        request
            .create_response(pt_journey)
            .map_err(|err| {
                trace!(
                    "An error occured while converting an engine journey to response. {:?}",
                    err
                );
            })
            .ok()
    });

    let responses: Vec<_> = journeys_iter
        .map(|journey| journey.to_response(data))
        .collect();

    responses
}
