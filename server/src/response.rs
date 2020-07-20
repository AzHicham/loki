
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

use chrono::{NaiveDate, NaiveDateTime};

use failure::{bail, format_err, Error};
use std::time::SystemTime;

use std::convert::TryFrom;
use std::fmt::Write;



fn fill_protobuf_response_from_engine_result(
    journey : & Journey,
    proto_journey : & mut navitia_proto::Journey, 
    model :& transit_model::Model,
    transit_data : & TransitData,
) -> Result<(), Error>
{
    proto_journey.sections.clear();

    //proto_journey.sections.resize_with(journey.nb_of_connections() * 3, || Default::default());

    let first_section = & mut proto_journey.sections[0];
    fill_public_transport_section(&journey.first_vehicle_section(transit_data), first_section, model)?;
    
    // for connection in journey.connections(transit_data) {
    //     let mut proto_section = {
    //         let has_section = proto_journey.sections.get_mut(section_idx);
    //         if let Some(section) = has_section {
    //             section
    //         }
    //         else {
    //             proto_journey.sections.push(navitia_proto::Section::default());
    //             proto_journey.sections.insert(index, element)
    //         }
    //     };
    //     let vehicle_section = &connection.2;
    //     fill_public_transport_section(vehicle_section, proto_section, model);
    //     section_idx +=1 ;
    // }
    


    
    Ok(())
}

fn fill_public_transport_section(
    vehicle_section : & VehicleSection,
    proto : & mut navitia_proto::Section,
    model : & transit_model::Model
) -> Result<(), Error>
{
    
    
    let vehicle_journey = & model.vehicle_journeys[vehicle_section.vehicle_journey];
    let from_stoptime_idx = vehicle_section.from_stoptime_idx;
    let from_stoptime = vehicle_journey.stop_times.get(from_stoptime_idx)
        .ok_or_else( || {
            format_err!("No stoptime at idx {} for vehicle journey {}",
                vehicle_section.from_stoptime_idx,
                vehicle_journey.id
            )
        })?;
    let from_stop_point = &model.stop_points[from_stoptime.stop_point_idx];
    let to_stoptime_idx = vehicle_section.to_stoptime_idx;
    let to_stoptime = vehicle_journey.stop_times.get(to_stoptime_idx)
        .ok_or_else( || {
            format_err!("No stoptime at idx {} for vehicle journey {}",
                vehicle_section.from_stoptime_idx,
                vehicle_journey.id
            )
        })?;
    let to_stop_point = &model.stop_points[to_stoptime.stop_point_idx];

    proto.set_type(navitia_proto::SectionType::PublicTransport);
    let proto_origin = proto.origin.get_or_insert_with( || navitia_proto::PtObject::default());
    fill_stop_point_pt_object(&from_stop_point, proto_origin, model)?;
    let proto_destination = proto.destination.get_or_insert_with(|| navitia_proto::PtObject::default());
    fill_stop_point_pt_object(to_stop_point, proto_destination, model)?;
    let proto_pt_display_info = proto.pt_display_informations.get_or_insert_with(|| navitia_proto::PtDisplayInfo::default());
    fill_pt_display_info(vehicle_section.vehicle_journey, proto_pt_display_info, model)?;
    
    proto.uris = None;
    proto.vehicle_journey = None;
    
    let nb_of_stop_times = to_stoptime_idx - from_stoptime_idx + 1;
    proto.stop_date_times.resize(nb_of_stop_times, navitia_proto::StopDateTime::default());
    for (stop_time, proto_stop_date_time) in vehicle_journey.stop_times[from_stoptime_idx..=to_stoptime_idx]
                                    .iter().zip(proto.stop_date_times.iter_mut()) 
    {
        fill_stop_datetime(stop_time, &vehicle_section.day_for_vehicle_journey, proto_stop_date_time, model)?;
    }

    proto.street_network = None;
    proto.cycle_lane_length = None;
    proto.transfer_type = None;
    proto.ridesharing_journeys.clear();
    proto.ridesharing_information = None;
    proto.shape.clear();

    proto.duration = Some(duration_to_i32(&vehicle_section.from_datetime, &vehicle_section.to_datetime)?);
    proto.begin_date_time = Some(to_u64_timestamp(&vehicle_section.from_datetime)?);
    proto.end_date_time = Some(to_u64_timestamp(&vehicle_section.to_datetime)?);

    proto.base_begin_date_time = None;
    proto.base_end_date_time = None;

    proto.realtime_level = None;
    proto.length = None;

    proto.id = None;

    proto.co2_emission = None;
    proto.additional_informations.clear();

    proto.length = None;


    //proto.stop_date_times
    // proto_section.r#type = Some(navitia_proto::SectionType::PublicTransport);
    // proto_section.origin = 
    Ok(())
}

fn fill_stop_point_pt_object(
    stop_point : & StopPoint,
    proto : & mut navitia_proto::PtObject,
    model : & transit_model::Model
) -> Result<(), Error> {
    proto.name = stop_point.name.clone();
    proto.uri = stop_point.id.clone();
    proto.set_embedded_type(navitia_proto::NavitiaType::StopPoint);

    proto.stop_area = None;
    proto.poi = None;
    let proto_stop_point = proto.stop_point.get_or_insert_with(|| navitia_proto::StopPoint::default());
    fill_stop_point(stop_point, proto_stop_point, model)?;
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
    stop_point : & StopPoint,
    proto : & mut navitia_proto::StopPoint,
    model : & transit_model::Model
) -> Result<(), Error> {
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

fn fill_pt_display_info(vehicle_journey_idx : Idx<VehicleJourney>,
    proto : & mut navitia_proto::PtDisplayInfo,
    model :& transit_model::Model
) -> Result<(), Error> 
{
    let vehicle_journey = &model.vehicle_journeys[vehicle_journey_idx];
    let route_id = &vehicle_journey.route_id;
    let route = model.routes.get(route_id).ok_or_else(|| 
        format_err!("Could not find route with id {}, referenced by vehicle_journey {}",
            route_id,
            vehicle_journey.id
        )
    )?;
    let line_id = &route.line_id;
    let line = model.lines.get(line_id).ok_or_else(|| 
        format_err!("Could not find line with id {},\
                 referenced by route {},\
                 referenced by vehicle journey {}.",
                 line_id,
                 route_id,
                 vehicle_journey.id
                )
    )?;

    proto.network = Some(line.network_id.clone());
    proto.code = line.code.clone();
    proto.headsign = vehicle_journey.headsign.clone();
    proto.direction = route.destination_id.clone();
    proto.color = line.color.as_ref().map(|color| {
        format!("{}", color)
    });
    proto.commercial_mode = Some(line.commercial_mode_id.clone());
    proto.physical_mode = Some(vehicle_journey.physical_mode_id.clone());
    proto.description = None;

    let proto_uris = proto.uris.get_or_insert_with(|| navitia_proto::Uris::default());
    {
        proto_uris.company = None;
        proto_uris.vehicle_journey = Some(format!("vehicle_journey:{}", vehicle_journey.id));
        proto_uris.line = Some(format!("line:{}", line.id));
        proto_uris.route = Some(format!("route:{}", route.id));
        proto_uris.commercial_mode = Some(format!("commercial_mode:{}", line.commercial_mode_id));
        proto_uris.network = Some(format!("network:{}", line.network_id));
        proto_uris.note = None;
        proto_uris.journey_pattern = vehicle_journey.journey_pattern_id.as_ref()
            .map(|journey_pattern| {
                format!("journey_pattern:{}", journey_pattern)
            });      
    }

    proto.has_equipments = Some(navitia_proto::HasEquipments::default());
    proto.name = Some(route.name.clone());
    proto.messages.clear();
    proto.impact_uris.clear();
    proto.notes.clear();
    proto.headsigns.clear();
    proto.text_color = line.text_color.as_ref().map(|text_color| {
        format!("{}", text_color)
    });

    proto.trip_short_name = vehicle_journey.short_name.clone();


    Ok(())
}

fn fill_stop_datetime(
    stoptime : & transit_model::objects::StopTime,
    day : & NaiveDate,
    proto : & mut navitia_proto::StopDateTime,
    model :& transit_model::Model
) -> Result<(), Error>
{
    proto.arrival_date_time = Some(to_utc_timestamp(day, &stoptime.arrival_time)?);
    proto.departure_date_time = Some(to_utc_timestamp(day, &stoptime.departure_time)?);
    let proto_stop_point = proto.stop_point.get_or_insert_with(|| navitia_proto::StopPoint::default());
    let stop_point = &model.stop_points[stoptime.stop_point_idx];
    fill_stop_point(stop_point, proto_stop_point, model)?;
    proto.properties = Some(navitia_proto::Properties::default());
    proto.data_freshness = None;
    proto.departure_status = None;
    proto.arrival_status = None;
    Ok(())
    
}

fn to_utc_timestamp(day : & NaiveDate, time_in_day : & transit_model::objects::Time) -> Result<u64, Error> {
    let timestamp_i64 = day.and_hms(time_in_day.hours(), time_in_day.minutes(), time_in_day.seconds()).timestamp();
    TryFrom::try_from(timestamp_i64).map_err(|_| {
        format_err!("Unable to convert day {} time_in_day {} to u64 utc timestamp.",
            day,
            time_in_day
        )
    })
}

fn duration_to_i32(from_datetime : & NaiveDateTime, to_datetime : & NaiveDateTime) -> Result<i32, Error>
{
    let duration_i64 = (*to_datetime - *from_datetime).num_seconds();
    TryFrom::try_from(duration_i64).map_err(|_| {
        format_err!("Unable to convert duration between {} and {} to i32 seconds.",
            from_datetime,
            to_datetime
        )
    })
}

fn to_u64_timestamp(datetime : & NaiveDateTime) -> Result<u64, Error> {
    let timestamp_i64 = datetime.timestamp();
    TryFrom::try_from(timestamp_i64).map_err(|_| {
        format_err!("Unable to convert  {} to u64 utc timestamp.",
            datetime
        )
    })
}









