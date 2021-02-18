use laxatips::{log::info, response, transit_model::Model};

use laxatips::{DepartAfter, LoadsDepartAfter, MultiCriteriaRaptor};

use laxatips::{traits, config};
use log::trace;

use std::fmt::{Debug, Display};
use traits::{RequestIO, RequestWithIters};

use failure::{Error};
use std::time::SystemTime;

use structopt::StructOpt;

use crate::{build, parse_datetime, solve, BaseOptions};

#[derive(StructOpt, Debug)]
#[structopt(
    name = "laxatips_stop_areas",
    about = "Perform a public transport request between two stop areas.",
    rename_all = "snake_case"
)]
pub struct Options {
    #[structopt(flatten)]
    pub base: BaseOptions,

    #[structopt(long)]
    pub start: String,

    #[structopt(long)]
    pub end: String,
}

impl Display for Options {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "laxatips_cli {} --start {} --end {}",
            self.base.to_string(),
            self.start,
            self.end
        )
    }
}

pub fn launch<Data>(options: Options) -> Result<(Model, Vec<response::Response>), Error>
where
    Data: traits::DataWithIters,
{
    let (data, model) = build(&options.base.ntfs_path, &options.base.loads_data_path)?;
    let responses = match options.base.request_type {
        config::RequestType::Classic => build_engine_and_solve::<Data, DepartAfter<Data>>(&model, &data, &options)?,
        config::RequestType::Loads => build_engine_and_solve::<Data, LoadsDepartAfter<Data>>(&model, &data, &options)?,
    };
    Ok((model, responses))
}

fn build_engine_and_solve<'data, Data, R>(
    model: &Model,
    data: &'data Data,
    options: &Options,
) -> Result<Vec<response::Response>, Error>
where
    R: RequestWithIters + RequestIO<'data, Data>,
    Data: traits::DataWithIters<
        Position = R::Position,
        Mission = R::Mission,
        Stop = R::Stop,
        Trip = R::Trip,
    >,
    R::Criteria: Debug,
{
    let nb_of_stops = data.nb_of_stops();
    let nb_of_missions = data.nb_of_missions();

    let departure_datetime = match &options.base.departure_datetime {
        Some(string_datetime) => parse_datetime(&string_datetime)?,
        None => {
            let naive_date = data.calendar().first_date();
            naive_date.and_hms(8, 0, 0)
        }
    };

    let request_config = &options.base.request_config;

    let leg_arrival_penalty = &request_config.leg_arrival_penalty;
    let leg_walking_penalty = &request_config.leg_walking_penalty;
    let max_journey_duration = &request_config.max_journey_duration;
    let max_nb_of_legs: u8 = request_config.max_nb_of_legs;
    let mut raptor = MultiCriteriaRaptor::<R>::new(nb_of_stops, nb_of_missions);

    let compute_timer = SystemTime::now();

    let start_stop_area_uri = &options.start;
    let end_stop_area_uri = &options.end;

    let journeys = solve(
        start_stop_area_uri,
        end_stop_area_uri,
        &mut raptor,
        model,
        data,
        &departure_datetime,
        &leg_arrival_penalty,
        &leg_walking_penalty,
        &max_journey_duration,
        max_nb_of_legs,
    )?;

    let duration = compute_timer.elapsed().unwrap().as_millis();
    info!("Duration : {} ms", duration as f64);

    for journey in journeys.iter() {
        trace!("{}", journey.print(data, model)?);
    }

    let responses = journeys
        .into_iter()
        .map(|journey| journey.to_response(data))
        .collect();

    Ok(responses)
}
