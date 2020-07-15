
use crate::navitia_proto;
use prost::Message;
use laxatips::log::{debug, info, warn, trace};
use laxatips::transit_model;
use laxatips::{
    DepartAfterRequest as EngineRequest, 
    MultiCriteriaRaptor, 
    PositiveDuration, 
    TransitData,
    Journey,
    DepartureSection,
    VehicleSection,
    WaitingSection,
    TransferSection,
    ArrivalSection,

};



use failure::{bail, format_err, Error};
use std::time::SystemTime;

use std::convert::TryFrom;




fn fill_protobuf_response_from_engine_result(
    journey : & Journey,
    proto_journey : & mut navitia_proto::Journey, 
    model :& transit_model::Model,
    transit_data : & TransitData,
) -> Result<(), Error>
{
    
    Ok(())
}
