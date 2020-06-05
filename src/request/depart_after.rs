
use crate::transit_data::{
    data::{
        EngineData,
        TransitData,
        Stop,
        StopIdx,
        StopPatternIdx,
        Position,
    },
    depart_after_queries::{
        ForwardMission,
        ForwardTrip,
        
    },
    time::{
        SecondsSinceDatasetStart, 
        PositiveDuration,
        DaysSinceDatasetStart,
    }
};

use crate::engine::public_transit::PublicTransit;

use typed_index_collection::{Idx};
use transit_model::{
    objects::{StopPoint,},
}; 

pub struct Request<'a> {
    transit_data : & 'a TransitData,
    departure_datetime : SecondsSinceDatasetStart,
    departures_stop_point_and_fallback_duration : Vec<(Idx<StopPoint>, PositiveDuration)>,
    arrivals_stop_point_and_fallbrack_duration : Vec<(Idx<StopPoint>, PositiveDuration)>

}

impl<'a> Request<'a> {

    pub fn new(transit_data : & 'a TransitData,
        departure_datetime : SecondsSinceDatasetStart,
        departures_stop_point_and_fallback_duration : Vec<(Idx<StopPoint>, PositiveDuration)>,
        arrivals_stop_point_and_fallbrack_duration : Vec<(Idx<StopPoint>, PositiveDuration)>
    ) -> Self
    {
        Self {
            transit_data,
            departure_datetime,
            departures_stop_point_and_fallback_duration,
            arrivals_stop_point_and_fallbrack_duration,
        }
    }

}




#[derive(Debug, PartialEq, Eq, Clone)]
struct Criteria {
    arrival_time : SecondsSinceDatasetStart,
    nb_of_transfers : usize,
    fallback_duration : PositiveDuration,
    transfers_duration : PositiveDuration,
}


// impl<'a> PublicTransit for Request<'a> {
//     type Stop = StopIdx;
//     type Mission = ForwardMission;
//     type Trip = ForwardTrip;
//     type Criteria = Criteria;

//     fn is_upstream(&self, upstream : & Self::Stop, downstream : & Self::Stop, mission : & Self::Mission) -> bool {
//         self.transit_data.engine_data.is_upstream_in_forward_mission(upstream, downstream, mission)
//     }

//     fn next_on_mission(&self, stop : & Self::Stop, mission : & Self::Mission) -> Option<Self::Stop> {
//         self.transit_data.engine_data.next_stop_in_forward_mission(stop, mission)
//     }

//     type Missions = ForwardMissionsIter;
//     fn boardable_missions_of(&self, stop : & Self::Stop) -> Self::Missions {
//         self.transit_data.engine_data
//             .boardable_forward_missions(stop)
            
//     }

//     fn mission_of(&self, trip : & Self::Trip) -> Self::Mission {
//         self.transit_data.engine_data.forward_mission_of(trip)
//     }

// }