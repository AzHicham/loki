use std::{fmt::Display, str::FromStr};

use serde::Deserialize;

use crate::PositiveDuration;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataImplem {
    Periodic,
    Daily,
    LoadsPeriodic,
    LoadsDaily,
}
impl std::str::FromStr for DataImplem {
    type Err = DataImplemConfigError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use DataImplem::*;
        let implem = match s {
            "periodic" => Periodic,
            "daily" => Daily,
            "loads_periodic" => LoadsPeriodic,
            "loads_daily" => LoadsDaily,
            _ => Err(DataImplemConfigError{ implem_name : s.to_string() })?,
        };
        Ok(implem)
    }
}

#[derive(Debug)]
pub struct DataImplemConfigError {
    implem_name : String
}


impl std::fmt::Display for DataImplem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use DataImplem::*;
        match self {
            Periodic => write!(f, "periodic"),
            Daily => write!(f, "daily"),
            LoadsPeriodic => write!(f, "loads_periodic"),
            LoadsDaily => write!(f, "loads_daily"),
        }
    }
}

impl std::fmt::Display for DataImplemConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad implem configuration given : `{}`", self.implem_name)
    }
}




#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriteriaImplem {
    Loads,
    Basic,
}
impl std::str::FromStr for CriteriaImplem {
    type Err = CriteriaImplemConfigError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let request_type = match s {
            "loads" => CriteriaImplem::Loads,
            "classic" => CriteriaImplem::Basic,
            _ => Err(CriteriaImplemConfigError{ criteria_implem_name : s.to_string() })?,
        };
        Ok(request_type)
    }
}

impl std::fmt::Display for CriteriaImplem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CriteriaImplem::Loads => write!(f, "loads"),
            CriteriaImplem::Basic => write!(f, "basic"),
        }
    }
}

#[derive(Debug)]
pub struct CriteriaImplemConfigError {
    criteria_implem_name : String
}


impl std::fmt::Display for CriteriaImplemConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad criteria_implem given : `{}`", self.criteria_implem_name)
    }
}





#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestType {
    LoadsDepartAfter,
    BasicDepartAfter,
}
impl std::str::FromStr for RequestType {
    type Err = RequestTypeConfigError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let request_type = match s {
            "loads" => RequestType::LoadsDepartAfter,
            "classic" => RequestType::BasicDepartAfter,
            _ => Err(RequestTypeConfigError{ request_type_name : s.to_string() })?,
        };
        Ok(request_type)
    }
}

impl std::fmt::Display for RequestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestType::LoadsDepartAfter => write!(f, "loads"),
            RequestType::BasicDepartAfter => write!(f, "basic"),
        }
    }
}

#[derive(Debug)]
pub struct RequestTypeConfigError {
    request_type_name : String
}


impl std::fmt::Display for RequestTypeConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad request type given : `{}`", self.request_type_name)
    }
}


pub const DEFAULT_LEG_ARRIVAL_PENALTY: &str = "00:02:00";
pub const DEFAULT_LEG_WALKING_PENALTY: &str = "00:02:00";
pub const DEFAULT_MAX_NB_LEGS: &str = "10";
pub const DEFAULT_MAX_JOURNEY_DURATION: &str = "24:00:00";

pub struct RequestParams {
    /// penalty to apply to arrival time for each vehicle leg in a journey
    pub leg_arrival_penalty: PositiveDuration,

    /// penalty to apply to walking time for each vehicle leg in a journey
    pub leg_walking_penalty: PositiveDuration,

    /// maximum number of vehicle legs in a journey
    pub max_nb_of_legs: u8,

    /// maximum duration of a journey
    pub max_journey_duration: PositiveDuration,
}

impl Default for RequestParams {
    fn default() -> Self {
        let max_nb_of_legs: u8 = FromStr::from_str(DEFAULT_MAX_NB_LEGS).unwrap();
        Self {
            leg_arrival_penalty: FromStr::from_str(DEFAULT_LEG_ARRIVAL_PENALTY).unwrap(),
            leg_walking_penalty: FromStr::from_str(DEFAULT_LEG_WALKING_PENALTY).unwrap(),
            max_nb_of_legs,
            max_journey_duration: FromStr::from_str(DEFAULT_MAX_JOURNEY_DURATION).unwrap(),
        }
    }
}

impl Display for RequestParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "--leg_arrival_penalty {} --leg_walking_penalty {} --max_nb_of_legs {} --max_journey_duration {}",
                self.leg_arrival_penalty,
                self.leg_walking_penalty,
                self.max_nb_of_legs,
                self.max_journey_duration
        )
    }
}
