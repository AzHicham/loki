use crate::navitia_proto;
use laxatips::transit_model;
use laxatips::{
    LaxatipsData,
    response::{Journey, TransferSection, VehicleSection, WaitingSection},
    Idx, StopPoint, VehicleJourney,
};

use chrono::{NaiveDate, NaiveDateTime};
use chrono_tz::Tz as Timezone;

use failure::{format_err, Error};

use std::convert::TryFrom;

pub fn make_response<Journeys>(
    journeys: Journeys,
    laxatips_data : & LaxatipsData, 
) -> Result<navitia_proto::Response, Error>
where
    Journeys: Iterator<Item = Journey>,
{
    let mut proto = navitia_proto::Response {
        journeys : journeys.enumerate()
                        .map(|(idx, journey) | make_journey(&journey, laxatips_data, idx))
                        .collect::<Result<Vec<_>, _>>()?,

        ..Default::default()
    };
    

    proto.set_response_type(navitia_proto::ResponseType::ItineraryFound);

    Ok(proto)
}

fn make_journey(
    journey: &Journey,

    laxatips_data : & LaxatipsData,
    journey_id: usize,
) -> Result<navitia_proto::Journey, Error> {
    let transit_data = & laxatips_data.transit_data;
    let model = & laxatips_data.model;

    // we have one section for the first vehicle,
    // and then for each connection, the 3 sections : transfer, waiting, vehicle
    let nb_of_sections = 1 + 3 * journey.nb_of_connections();

    let mut proto = navitia_proto::Journey {
        duration : Some(i32::try_from(
            journey.total_duration_in_pt(transit_data).total_seconds(),
        )?),
        nb_transfers : Some(i32::try_from(journey.nb_of_transfers())?),
        departure_date_time : Some(to_u64_timestamp(&journey.first_vehicle_board_datetime(transit_data))?),
        arrival_date_time : Some(to_u64_timestamp(&journey.last_vehicle_debark_datetime(transit_data))?),
        sections : Vec::with_capacity(nb_of_sections), // to be filled below
        sn_dur : Some(journey.total_fallback_duration().total_seconds()),
        transfer_dur : Some(journey
                                .total_transfer_duration(transit_data)
                                .total_seconds(),
                        ),
        nb_sections : Some(u32::try_from(journey.nb_of_legs())?),
        durations :  Some(navitia_proto::Durations {
            total: Some(i32::try_from(
                journey.total_duration_in_pt(transit_data).total_seconds(),
            )?),
            walking: Some(i32::try_from(
                (
                    //journey.total_fallback_duration() +
                    journey.total_transfer_duration(transit_data)
                )
                    .total_seconds(),
            )?),
            bike: Some(0),
            car: Some(0),
            ridesharing: Some(0),
            taxi: Some(0),
        }),
        ..Default::default()
    };

    let section_id = format!("section_{}_{}", journey_id, 0);
    proto.sections.push(make_public_transport_section(&journey.first_vehicle_section(transit_data), model, section_id)?);

    for (connection_idx, connection) in journey.connections(transit_data).enumerate() {
        {
            let section_id = format!("section_{}_{}", journey_id, 1 + 3 * connection_idx);
            let transfer_section = &connection.0;
            let proto_section = make_transfer_section(transfer_section, model, section_id)?;
            proto.sections.push(proto_section);
        }
        {
            let section_id = format!("section_{}_{}", journey_id, 2 + 3 * connection_idx);
            let waiting_section = &connection.1;
            let proto_section = make_waiting_section(waiting_section, model, section_id)?;
            proto.sections.push(proto_section);
        }
        {
            let section_id = format!("section_{}_{}", journey_id, 3 + 3 * connection_idx);
            let vehicle_section = &connection.2;
            let proto_section = make_public_transport_section(
                vehicle_section,
                model,
                section_id,
            )?;
            proto.sections.push(proto_section);
        }
    }

    Ok(proto)
}

fn make_transfer_section(
    transfer_section: &TransferSection,
    model: &transit_model::Model,
    section_id: String,
) -> Result<navitia_proto::Section, Error> {

    let mut proto = navitia_proto::Section {
        origin : Some(make_stop_point_pt_object(transfer_section.from_stop_point, model)?),
        destination : Some(make_stop_point_pt_object(transfer_section.to_stop_point, model)?),
        duration : Some(duration_to_i32(
                        &transfer_section.from_datetime,
                        &transfer_section.to_datetime,
                    )?),
        begin_date_time : Some(to_u64_timestamp(&transfer_section.from_datetime)?),
        end_date_time : Some(to_u64_timestamp(&transfer_section.to_datetime)?),
        id : Some(section_id),
        ..Default::default()
    };
    proto.set_type(navitia_proto::SectionType::Transfer);

    proto.set_transfer_type(navitia_proto::TransferType::Walking);

    Ok(proto)
}

fn make_waiting_section(
    waiting_section: &WaitingSection,
    model: &transit_model::Model,
    section_id: String,
) -> Result<navitia_proto::Section, Error> {

    let mut proto = navitia_proto::Section {
        origin : Some(make_stop_point_pt_object(waiting_section.stop_point, model)?),
        destination : Some(make_stop_point_pt_object(waiting_section.stop_point, model)?),
        duration : Some(duration_to_i32(
                        &waiting_section.from_datetime,
                        &waiting_section.to_datetime,
                    )?),
        begin_date_time : Some(to_u64_timestamp(&waiting_section.from_datetime)?),
        end_date_time : Some(to_u64_timestamp(&waiting_section.to_datetime)?),
        id : Some(section_id),
        ..Default::default()
    };

    proto.set_type(navitia_proto::SectionType::Waiting);

    Ok(proto)
}

fn make_public_transport_section(
    vehicle_section: &VehicleSection,
    model: &transit_model::Model,
    section_id: String,
) -> Result<navitia_proto::Section, Error> {
    let vehicle_journey = &model.vehicle_journeys[vehicle_section.vehicle_journey];
    let from_stoptime_idx = vehicle_section.from_stoptime_idx;
    let from_stoptime = vehicle_journey
        .stop_times
        .get(from_stoptime_idx)
        .ok_or_else(|| {
            format_err!(
                "No stoptime at idx {} for vehicle journey {}",
                vehicle_section.from_stoptime_idx,
                vehicle_journey.id
            )
        })?;
    let from_stop_point_idx = from_stoptime.stop_point_idx;
    let to_stoptime_idx = vehicle_section.to_stoptime_idx;
    let to_stoptime = vehicle_journey
        .stop_times
        .get(to_stoptime_idx)
        .ok_or_else(|| {
            format_err!(
                "No stoptime at idx {} for vehicle journey {}",
                vehicle_section.from_stoptime_idx,
                vehicle_journey.id
            )
        })?;
    let to_stop_point_idx = to_stoptime.stop_point_idx;
    let stop_times = &vehicle_journey.stop_times[from_stoptime_idx..=to_stoptime_idx];
    let day = &vehicle_section.day_for_vehicle_journey;
    let timezone = get_timezone(&vehicle_section.vehicle_journey, model).unwrap_or(chrono_tz::UTC);

    let mut proto = navitia_proto::Section {
        origin : Some(make_stop_point_pt_object(from_stop_point_idx, model)?),
        destination : Some(make_stop_point_pt_object(to_stop_point_idx, model)?),
        pt_display_informations : Some(make_pt_display_info(vehicle_section.vehicle_journey, model)?),
        stop_date_times : stop_times
                            .iter()
                            .map(|stop_time| 
                                make_stop_datetime(stop_time, day, &timezone, model)
                            ).collect::<Result<Vec<_>,_>>()?,
        shape : make_shape_from_stop_points(stop_times.iter().map(|stop_time| stop_time.stop_point_idx), model),
        duration : Some(duration_to_i32(
                        &vehicle_section.from_datetime,
                        &vehicle_section.to_datetime,
                    )?),
        begin_date_time : Some(to_u64_timestamp(&vehicle_section.from_datetime)?),
        end_date_time : Some(to_u64_timestamp(&vehicle_section.to_datetime)?),
        id : Some(section_id),
        ..Default::default()

    };

    proto.set_type(navitia_proto::SectionType::PublicTransport);

    Ok(proto)
}

fn make_stop_point_pt_object(
    stop_point_idx: Idx<StopPoint>,
    model: &transit_model::Model,
) -> Result<navitia_proto::PtObject, Error> {
    let stop_point = &model.stop_points[stop_point_idx];

    let mut proto = navitia_proto::PtObject {
        name : stop_point.name.clone(),
        uri : format!("stop_point:{}", stop_point.id),
        stop_point : Some(make_stop_point(stop_point, model)?),
        ..Default::default()
    };
    proto.set_embedded_type(navitia_proto::NavitiaType::StopPoint);

    Ok(proto)
}

fn make_stop_point(
    stop_point: &StopPoint,
    model: &transit_model::Model,
) -> Result<navitia_proto::StopPoint, Error> {

    let proto = navitia_proto::StopPoint {
        name : Some(stop_point.name.clone()),
        uri : Some(format!("stop_point:{}", stop_point.id)),
        coord : Some(navitia_proto::GeographicalCoord {
                    lat: stop_point.coord.lat,
                    lon: stop_point.coord.lon,
                }),
        stop_area : Some(make_stop_area(&stop_point.stop_area_id, model)?),
        codes : stop_point.codes.iter().map(|(key, value)| 
                navitia_proto::Code {
                    r#type: key.clone(),
                    value: value.clone(),
                }).collect(), 
        platform_code : stop_point.platform_code.clone(),    
        fare_zone : Some( navitia_proto::FareZone {
                        name : stop_point.fare_zone_id.clone()
                    }),
        ..Default::default()

    };

    Ok(proto)
}

fn make_stop_area(
    stop_area_id: &str,
    model: &transit_model::Model,
) -> Result<navitia_proto::StopArea, Error> {
    let stop_area = model.stop_areas.get(stop_area_id).ok_or_else(|| {
        format_err!(
            "The stop_area {} cannot be found in the model.",
            stop_area_id
        )
    })?;
    let proto = navitia_proto::StopArea {
        name : Some(stop_area.name.clone()),
        uri : Some(format!("stop_area:{}", stop_area.id)),
        coord : Some(navitia_proto::GeographicalCoord {
            lat: stop_area.coord.lat,
            lon: stop_area.coord.lon,
        }),
        codes : stop_area.codes.iter().map(|(key, value)| 
            navitia_proto::Code {
                r#type: key.clone(),
                value: value.clone(),
            }).collect(),       
        timezone : stop_area.timezone.clone(),
        ..Default::default()
    };

    Ok(proto)
}

fn make_pt_display_info(
    vehicle_journey_idx: Idx<VehicleJourney>,
    model: &transit_model::Model,
) -> Result<navitia_proto::PtDisplayInfo, Error> {
    let vehicle_journey = &model.vehicle_journeys[vehicle_journey_idx];
    let route_id = &vehicle_journey.route_id;
    let route = model.routes.get(route_id).ok_or_else(|| {
        format_err!(
            "Could not find route with id {}, referenced by vehicle_journey {}",
            route_id,
            vehicle_journey.id
        )
    })?;
    let line_id = &route.line_id;
    let line = model.lines.get(line_id).ok_or_else(|| {
        format_err!(
            "Could not find line with id {},\
                 referenced by route {},\
                 referenced by vehicle journey {}.",
            line_id,
            route_id,
            vehicle_journey.id
        )
    })?;

    let proto = navitia_proto::PtDisplayInfo {
        network : Some(line.network_id.clone()),
        code : line.code.clone(),
        headsign : vehicle_journey.headsign.clone(),
        direction : route.destination_id.clone(),
        color : line.color.as_ref().map(|color| format!("{}", color)),
        commercial_mode : Some(line.commercial_mode_id.clone()),
        physical_mode : Some(vehicle_journey.physical_mode_id.clone()),
        uris : Some(navitia_proto::Uris {
            vehicle_journey : Some(format!("vehicle_journey:{}", vehicle_journey.id)),
            line : Some(format!("line:{}", line.id)),
            route : Some(format!("route:{}", route.id)),
            commercial_mode : Some(format!("commercial_mode:{}", line.commercial_mode_id)),
            physical_mode : Some(format!(
                "physical_mode:{}",
                vehicle_journey.physical_mode_id
            )),
            network : Some(format!("network:{}", line.network_id)),
            journey_pattern : vehicle_journey
                .journey_pattern_id
                .as_ref()
                .map(|journey_pattern| format!("journey_pattern:{}", journey_pattern)),
            ..Default::default()
        }),
        name : Some(route.name.clone()),
        text_color : line.text_color.as_ref().map(|text_color| format!("{}", text_color)),
        trip_short_name : vehicle_journey.short_name.clone(),
        ..Default::default()
    };

    Ok(proto)
}

fn make_stop_datetime(
    stoptime: &transit_model::objects::StopTime,
    day: &NaiveDate,
    timezone : & Timezone,
    model: &transit_model::Model,
) -> Result<navitia_proto::StopDateTime, Error> {
    let stop_point = &model.stop_points[stoptime.stop_point_idx];
    let proto = navitia_proto::StopDateTime {
        arrival_date_time : Some(to_utc_timestamp(timezone, day, &stoptime.arrival_time)?),
        departure_date_time : Some(to_utc_timestamp(timezone, day, &stoptime.departure_time)?),
        stop_point : Some(make_stop_point(stop_point, model)?),
        ..Default::default()
    };
    Ok(proto)
}

fn to_utc_timestamp(
    timezone : & Timezone,
    day: &NaiveDate,
    time_in_day: &transit_model::objects::Time,
) -> Result<u64, Error> {
    use chrono::TimeZone;
    let local_datetime = day.and_hms(0,0,0) + chrono::Duration::seconds(time_in_day.total_seconds() as i64);
    let timezoned_datetime = timezone.from_local_datetime(&local_datetime).earliest().unwrap();
    let timestamp_i64 = timezoned_datetime.timestamp();
    TryFrom::try_from(timestamp_i64).map_err(|_| {
        format_err!(
            "Unable to convert day {} time_in_day {} to u64 utc timestamp.",
            day,
            time_in_day
        )
    })
}

fn duration_to_i32(
    from_datetime: &NaiveDateTime,
    to_datetime: &NaiveDateTime,
) -> Result<i32, Error> {
    let duration_i64 = (*to_datetime - *from_datetime).num_seconds();
    TryFrom::try_from(duration_i64).map_err(|_| {
        format_err!(
            "Unable to convert duration between {} and {} to i32 seconds.",
            from_datetime,
            to_datetime
        )
    })
}

fn to_u64_timestamp(datetime: &NaiveDateTime) -> Result<u64, Error> {
    let timestamp_i64 = datetime.timestamp();
    TryFrom::try_from(timestamp_i64)
        .map_err(|_| format_err!("Unable to convert  {} to u64 utc timestamp.", datetime))
}

fn make_shape_from_stop_points(
    stop_points: impl Iterator<Item = Idx<StopPoint>>,
    model: &transit_model::Model,
) -> Vec<navitia_proto::GeographicalCoord> {
    stop_points.map(|stop_point_idx| {
        let stop_point = &model.stop_points[stop_point_idx];
        navitia_proto::GeographicalCoord {
            lat: stop_point.coord.lat,
            lon: stop_point.coord.lon,
        }
    })
    .collect()

}

fn get_timezone(vehicle_journey_idx: & Idx<VehicleJourney>, model: &transit_model::Model) -> Option<Timezone> {
    let route_id = &model.vehicle_journeys[*vehicle_journey_idx].route_id;
    let route = model.routes.get(&route_id)?;
    let line = model.lines.get(&route.line_id)?;
    let network = model.networks.get(&line.network_id)?;
    let timezone_string = &network.timezone.as_ref()?;
    let has_timezone : Result<Timezone, _> = timezone_string.parse();
    has_timezone.ok()
}
