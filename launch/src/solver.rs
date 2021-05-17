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

use loki::log::{debug, trace};

use loki::{
    response,
    traits::{self, BadRequest, RequestIO, RequestInput, RequestTypes, RequestWithIters},
    transit_model, MultiCriteriaRaptor,
};

use loki::request::basic_criteria;

use super::config;

pub trait Solver<Data> {
    fn new(nb_of_stops: usize, nb_of_missions: usize) -> Self;

    fn solve_request(
        &mut self,
        data: &Data,
        model: &transit_model::Model,
        request_input: &RequestInput,
        comparator: &config::ComparatorType,
    ) -> Result<Vec<response::Response>, BadRequest>
    where
        Self: Sized,
        Data: traits::DataWithIters;
}

pub struct BasicCriteriaSolver<Data: traits::Data> {
    engine: MultiCriteriaRaptor<basic_criteria::Types<Data>>,
}

impl<Data: traits::Data> Solver<Data> for BasicCriteriaSolver<Data> {
    fn new(nb_of_stops: usize, nb_of_missions: usize) -> Self {
        Self {
            engine: MultiCriteriaRaptor::new(nb_of_stops, nb_of_missions),
        }
    }

    fn solve_request(
        &mut self,
        data: &Data,
        model: &transit_model::Model,
        request_input: &RequestInput,
        comparator_type: &config::ComparatorType,
    ) -> Result<Vec<response::Response>, BadRequest>
    where
        Self: Sized,
        Data: traits::DataWithIters,
    {
        match comparator_type {
            config::ComparatorType::Loads => {
                let request = basic_criteria::depart_after::classic_comparator::Request::new(
                    model,
                    data,
                    request_input, 
                )?;
                let responses = solve_request_inner(&mut self.engine, &request, data);
                Ok(responses)
            }
            config::ComparatorType::Basic => {
                let request = basic_criteria::depart_after::classic_comparator::Request::new(
                    model,
                    data,
                    request_input,
                )?;
                let responses = solve_request_inner(&mut self.engine, &request, data);
                Ok(responses)
            }
        }
    }
}

use loki::request::loads_criteria;

pub struct LoadsCriteriaSolver<Data: traits::Data> {
    engine: MultiCriteriaRaptor<loads_criteria::Types<Data>>,
}

impl<Data: traits::Data> Solver<Data> for LoadsCriteriaSolver<Data> {
    fn new(nb_of_stops: usize, nb_of_missions: usize) -> Self {
        Self {
            engine: MultiCriteriaRaptor::new(nb_of_stops, nb_of_missions),
        }
    }

    fn solve_request(
        &mut self,
        data: &Data,
        model: &transit_model::Model,
        request_input: &RequestInput,
        comparator_type: &config::ComparatorType,
    ) -> Result<Vec<response::Response>, BadRequest>
    where
        Self: Sized,
        Data: traits::DataWithIters,
    {
        match comparator_type {
            config::ComparatorType::Loads => {
                let request = loads_criteria::depart_after::loads_comparator::Request::new(
                    model,
                    data,
                    request_input,
                )?;
                let responses = solve_request_inner(&mut self.engine, &request, data);
                Ok(responses)
            }
            config::ComparatorType::Basic => {
                let request = loads_criteria::depart_after::classic_comparator::Request::new(
                    model,
                    data,
                    request_input,
                )?;
                let responses = solve_request_inner(&mut self.engine, &request, data);
                Ok(responses)
            }
        }
    }
}

fn solve_request_inner<'data, Data, Request, Types>(
    engine: &mut MultiCriteriaRaptor<Types>,
    request: &Request,
    data: &'data Data,
) -> Vec<response::Response>
where
    Request: RequestWithIters,
    Request: RequestIO<'data, Data>,
    Data: traits::Data,
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
    debug!("Start computing journey");
    let request_timer = SystemTime::now();
    engine.compute(request);
    debug!(
        "Journeys computed in {} ms with {} rounds",
        request_timer.elapsed().unwrap().as_millis(),
        engine.nb_of_rounds()
    );
    debug!("Nb of journeys found : {}", engine.nb_of_journeys());
    debug!("Tree size : {}", engine.tree_size());

    let journeys_iter = engine.responses().filter_map(|pt_journey| {
        request
            .create_response(pt_journey)
            .or_else(|err| {
                trace!(
                    "An error occured while converting an engine journey to response. {:?}",
                    err
                );
                Err(err)
            })
            .ok()
    });

    let responses: Vec<_> = journeys_iter
        .map(|journey| journey.to_response(data))
        .collect();

    responses
}
