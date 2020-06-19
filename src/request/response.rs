
use crate::transit_data::{
    data::{
        TransitData,
        Stop,
        StopData,
        StopPattern,
        Transfer,
        Mission,
        Trip,
    },

    time::{
        SecondsSinceDatasetStart, 
    }
};

pub struct DepartureSection {
    pub from_datetime : SecondsSinceDatasetStart,
    pub to_datetime : SecondsSinceDatasetStart,
    pub to_stop : Stop,

}

pub struct VehicleSection {
    pub from_datetime : SecondsSinceDatasetStart,
    pub to_datetime : SecondsSinceDatasetStart,
    pub from_stop : Stop,
    pub to_stop : Stop,
    pub trip : Trip,
}

pub struct WaitingSection {
    pub from_datetime : SecondsSinceDatasetStart,
    pub to_datetime : SecondsSinceDatasetStart,
    pub stop : Stop,
}

pub struct TransferSection {
    pub from_datetime : SecondsSinceDatasetStart,
    pub to_datetime : SecondsSinceDatasetStart,
    pub from_stop : Stop,
    pub to_stop : Stop,
    pub transfer : Transfer,
}

pub struct ArrivalSection {
    pub from_datetime : SecondsSinceDatasetStart,
    pub to_datetime : SecondsSinceDatasetStart,
    pub from_stop : Stop,
}

pub struct Journey {
    pub departure_section : DepartureSection,
    pub first_waiting : Option<WaitingSection>,
    pub first_vehicle : VehicleSection,
    pub connections : Vec<(TransferSection, Option<WaitingSection>, VehicleSection)>,
    pub arrival_section : ArrivalSection,

}