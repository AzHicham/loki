
#[derive(Debug)]
pub enum Implem {
    Periodic,
    Daily,
    LoadsPeriodic,
    LoadsDaily,
}
impl std::str::FromStr for Implem {
    type Err = ImplConfigError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Implem::*;
        let implem = match s {
            "periodic" => Periodic,
            "daily" => Daily,
            "loads_periodic" => LoadsPeriodic,
            "loads_daily" => LoadsDaily,
            _ => Err(ImplConfigError{ implem_name : s.to_string() })?,
        };
        Ok(implem)
    }
}
#[derive(Debug)]
pub struct ImplConfigError {
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

impl std::fmt::Display for ImplConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad implem configuration given : `{}`", self.implem_name)
    }
}

