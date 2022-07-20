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

use std::{fmt::Debug, time::SystemTime};

use loki::{
    filters::Filters,
    models::ModelRefs,
    places_nearby,
    request::generic_request,
    schedule::{self, ScheduleRequestError, ScheduleRequestInput, ScheduleResponse},
    tracing::{debug, info, trace},
    DataWithIters,
};

use loki::{
    response, transit_data_filtered::FilterMemory, BadRequest, MultiCriteriaRaptor, RequestDebug,
    RequestIO, RequestInput, RequestTypes as RequestTypesTrait, RequestWithIters,
};

use super::config;
use crate::{
    datetime::DateTimeRepresent,
    loki::{DataTrait, TransitData},
    timer,
};
use loki::{
    places_nearby::{BadPlacesNearby, PlacesNearbyIter},
    request::{self, generic_request::RequestTypes},
    transit_data_filtered::TransitDataFiltered,
};

pub struct Solver {
    engine: MultiCriteriaRaptor<RequestTypes>,

    filter_memory: FilterMemory,
}

impl Solver {
    pub fn new(nb_of_stops: usize, nb_of_missions: usize) -> Self {
        Self {
            engine: MultiCriteriaRaptor::new(nb_of_stops, nb_of_missions),
            filter_memory: FilterMemory::new(),
        }
    }

    fn fill_allowed_stops_and_vehicles(&mut self, model: &ModelRefs<'_>, filters: &Filters) {
        self.filter_memory
            .fill_allowed_stops_and_vehicles(filters, model);
    }

    pub fn solve_journey_request(
        &mut self,
        data: &TransitData,
        model: &ModelRefs<'_>,
        request_input: &RequestInput,
        has_filters: Option<Filters>,
        comparator_type: &config::ComparatorType,
        datetime_represent: &DateTimeRepresent,
    ) -> Result<Vec<response::Response>, BadRequest>
    where
        Self: Sized,
    {
        if let Some(filters) = has_filters {
            self.fill_allowed_stops_and_vehicles(model, &filters);

            let filtered_data = TransitDataFiltered::new(data, &self.filter_memory);
            select_journeys_implem_and_solve(
                &mut self.engine,
                &filtered_data,
                model,
                request_input,
                comparator_type,
                datetime_represent,
            )
        } else {
            select_journeys_implem_and_solve(
                &mut self.engine,
                data,
                model,
                request_input,
                comparator_type,
                datetime_represent,
            )
        }
    }

    pub fn solve_schedule(
        &mut self,
        data: &TransitData,
        model: &ModelRefs<'_>,
        request_input: &ScheduleRequestInput,
        has_filters: Option<Filters<'_>>,
    ) -> Result<Vec<ScheduleResponse>, ScheduleRequestError>
    where
        Self: Sized,
    {
        let has_filter_memory = if let Some(filters) = has_filters {
            self.fill_allowed_stops_and_vehicles(model, &filters);
            Some(&self.filter_memory)
        } else {
            None
        };

        schedule::solve_schedule_request(request_input, data, model, has_filter_memory)
    }

    pub fn solve_places_nearby<'model>(
        &self,
        models: &ModelRefs<'model>,
        uri: &str,
        radius: f64,
    ) -> Result<PlacesNearbyIter<'model>, BadPlacesNearby> {
        places_nearby::solve_places_nearby_request(models, uri, radius)
    }
}

fn select_journeys_implem_and_solve<Data>(
    engine: &mut MultiCriteriaRaptor<RequestTypes>,
    data: &Data,
    model: &ModelRefs<'_>,
    request_input: &RequestInput,
    comparator_type: &config::ComparatorType,
    datetime_represent: &DateTimeRepresent,
) -> Result<Vec<response::Response>, BadRequest>
where
    Data: DataWithIters<
        Position = generic_request::Position,
        Mission = generic_request::Mission,
        Stop = generic_request::Stop,
        Trip = generic_request::Trip,
        Transfer = generic_request::Transfer,
    >,
{
    use crate::datetime::DateTimeRepresent::{Arrival, Departure};
    use config::ComparatorType::{Basic, Loads, Robustness};

    let responses = match (datetime_represent, comparator_type) {
        (Arrival, Loads) => {
            let request =
                request::arrive_before::loads_comparator::Request::new(model, data, request_input)?;
            solve_journeys_request_inner(engine, &request, &data)
        }
        (Departure, Loads) => {
            let request =
                request::depart_after::loads_comparator::Request::new(model, data, request_input)?;
            solve_journeys_request_inner(engine, &request, &data)
        }
        (Arrival, Basic) => {
            let request =
                request::arrive_before::basic_comparator::Request::new(model, data, request_input)?;
            solve_journeys_request_inner(engine, &request, &data)
        }
        (Departure, Basic) => {
            let request =
                request::depart_after::basic_comparator::Request::new(model, data, request_input)?;
            solve_journeys_request_inner(engine, &request, &data)
        }

        (Arrival, Robustness) => {
            let request = request::arrive_before::robustness_comparator::Request::new(
                model,
                data,
                request_input,
            )?;
            solve_journeys_request_inner(engine, &request, &data)
        }
        (Departure, Robustness) => {
            let request = request::depart_after::robustness_comparator::Request::new(
                model,
                data,
                request_input,
            )?;
            solve_journeys_request_inner(engine, &request, &data)
        }
    };

    Ok(responses)
}

fn solve_journeys_request_inner<'data, 'model, Data, Request>(
    engine: &mut MultiCriteriaRaptor<RequestTypes>,
    request: &Request,
    data: &'data Data,
) -> Vec<response::Response>
where
    Request: RequestWithIters,
    Request: RequestIO<'data, 'model, Data> + RequestDebug,
    Data: DataTrait,
    Request: RequestTypesTrait<
        Position = generic_request::Position,
        Mission = generic_request::Mission,
        Stop = generic_request::Stop,
        Trip = generic_request::Trip,
        Transfer = generic_request::Transfer,
        Departure = generic_request::Departure,
        Arrival = generic_request::Arrival,
        Criteria = generic_request::Criteria,
    >,
    Request::Criteria: Debug,
{
    debug!("Start computing journeys");
    let start_compute_time = SystemTime::now();
    engine.compute(request);
    info!(
        "Computed {} journeys in {} ms with {} rounds. Tree size : {}",
        engine.nb_of_journeys(),
        timer::duration_since(start_compute_time),
        engine.nb_of_rounds(),
        engine.tree_size(),
    );

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
