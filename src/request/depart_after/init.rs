use crate::laxatips_data::{
    LaxatipsData,
    time::{PositiveDuration},
};

use chrono::NaiveDateTime;
use log::{warn};

use super::Request;

#[derive(Debug)]
pub enum BadRequest {
    DepartureDatetime, 
    NoValidDepartureStop,
    NoValidArrivalStop,
}

use std::fmt;

impl fmt::Display for BadRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      match self {
        BadRequest::DepartureDatetime => write!(f, "The requested datetime is out of the validity period of the data."),
        BadRequest::NoValidDepartureStop => write!(f, "No valid departure stop among the provided ones."),
        BadRequest::NoValidArrivalStop => write!(f, "No valid arrival stop among the provided ones."),
    }
  }
}

impl std::error::Error for BadRequest {}

impl<'data> Request<'data> {

    pub fn new<'a, 'b>(
        laxatips_data : & 'data LaxatipsData,
        departure_datetime: NaiveDateTime,
        departures_stop_point_and_fallback_duration: impl Iterator<Item=(&'a str, PositiveDuration)>,
        arrivals_stop_point_and_fallback_duration: impl Iterator<Item=(&'b str, PositiveDuration)>,
        leg_arrival_penalty: PositiveDuration,
        leg_walking_penalty: PositiveDuration,
        max_duration_to_arrival: PositiveDuration,
        max_nb_legs: u8,
    ) ->  Result<Self, BadRequest>
    {
        let transit_data = &laxatips_data.transit_data;
        let model = &laxatips_data.model;

        let departure_datetime = transit_data
            .calendar
            .naive_datetime_to_seconds_since_start(&departure_datetime)
            .ok_or_else(|| {
                warn!("The departure datetime {} is out of bound of the allowed dates. \
                    Allowed dates are between {} and {}.",
                        departure_datetime,
                        transit_data.calendar.first_date(),
                        transit_data.calendar.last_date(),
                );
                BadRequest::DepartureDatetime
            })?;
        

        

        let departures : Vec<_> = departures_stop_point_and_fallback_duration
            .enumerate()
            .filter_map(|(idx, (stop_point_uri, fallback_duration))| {
                let stop_idx = model.stop_points.get_idx(stop_point_uri).or_else(|| {
                    warn!(
                        "The {}th departure stop point {} is not found in model. \
                                I ignore it.",
                        idx, stop_point_uri
                    );
                    None
                })?;
                let stop = transit_data.stop_point_idx_to_stop(&stop_idx).or_else(|| {
                    warn!(
                        "The {}th departure stop point {} with idx {:?} is not found in transit_data. \
                            I ignore it",
                        idx, stop_point_uri, stop_idx
                    );
                    None
                })?;
                Some((stop.clone(), fallback_duration))
            })
            .collect();
        if departures.is_empty() {
            return Err(BadRequest::NoValidDepartureStop);
        }
        
        let arrivals : Vec<_> = arrivals_stop_point_and_fallback_duration
            .enumerate()
            .filter_map(|(idx, (stop_point_uri, fallback_duration))| {
                let stop_idx = model.stop_points.get_idx(stop_point_uri).or_else(|| {
                    warn!(
                        "The {}th arrival stop point {} is not found in model. \
                                I ignore it.",
                        idx, stop_point_uri
                    );
                    None
                })?;
                let stop = transit_data.stop_point_idx_to_stop(&stop_idx).or_else(|| {
                    warn!(
                        "The {}th arrival stop point {} with idx {:?} is not found in transit_data. \
                            I ignore it",
                        idx, stop_point_uri, stop_idx
                    );
                    None
                })?;
                Some((stop.clone(), fallback_duration))
            })
            .collect();
        
        if arrivals.is_empty() {
            return Err(BadRequest::NoValidArrivalStop);
        }


        let result =Self {
            laxatips_data,
            departure_datetime,
            departures_stop_point_and_fallback_duration : departures,
            arrivals_stop_point_and_fallbrack_duration : arrivals,
            leg_arrival_penalty,
            leg_walking_penalty,
            max_arrival_time : departure_datetime + max_duration_to_arrival,
            max_nb_legs
        };

        Ok(result)
    }




    
}
