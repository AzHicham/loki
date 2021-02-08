use laxatips::{response, transit_model::Model, log::info};

use laxatips::{
     DepartAfter, LoadsDepartAfter, MultiCriteriaRaptor,
};

use laxatips::traits;

use std::{fmt::Debug,};
use traits::{RequestIO, RequestWithIters};

use failure::{bail, Error};
use std::time::SystemTime;

use structopt::StructOpt;

use crate::{parse_datetime, parse_duration, solve, build, BaseOptions};


#[derive(StructOpt, Debug)]
#[structopt(rename_all = "snake_case")]
pub struct Options {
   
    #[structopt(flatten)]
    pub base : BaseOptions,

    #[structopt(long)]
    pub start: String,

    #[structopt(long)]
    pub end: String,

}




pub fn launch<Data>(options: Options) -> Result<Vec<response::Journey<Data>>, Error>
where
    Data: traits::DataWithIters,
{
    let (data, model) = build(&options.base.ntfs_path, &options.base.loads_data_path)?;
    match options.base.request_type.as_str() {
        "classic" => build_engine_and_solve::<Data, DepartAfter<Data>>(&model, &data, &options),
        "loads" => build_engine_and_solve::<Data, LoadsDepartAfter<Data>>(&model, &data, &options),
        _ => {
            bail!("Invalid request_type : {}", options.base.request_type)
        }
    }
}


fn build_engine_and_solve<'data, Data, R>(
    model: &Model,
    data: &'data Data,
    options: &Options,
) -> Result< Vec<response::Journey<Data>>, Error>
where
    R: RequestWithIters + RequestIO<'data, Data>,
    Data: traits::DataWithIters<
        Position = R::Position,
        Mission = R::Mission,
        Stop = R::Stop,
        Trip = R::Trip,
    >,
    R::Criteria : Debug
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

    let leg_arrival_penalty = parse_duration(&request_config.leg_arrival_penalty).unwrap();
    let leg_walking_penalty = parse_duration(&request_config.leg_walking_penalty).unwrap();
    let max_journey_duration = parse_duration(&request_config.max_journey_duration).unwrap();
    let max_nb_of_legs: u8 = request_config.max_nb_of_legs;
    let mut raptor = MultiCriteriaRaptor::<R>::new(nb_of_stops, nb_of_missions);

    let compute_timer = SystemTime::now();

    let start_stop_area_uri = &options.start;
    let end_stop_area_uri = &options.end;

    let responses = solve(
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

    Ok(responses)
}
