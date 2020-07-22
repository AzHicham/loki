
use crate::navitia_proto;
use laxatips::transit_model;
use laxatips::{
    TransitData,
    response:: {Journey,
        VehicleSection,
        WaitingSection,
        TransferSection,
    },
    Idx,
    StopPoint,
    VehicleJourney,
};

use chrono::{NaiveDate, NaiveDateTime};

use failure::{format_err, Error};


use std::convert::TryFrom;



pub fn fill_response<Journeys>(
    journeys : Journeys,
    proto : & mut navitia_proto::Response,
    model :& transit_model::Model,
    transit_data : & TransitData,
) -> Result<(), Error>
where Journeys : Iterator<Item = Journey> 
{
    proto.status_code = None;
    proto.error = None;
    proto.info = None;
    proto.publication_date = None;
    proto.ignored_words.clear();
    proto.bad_words.clear();
    proto.places.clear();
    proto.places_nearby.clear();
    proto.validity_patterns.clear();
    proto.lines.clear();
    proto.journey_patterns.clear();
    proto.vehicle_journeys.clear();
    proto.stop_points.clear();
    proto.stop_areas.clear();
    proto.networks.clear();
    proto.physical_modes.clear();
    proto.commercial_modes.clear();
    proto.connections.clear();
    proto.journey_pattern_points.clear();
    proto.companies.clear();
    proto.routes.clear();
    proto.pois.clear();
    proto.poi_types.clear();
    proto.calendars.clear();
    proto.line_groups.clear();
    proto.trips.clear();
    proto.contributors.clear();
    proto.datasets.clear();
    proto.route_points.clear();
    proto.impacts.clear();
    let mut nb_of_journeys = 0usize;
    for (idx, journey) in journeys.enumerate() {
        let proto_journey = if idx < proto.journeys.len() {
            & mut proto.journeys[idx]
        }
        else {
            proto.journeys.resize_with(idx + 1, || navitia_proto::Journey::default());
            & mut proto.journeys[idx]
        };
        fill_journey(&journey, proto_journey, model, transit_data)?;
        nb_of_journeys+=1;
    }
    proto.journeys.truncate(nb_of_journeys);

    proto.set_response_type(navitia_proto::ResponseType::ItineraryFound);
    proto.prev = None;
    proto.next = None;
    proto.next_request_date_time = None;
    proto.route_schedules.clear();
    proto.departure_boards.clear();
    proto.next_departures.clear();
    proto.next_arrivals.clear();
    proto.stop_schedules.clear();
    proto.load = None;
    proto.metadatas = None;
    proto.pagination = None;
    proto.traffic_reports.clear();
    proto.line_reports.clear();
    proto.tickets.clear();
    proto.pt_objects.clear();
    proto.feed_publishers.clear();
    proto.nearest_stop_points.clear();
    proto.links.clear();
    proto.graphical_isochrones.clear();
    proto.heat_maps.clear();
    proto.geo_status = None;
    proto.car_co2_emission = None;
    proto.sn_routing_matrix = None;
    proto.equipment_reports.clear();
    proto.terminus_schedules.clear();

    Ok(())
    
}

fn fill_journey(
    journey : & Journey,
    proto : & mut navitia_proto::Journey, 
    model :& transit_model::Model,
    transit_data : & TransitData,
) -> Result<(), Error>
{

    proto.duration = Some(duration_to_i32(
                &journey.departure_datetime(transit_data), 
                &journey.arrival_datetime(transit_data)
            )?
        );

    proto.nb_transfers = Some(i32::try_from(journey.nb_of_transfers())?);

    proto.departure_date_time = Some(to_u64_timestamp(&journey.departure_datetime(transit_data))?);
    proto.arrival_date_time = Some(to_u64_timestamp(&journey.arrival_datetime(transit_data))?);

    proto.requested_date_time = None;


    // we have one section for the first vehicle,
    // and then for each connection, the 3 sections : transfer, waiting, vehicle
    proto.sections.resize_with(1 + 3 * journey.nb_of_connections(), || Default::default());

    let first_section = & mut proto.sections[0];
    fill_public_transport_section(&journey.first_vehicle_section(transit_data), first_section, model)?;
    
    for (connection_idx, connection) in journey.connections(transit_data).enumerate() {
        {
            let proto_transfer_section = & mut proto.sections[1 + 3 * connection_idx];
            let transfer_section = &connection.0;
            fill_transfer_section(transfer_section, proto_transfer_section, model)?;
        }
        {
            let proto_waiting_section = & mut proto.sections[2 + 3 * connection_idx];
            let waiting_section = &connection.1;
            fill_waiting_section(waiting_section, proto_waiting_section, model)?;
        }
        {
            let proto_vehicle_section = & mut proto.sections[3 + 3 * connection_idx];
            let vehicle_section = &connection.2;
            fill_public_transport_section(vehicle_section, proto_vehicle_section, model)?;
        }
         
    }

    proto.origin = None;
    proto.destination = None;
    proto.r#type = None;
    proto.fare = None;
    proto.tags.clear();
    proto.calendars.clear();
    proto.co2_emission = None;
    proto.most_serious_disruption_effect = None;
    proto.internal_id = None;
    proto.sn_dur = Some(journey.total_fallback_duration().total_seconds());
    proto.transfer_dur = Some(journey.total_transfer_duration(transit_data).total_seconds());
    proto.min_waiting_dur = None;
    proto.nb_vj_extentions = None;
    proto.nb_sections = Some(u32::try_from(journey.nb_of_legs())?);
    proto.durations = Some( navitia_proto::Durations {
        total : Some(i32::try_from(journey.total_duration(transit_data).total_seconds())?),
        walking : Some(i32::try_from(
            (journey.total_fallback_duration() + journey.total_transfer_duration(transit_data)).total_seconds()
        )?),
        bike : Some(0),
        car : Some(0),
        ridesharing : Some(0),
        taxi : Some(0),
    }
    );
    


    
    Ok(())
}

fn fill_transfer_section(
    transfer_section : & TransferSection,
    proto : & mut navitia_proto::Section,
    model : & transit_model::Model
) -> Result<(), Error>
{

    proto.set_type(navitia_proto::SectionType::Transfer);

    let proto_origin = proto.origin.get_or_insert_with( || navitia_proto::PtObject::default());
    fill_stop_point_pt_object(transfer_section.from_stop_point, proto_origin, model)?;
    let proto_destination = proto.destination.get_or_insert_with( || navitia_proto::PtObject::default());
    fill_stop_point_pt_object(transfer_section.to_stop_point, proto_destination, model)?;

    proto.pt_display_informations = None;
    proto.uris = None;
    proto.vehicle_journey = None;
    proto.stop_date_times.clear();
    proto.street_network = None;
    proto.cycle_lane_length = None;
    proto.set_transfer_type(navitia_proto::TransferType::Walking);
    proto.ridesharing_journeys.clear();
    proto.ridesharing_information = None;

    proto.shape.clear();

    proto.duration = Some(duration_to_i32(&transfer_section.from_datetime, &transfer_section.to_datetime)?);

    proto.begin_date_time = Some(to_u64_timestamp(&transfer_section.from_datetime)?);
    proto.end_date_time = Some(to_u64_timestamp(&transfer_section.to_datetime)?);

    proto.base_begin_date_time = None;
    proto.base_end_date_time = None;
    proto.length = None;
    proto.id = None;
    proto.co2_emission = None;
    proto.additional_informations.clear();

    Ok(())
}


fn fill_waiting_section(
    waiting_section : & WaitingSection,
    proto : & mut navitia_proto::Section,
    model : & transit_model::Model
) -> Result<(), Error>
{

    proto.set_type(navitia_proto::SectionType::Waiting);

    let proto_origin = proto.origin.get_or_insert_with( || navitia_proto::PtObject::default());
    fill_stop_point_pt_object(waiting_section.stop_point, proto_origin, model)?;
    let proto_destination = proto.destination.get_or_insert_with( || navitia_proto::PtObject::default());
    fill_stop_point_pt_object(waiting_section.stop_point, proto_destination, model)?;

    proto.pt_display_informations = None;
    proto.uris = None;
    proto.vehicle_journey = None;
    proto.stop_date_times.clear();
    proto.street_network = None;
    proto.cycle_lane_length = None;
    proto.transfer_type = None;
    proto.ridesharing_journeys.clear();
    proto.ridesharing_information = None;
    proto.shape.clear();
    proto.duration = Some(duration_to_i32(&waiting_section.from_datetime, &waiting_section.to_datetime)?);
    proto.begin_date_time = Some(to_u64_timestamp(&waiting_section.from_datetime)?);
    proto.end_date_time = Some(to_u64_timestamp(&waiting_section.to_datetime)?);
    proto.begin_date_time = None;
    proto.base_end_date_time = None;
    proto.realtime_level = None;
    proto.length = None;
    proto.id = None;
    proto.co2_emission = None;
    proto.additional_informations.clear();


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
    let from_stop_point_idx = from_stoptime.stop_point_idx;
    let to_stoptime_idx = vehicle_section.to_stoptime_idx;
    let to_stoptime = vehicle_journey.stop_times.get(to_stoptime_idx)
        .ok_or_else( || {
            format_err!("No stoptime at idx {} for vehicle journey {}",
                vehicle_section.from_stoptime_idx,
                vehicle_journey.id
            )
        })?;
    let to_stop_point_idx = to_stoptime.stop_point_idx;

    proto.set_type(navitia_proto::SectionType::PublicTransport);
    let proto_origin = proto.origin.get_or_insert_with( || navitia_proto::PtObject::default());
    fill_stop_point_pt_object(from_stop_point_idx, proto_origin, model)?;
    let proto_destination = proto.destination.get_or_insert_with(|| navitia_proto::PtObject::default());
    fill_stop_point_pt_object(to_stop_point_idx, proto_destination, model)?;
    let proto_pt_display_info = proto.pt_display_informations.get_or_insert_with(|| navitia_proto::PtDisplayInfo::default());
    fill_pt_display_info(vehicle_section.vehicle_journey, proto_pt_display_info, model)?;
    
    proto.uris = None;
    proto.vehicle_journey = None;
    
    let nb_of_stop_times = to_stoptime_idx - from_stoptime_idx + 1;
    let stop_times = &vehicle_journey.stop_times[from_stoptime_idx..=to_stoptime_idx];
    proto.stop_date_times.resize(nb_of_stop_times, navitia_proto::StopDateTime::default());
    for (stop_time, proto_stop_date_time) in stop_times.iter().zip(proto.stop_date_times.iter_mut()) 
    {
        fill_stop_datetime(stop_time, &vehicle_section.day_for_vehicle_journey, proto_stop_date_time, model)?;
    }

    proto.street_network = None;
    proto.cycle_lane_length = None;
    proto.transfer_type = None;
    proto.ridesharing_journeys.clear();
    proto.ridesharing_information = None;

    proto.shape.clear();
    fill_shape_from_stop_points(
        stop_times.iter().map(|stop_time| stop_time.stop_point_idx), 
        & mut proto.shape, 
        model
    )?;

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
    stop_point_idx : Idx<StopPoint>,
    proto : & mut navitia_proto::PtObject,
    model : & transit_model::Model
) -> Result<(), Error> {

    let stop_point = & model.stop_points[stop_point_idx];

    proto.name = stop_point.name.clone();
    proto.uri = format!("stop_point:{}", stop_point.id);
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
    proto.uri = Some(format!("stop_point:{}",stop_point.id));
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
    proto.fare_zone =  Some(navitia_proto::FareZone {
            name : stop_point.fare_zone_id.clone()
        }
    );
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
    proto.uri = Some(format!("stop_area:{}", stop_area.id));
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
        proto_uris.physical_mode = Some(format!("physical_mode:{}", vehicle_journey.physical_mode_id));
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

fn fill_shape_from_stop_points(stop_points : impl Iterator<Item = Idx<StopPoint>>,
    proto : & mut Vec<navitia_proto::GeographicalCoord>,
    model : & transit_model::Model
) -> Result<(), Error>
{
    proto.clear();
    for stop_point_idx in stop_points {
        let stop_point = &model.stop_points[stop_point_idx];
        proto.push(navitia_proto::GeographicalCoord {
            lat : stop_point.coord.lat,
            lon : stop_point.coord.lon
        });
    }
    Ok(())
}









