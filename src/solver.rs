use std::{fmt::Debug, time::SystemTime};

use log::{debug, trace};

use crate::{MultiCriteriaRaptor, PositiveDuration, config, response, traits::{self, BadRequest, RequestIO, RequestInput, RequestTypes, RequestWithIters, Solver}};


use crate::request::basic_criteria;

pub struct BasicCriteriaSolver<'data, Data : traits::Data> {
    engine : MultiCriteriaRaptor<basic_criteria::Types<'data, Data>>,
    
}

impl<'data, Data : traits::Data > Solver<'data, Data> for BasicCriteriaSolver<'data, Data> 
{
    fn new(nb_of_stops: usize, nb_of_missions: usize) -> Self {
        Self {
            engine : MultiCriteriaRaptor::new(nb_of_stops, nb_of_missions)
        }
    }

    fn solve_request<Departures, Arrivals, D, A>(
        & mut self,
        data : & 'data Data,
        model : & transit_model::Model,
        request_input : RequestInput<Departures, Arrivals, D, A>,
        comparator_type : & config::ComparatorType,
    ) -> Result<Vec<response::Response>, BadRequest>
    where Self : Sized,
    Arrivals : Iterator<Item = (A, PositiveDuration)>,
    Departures : Iterator<Item = (D, PositiveDuration)>,
    A : AsRef<str>,
    D : AsRef<str>,
    Data : traits::DataWithIters,
    {
        match comparator_type {
            config::ComparatorType::Loads => {
                let request = basic_criteria::depart_after::classic_comparator::Request::new(model, data, request_input)?;
                let responses = solve_request_inner(& mut self.engine, &request, data, model);
                Ok(responses)
            }
            config::ComparatorType::Basic => {
                let request = basic_criteria::depart_after::classic_comparator::Request::new(model, data, request_input)?;
                let responses = solve_request_inner(& mut self.engine, &request, data, model);
                Ok(responses)
            }
        }
    }
}



use crate::request::loads_criteria;

pub struct LoadsCriteriaSolver<'data, Data : traits::Data> {
    engine : MultiCriteriaRaptor<loads_criteria::Types<'data, Data>>,
    
}

impl<'data, Data : traits::Data > Solver<'data, Data> for LoadsCriteriaSolver<'data, Data> 
{
    fn new(nb_of_stops: usize, nb_of_missions: usize) -> Self {
        Self {
            engine : MultiCriteriaRaptor::new(nb_of_stops, nb_of_missions)
        }
    }

    fn solve_request<Departures, Arrivals, D, A>(
        & mut self,
        data : & 'data Data,
        model : & transit_model::Model,
        request_input : RequestInput<Departures, Arrivals, D, A>,
        comparator_type : & config::ComparatorType,
    ) -> Result<Vec<response::Response>, BadRequest>
    where Self : Sized,
    Arrivals : Iterator<Item = (A, PositiveDuration)>,
    Departures : Iterator<Item = (D, PositiveDuration)>,
    A : AsRef<str>,
    D : AsRef<str>,
    Data : traits::DataWithIters,
    {
        match comparator_type {
            config::ComparatorType::Loads => {
                let request = loads_criteria::depart_after::loads_comparator::Request::new(model, data, request_input)?;
                let responses = solve_request_inner(& mut self.engine, &request, data, model);
                Ok(responses)
            }
            config::ComparatorType::Basic => {
                let request = loads_criteria::depart_after::classic_comparator::Request::new(model, data, request_input)?;
                let responses = solve_request_inner(& mut self.engine, &request, data, model);
                Ok(responses)
            }
        }
    }
}




fn solve_request_inner<'data, Data, Request, Types>(
    engine : & mut MultiCriteriaRaptor<Types>, 
    request : & Request,
    data : & 'data Data,
    model : &transit_model::Model
) -> Vec<response::Response>
where 
Request : RequestWithIters,
Request : RequestIO<'data, Data>,
Data : traits::Data,
Types : RequestTypes<
    Position = Request::Position,
    Mission = Request::Mission,
    Stop = Request::Stop,
    Trip = Request::Trip,
    Transfer = Request::Transfer,
    Departure = Request::Departure,
    Arrival = Request::Arrival,
    Criteria = Request::Criteria,
>,
Types::Criteria : Debug,
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


    for pt_journey in engine.responses() {
        let response = request.create_response(pt_journey);
        match response {
            Ok(journey) => {
                trace!("{}", journey.print(data, model).unwrap());
            }
            Err(_) => {
                trace!("An error occured while converting an engine journey to response.");
            }
        };
    }

    let journeys_iter = engine
        .responses()
        .filter_map(|pt_journey| request.create_response(pt_journey).ok());

    let responses : Vec<_> = journeys_iter.map(|journey| journey.to_response(data)).collect();

    responses
}
