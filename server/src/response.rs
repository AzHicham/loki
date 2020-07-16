
use crate::navitia_proto;
use prost::Message;
use laxatips::log::{debug, info, warn, trace};
use laxatips::transit_model;
use laxatips::{
    DepartAfterRequest as EngineRequest, 
    MultiCriteriaRaptor, 
    PositiveDuration, 
    TransitData,
    response:: {Journey,
        DepartureSection,
        VehicleSection,
        WaitingSection,
        TransferSection,
        ArrivalSection,
    },
    Idx,
    StopPoint,
    VehicleJourney,
    TransitModelTransfer,
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

fn fill_public_transport_section(
    vehicle_section : & VehicleSection,
    proto_section : & mut navitia_proto::Section,
    model : & transit_model::Model
) -> Result<(), Error>
{
    proto_section.set_type(navitia_proto::SectionType::PublicTransport);
    let proto_origin = proto_section.origin.get_or_insert_with( || navitia_proto::PtObject::default());
    fill_stop_point_pt_object(vehicle_section.from_stop_point, proto_origin, model)?;
    let proto_destination = proto_section.destination.get_or_insert_with(|| navitia_proto::PtObject::default());
    fill_stop_point_pt_object(vehicle_section.to_stop_point, proto_destination, model);
    
    // proto_section.r#type = Some(navitia_proto::SectionType::PublicTransport);
    // proto_section.origin = 
    Ok(())
}

fn fill_stop_point_pt_object(
    stop_point_idx : Idx<StopPoint>,
    proto : & mut navitia_proto::PtObject,
    model : & transit_model::Model
) -> Result<(), Error> {
    let stop_point = &model.stop_points[stop_point_idx];
    proto.name = stop_point.name.clone();
    proto.uri = stop_point.id.clone();
    proto.set_embedded_type(navitia_proto::NavitiaType::StopPoint);

    proto.stop_area = None;
    proto.poi = None;
    let proto_stop_point = proto.stop_point.get_or_insert_with(|| navitia_proto::StopPoint::default());
    fill_stop_point(stop_point_idx, proto_stop_point, model)?;
    proto.address = None;
    proto.line = None;
    proto.commercial_mode = None;
    proto.administrative_region = None;
    proto.distance = None;
    proto.quality = None;
    proto.company = None;
    proto.vehicle_journey = None;
    proto.calendar = None;
    proto.score = None;
    proto.trip = None;
    proto.scores.clear();
    proto.stop_points_nearby.clear();
    Ok(())
}

fn fill_stop_point(
    stop_point_idx : Idx<StopPoint>,
    proto : & mut navitia_proto::StopPoint,
    model : & transit_model::Model
) -> Result<(), Error> {
    let stop_point =  &model.stop_points[stop_point_idx];
    proto.name = Some(stop_point.name.clone());
    proto.administrative_regions.clear();
    proto.uri = Some(stop_point.id.clone());
    proto.coord = Some(navitia_proto::GeographicalCoord {
        lat : stop_point.coord.lat,
        lon : stop_point.coord.lon
    });
    let proto_stop_area = proto.stop_area.get_or_insert_with( || navitia_proto::StopArea::default());
    fill_stop_area(&stop_point.stop_area_id, proto_stop_area, model)?;
    proto.has_equipments = None;
    proto.messages.clear();
    proto.impact_uris.clear();
    proto.comments.clear();
    proto.codes.clear();
    for (key, value) in stop_point.codes.iter() {
        proto.codes.push(navitia_proto::Code{
            r#type : key.clone(),
            value : value.clone(),
        });
    }
    proto.address = None;
    proto.platform_code = stop_point.platform_code.clone();
    proto.label = None;
    proto.commercial_modes.clear();
    proto.physical_modes.clear();
    proto.fare_zone = None;
    proto.equipment_details.clear();
    
    Ok(())
}

fn fill_stop_area(
    stop_area_id : & str,
    proto : & mut navitia_proto::StopArea,
    model : & transit_model::Model
) -> Result<(), Error> {
    let stop_area = model.stop_areas.get(stop_area_id).ok_or_else( || {
        format_err!("The stop_area {} cannot be found in the model.", stop_area_id)
    })?;
    proto.name = Some(stop_area.name.clone());
    proto.uri = Some(stop_area.id.clone());
    proto.coord = Some(navitia_proto::GeographicalCoord {
        lat : stop_area.coord.lat,
        lon : stop_area.coord.lon
    });
    proto.administrative_regions.clear();
    proto.stop_points.clear();
    proto.messages.clear();
    proto.impact_uris.clear();
    proto.comments.clear();
    proto.codes.clear();
    for (key, value) in stop_area.codes.iter() {
        proto.codes.push(navitia_proto::Code{
            r#type : key.clone(),
            value : value.clone(),
        });
        
    }
    proto.timezone = stop_area.timezone.clone();
    proto.label = None;
    proto.commercial_modes.clear();
    proto.physical_modes.clear();

    Ok(())
}



