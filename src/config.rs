use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Implem {
    Periodic,
    Daily,
    LoadsPeriodic,
    LoadsDaily,
}
impl std::str::FromStr for Implem {
    type Err = ImplemConfigError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Implem::*;
        let implem = match s {
            "periodic" => Periodic,
            "daily" => Daily,
            "loads_periodic" => LoadsPeriodic,
            "loads_daily" => LoadsDaily,
            _ => Err(ImplemConfigError{ implem_name : s.to_string() })?,
        };
        Ok(implem)
    }
}

#[derive(Debug)]
pub struct ImplemConfigError {
    implem_name : String
}


impl std::fmt::Display for Implem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Implem::*;
        match self {
            Periodic => write!(f, "periodic"),
            Daily => write!(f, "daily"),
            LoadsPeriodic => write!(f, "loads_periodic"),
            LoadsDaily => write!(f, "loads_daily"),
        }
    }
}

impl std::fmt::Display for ImplemConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad implem configuration given : `{}`", self.implem_name)
    }
}




#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestType {
    Loads,
    Classic,
}
impl std::str::FromStr for RequestType {
    type Err = RequestTypeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let request_type = match s {
            "loads" => RequestType::Loads,
            "classic" => RequestType::Classic,
            _ => Err(RequestTypeError{ request_type_name : s.to_string() })?,
        };
        Ok(request_type)
    }
}

impl std::fmt::Display for RequestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestType::Loads => write!(f, "loads"),
            RequestType::Classic => write!(f, "classic"),
        }
    }
}

#[derive(Debug)]
pub struct RequestTypeError {
    request_type_name : String
}


impl std::fmt::Display for RequestTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad request_type given : `{}`", self.request_type_name)
    }
}

