// Copyright  (C) 2020, Kisio Digital and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Kisio Digital (www.kisio.com).
// Help us simplify mobility and open public transport:
// a non ending quest to the responsive locomotion way of traveling!
//
// This contribution is a part of the research and development work of the
// IVA Project which aims to enhance traveler information and is carried out
// under the leadership of the Technological Research Institute SystemX,
// with the partnership and support of the transport organization authority
// Ile-De-France Mobilités (IDFM), SNCF, and public funds
// under the scope of the French Program "Investissements d’Avenir".
//
// LICENCE: This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// Stay tuned using
// twitter @navitia
// channel `#navitia` on riot https://riot.im/app/#/room/#navitia:matrix.org
// https://groups.google.com/d/forum/navitia
// www.navitia.io

use crate::navitia_proto;

use launch::loki::{self, RequestInput};

use loki::transit_model;
use loki::{
    response::{TransferSection, VehicleSection, WaitingSection},
    Idx, StopPoint, VehicleJourney,
};

use loki::chrono::{self, NaiveDate, NaiveDateTime};
use loki::chrono_tz::{self, Tz as Timezone};

use failure::{format_err, Error};
use transit_model::Model;

use launch::loki::transit_model::objects::StopTime;
use std::convert::TryFrom;

const N_DEG_TO_RAD: f64 = 0.01745329238;
const EARTH_RADIUS_IN_METERS: f64 = 6372797.560856;

pub fn make_response(
    request_input: &RequestInput,
    journeys: Vec<loki::Response>,
    model: &Model,
) -> Result<navitia_proto::Response, Error> {
    let mut proto = navitia_proto::Response {
        journeys: journeys
            .iter()
            .enumerate()
            .map(|(idx, journey)| make_journey(request_input, journey, idx, model))
            .collect::<Result<Vec<_>, _>>()?,
        feed_publishers: make_feed_publishers(model),
        ..Default::default()
    };

    proto.set_response_type(navitia_proto::ResponseType::ItineraryFound);

    Ok(proto)
}

fn make_journey(
    request_input: &RequestInput,
    journey: &loki::Response,
    journey_id: usize,
    model: &Model,
) -> Result<navitia_proto::Journey, Error> {
    // we have one section for the first vehicle,
    // and then for each connection, the 3 sections : transfer, waiting, vehicle
    let nb_of_sections = journey.nb_of_sections();

    let mut proto = navitia_proto::Journey {
        calendars: make_calendars(model),
        duration: Some(i32::try_from(journey.total_duration())?),
        nb_transfers: Some(i32::try_from(journey.nb_of_transfers())?),
        departure_date_time: Some(to_u64_timestamp(&journey.first_vehicle_board_datetime())?),
        arrival_date_time: Some(to_u64_timestamp(&journey.last_vehicle_debark_datetime())?),
        sections: Vec::with_capacity(nb_of_sections), // to be filled below
        sn_dur: Some(u64::try_from(journey.total_fallback_duration())?),
        transfer_dur: Some(u64::try_from(journey.total_transfer_duration())?),
        nb_sections: Some(u32::try_from(journey.nb_of_sections())?),
        durations: Some(navitia_proto::Durations {
            total: Some(i32::try_from(journey.total_duration())?),
            walking: Some(i32::try_from(journey.total_transfer_duration())?),
            bike: Some(0),
            car: Some(0),
            ridesharing: Some(0),
            taxi: Some(0),
        }),
        requested_date_time: Some(to_u64_timestamp(&request_input.datetime)?),
        ..Default::default()
    };

    let section_id = format!("section_{}_{}", journey_id, 0);
    proto.sections.push(make_public_transport_section(
        &journey.first_vehicle,
        model,
        section_id,
    )?);

    for (connection_idx, connection) in journey.connections.iter().enumerate() {
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
            let proto_section = make_public_transport_section(vehicle_section, model, section_id)?;
            proto.sections.push(proto_section);
        }
    }

    proto.co2_emission = compute_journey_co2_emission(proto.sections.as_slice());

    Ok(proto)
}

fn make_transfer_section(
    transfer_section: &TransferSection,
    model: &transit_model::Model,
    section_id: String,
) -> Result<navitia_proto::Section, Error> {
    let mut proto = navitia_proto::Section {
        origin: Some(make_stop_point_pt_object(
            transfer_section.from_stop_point,
            model,
        )?),
        destination: Some(make_stop_point_pt_object(
            transfer_section.to_stop_point,
            model,
        )?),
        duration: Some(duration_to_i32(
            &transfer_section.from_datetime,
            &transfer_section.to_datetime,
        )?),
        begin_date_time: Some(to_u64_timestamp(&transfer_section.from_datetime)?),
        end_date_time: Some(to_u64_timestamp(&transfer_section.to_datetime)?),
        id: Some(section_id),
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
        origin: Some(make_stop_point_pt_object(
            waiting_section.stop_point,
            model,
        )?),
        destination: Some(make_stop_point_pt_object(
            waiting_section.stop_point,
            model,
        )?),
        duration: Some(duration_to_i32(
            &waiting_section.from_datetime,
            &waiting_section.to_datetime,
        )?),
        begin_date_time: Some(to_u64_timestamp(&waiting_section.from_datetime)?),
        end_date_time: Some(to_u64_timestamp(&waiting_section.to_datetime)?),
        id: Some(section_id),
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
    let additional_informations = make_additional_informations(stop_times);
    let shape = make_shape_from_stop_points(
        stop_times.iter().map(|stop_time| stop_time.stop_point_idx),
        model,
    );
    let length_f64 = compute_length_public_transport_section(shape.as_slice());
    let co2_emission =
        compute_section_co2_emission(length_f64, &vehicle_section.vehicle_journey, model);

    let mut proto = navitia_proto::Section {
        origin: Some(make_stop_point_pt_object(from_stop_point_idx, model)?),
        destination: Some(make_stop_point_pt_object(to_stop_point_idx, model)?),
        pt_display_informations: Some(make_pt_display_info(
            vehicle_section.vehicle_journey,
            model,
        )?),
        stop_date_times: stop_times
            .iter()
            .map(|stop_time| make_stop_datetime(stop_time, day, &timezone, model))
            .collect::<Result<Vec<_>, _>>()?,
        shape,
        length: Some(length_f64 as i32),
        co2_emission,
        duration: Some(duration_to_i32(
            &vehicle_section.from_datetime,
            &vehicle_section.to_datetime,
        )?),
        begin_date_time: Some(to_u64_timestamp(&vehicle_section.from_datetime)?),
        end_date_time: Some(to_u64_timestamp(&vehicle_section.to_datetime)?),
        id: Some(section_id),
        additional_informations,
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
    // let trimmed = stop_point.id.trim_start_matches("StopPoint:");
    // let stop_point_uri = format!("stop_point:{}", trimmed);
    // let stop_point_uri = stop_point.id.clone();
    let stop_point_uri = format!("stop_point:{}", stop_point.id);
    let mut proto = navitia_proto::PtObject {
        name: stop_point.name.clone(),
        uri: stop_point_uri,
        stop_point: Some(make_stop_point(stop_point, model)?),
        ..Default::default()
    };
    proto.set_embedded_type(navitia_proto::NavitiaType::StopPoint);

    Ok(proto)
}

fn make_stop_point(
    stop_point: &StopPoint,
    model: &transit_model::Model,
) -> Result<navitia_proto::StopPoint, Error> {
    let proto_address = stop_point
        .address_id
        .as_ref()
        .and_then(|address_id| model.addresses.get(address_id.as_str()))
        .map(|address| navitia_proto::Address {
            uri: Some(format!("{};{}", stop_point.coord.lat, stop_point.coord.lon)),
            house_number: Some(address.house_number.as_ref().map_or(0, |str_number| {
                str_number.parse::<i32>().unwrap_or_default()
            })),
            coord: Some(navitia_proto::GeographicalCoord {
                lat: stop_point.coord.lat,
                lon: stop_point.coord.lon,
            }),
            name: Some(address.street_name.clone()),
            label: Some(address.street_name.clone()),
            ..Default::default()
        });

    let proto = navitia_proto::StopPoint {
        name: Some(stop_point.name.clone()),
        // uri: Some(stop_point.id.clone()),
        uri: Some(format!("stop_point:{}", stop_point.id)),
        coord: Some(navitia_proto::GeographicalCoord {
            lat: stop_point.coord.lat,
            lon: stop_point.coord.lon,
        }),
        address: proto_address,
        label: Some(stop_point.name.clone()),
        stop_area: Some(make_stop_area(&stop_point.stop_area_id, model)?),
        codes: stop_point
            .codes
            .iter()
            .map(|(key, value)| navitia_proto::Code {
                r#type: key.clone(),
                value: value.clone(),
            })
            .collect(),
        platform_code: stop_point.platform_code.clone(),
        fare_zone: Some(navitia_proto::FareZone {
            name: stop_point.fare_zone_id.clone(),
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
        name: Some(stop_area.name.clone()),
        uri: Some(format!("stop_area:{}", stop_area.id)),
        coord: Some(navitia_proto::GeographicalCoord {
            lat: stop_area.coord.lat,
            lon: stop_area.coord.lon,
        }),
        label: Some(stop_area.name.clone()),
        codes: stop_area
            .codes
            .iter()
            .map(|(key, value)| navitia_proto::Code {
                r#type: key.clone(),
                value: value.clone(),
            })
            .collect(),
        timezone: stop_area
            .timezone
            .or(Some(chrono_tz::UTC))
            .map(|timezone| timezone.to_string()),
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
    let network_id = &line.network_id;
    let network = model.networks.get(network_id).ok_or_else(|| {
        format_err!(
            "Could not find network with id {},\
                 referenced by line {},\
                 referenced by vehicle journey {}.",
            network_id,
            line_id,
            vehicle_journey.id
        )
    })?;

    let destination_sa_name = route
        .destination_id
        .as_ref()
        .and_then(|destination_id| model.stop_areas.get(destination_id.as_str()))
        .map(|stop_area| stop_area.name.clone());

    let proto = navitia_proto::PtDisplayInfo {
        network: Some(network.name.clone()),
        code: line.code.clone(),
        headsign: vehicle_journey.headsign.clone(),
        direction: destination_sa_name,
        color: line.color.as_ref().map(|color| format!("{}", color)),
        commercial_mode: Some(line.commercial_mode_id.clone()),
        physical_mode: Some(vehicle_journey.physical_mode_id.clone()),
        uris: Some(navitia_proto::Uris {
            vehicle_journey: Some(format!("vehicle_journey:{}", vehicle_journey.id)),
            line: Some(format!("line:{}", line.id)),
            route: Some(format!("route:{}", route.id)),
            commercial_mode: Some(format!("commercial_mode:{}", line.commercial_mode_id)),
            physical_mode: Some(format!(
                "physical_mode:{}",
                vehicle_journey.physical_mode_id
            )),
            network: Some(format!("network:{}", line.network_id)),
            journey_pattern: vehicle_journey
                .journey_pattern_id
                .as_ref()
                .map(|journey_pattern| format!("journey_pattern:{}", journey_pattern)),
            ..Default::default()
        }),
        name: Some(line.name.clone()),
        text_color: line
            .text_color
            .as_ref()
            .map(|text_color| format!("{}", text_color)),
        trip_short_name: vehicle_journey
            .short_name
            .clone()
            .or_else(|| vehicle_journey.headsign.clone()),
        ..Default::default()
    };

    Ok(proto)
}

fn make_stop_datetime(
    stoptime: &StopTime,
    day: &NaiveDate,
    timezone: &Timezone,
    model: &transit_model::Model,
) -> Result<navitia_proto::StopDateTime, Error> {
    let stop_point = &model.stop_points[stoptime.stop_point_idx];
    let proto = navitia_proto::StopDateTime {
        arrival_date_time: Some(to_utc_timestamp(timezone, day, &stoptime.arrival_time)?),
        departure_date_time: Some(to_utc_timestamp(timezone, day, &stoptime.departure_time)?),
        stop_point: Some(make_stop_point(stop_point, model)?),
        ..Default::default()
    };
    Ok(proto)
}

fn make_additional_informations(
    stop_times: &[StopTime],
    /*stop_points: &Vec<StopPoint>,*/
) -> Vec<i32> {
    let mut result = Vec::new();

    let st_is_empty = stop_times.is_empty();
    let has_datetime_estimated = !st_is_empty
        && (stop_times.first().unwrap().datetime_estimated
            || stop_times.last().unwrap().datetime_estimated);
    let has_odt = false;
    let is_zonal = false;

    if has_datetime_estimated {
        result.push(navitia_proto::SectionAdditionalInformationType::HasDatetimeEstimated as i32);
    }
    if is_zonal {
        result.push(navitia_proto::SectionAdditionalInformationType::OdtWithZone as i32);
    } else if has_odt && has_datetime_estimated {
        result.push(navitia_proto::SectionAdditionalInformationType::OdtWithStopPoint as i32);
    } else if has_odt {
        result.push(navitia_proto::SectionAdditionalInformationType::OdtWithStopTime as i32);
    }
    if result.is_empty() {
        result.push(navitia_proto::SectionAdditionalInformationType::Regular as i32);
    }

    result
}

fn to_utc_timestamp(
    timezone: &Timezone,
    day: &NaiveDate,
    time_in_day: &transit_model::objects::Time,
) -> Result<u64, Error> {
    use chrono::TimeZone;
    let local_datetime =
        day.and_hms(0, 0, 0) + chrono::Duration::seconds(time_in_day.total_seconds() as i64);
    let timezoned_datetime = timezone
        .from_local_datetime(&local_datetime)
        .earliest()
        .unwrap();
    let timestamp_i64 = timezoned_datetime.timestamp();
    TryFrom::try_from(timestamp_i64).map_err(|_| {
        format_err!(
            "Unable to convert day {} time_in_day {} to u64 utc timestamp.",
            day,
            time_in_day
        )
    })
}

fn compute_length_public_transport_section(shape: &[navitia_proto::GeographicalCoord]) -> f64 {
    if shape.len() > 1 {
        let from_iter = shape.iter();
        let to_iter = shape.iter().skip(1);
        let shape_iter = from_iter.zip(to_iter);
        shape_iter.fold(0.0, |acc, from_to| {
            acc + distance_coord_to_coord(from_to.0, from_to.1)
        })
    } else {
        0.0
    }
}

fn compute_section_co2_emission(
    length: f64,
    vehicle_journey_idx: &Idx<VehicleJourney>,
    model: &Model,
) -> Option<navitia_proto::Co2Emission> {
    let vehicle_journey = &model.vehicle_journeys[*vehicle_journey_idx];
    let physical_mode_str = &vehicle_journey.physical_mode_id;

    model
        .physical_modes
        .get(physical_mode_str)
        .and_then(|physical_mode| physical_mode.co2_emission)
        .map(|co2_emission| navitia_proto::Co2Emission {
            unit: Some("gEC".to_string()),
            value: Some(co2_emission as f64 * length * 1e-3_f64),
        })
}

fn make_calendars(model: &Model) -> Vec<navitia_proto::Calendar> {
    let mut proto_calendar: Vec<navitia_proto::Calendar> = Vec::new();
    for calendar in &model.calendars {
        let dates = &calendar.1.dates;
        let active_periods = navitia_proto::CalendarPeriod {
            begin: Some(
                dates
                    .iter()
                    .next()
                    .map_or("".to_string(), |date| date.format("%Y%m%d").to_string()),
            ),
            end: Some(
                dates
                    .iter()
                    .next_back()
                    .map_or("".to_string(), |date| date.format("%Y%m%d").to_string()),
            ),
        };
        let week_pattern = navitia_proto::WeekPattern {
            monday: Some(false),
            tuesday: Some(false),
            wednesday: Some(false),
            thursday: Some(false),
            friday: Some(false),
            saturday: Some(false),
            sunday: Some(false),
        };
        let calendar = navitia_proto::Calendar {
            active_periods: vec![active_periods],
            week_pattern: Some(week_pattern),
            uri: Some(calendar.1.id.clone()),
            ..Default::default()
        };
        proto_calendar.push(calendar);
    }
    proto_calendar
}

fn compute_journey_co2_emission(
    sections: &[navitia_proto::Section],
) -> Option<navitia_proto::Co2Emission> {
    let total_co2 = sections
        .iter()
        .map(|section| &section.co2_emission)
        .filter_map(|co2_emission| co2_emission.as_ref())
        .filter_map(|co2| co2.value)
        .fold(0_f64, |acc, value| acc + value);

    Some(navitia_proto::Co2Emission {
        unit: Some("gEC".to_string()),
        value: Some(total_co2),
    })
}

fn make_feed_publishers(model: &Model) -> Vec<navitia_proto::FeedPublisher> {
    model
        .contributors
        .iter()
        .map(|id_contributor| {
            let contributor = id_contributor.1;
            navitia_proto::FeedPublisher {
                id: contributor.id.clone(),
                name: Some(contributor.name.clone()),
                license: contributor.license.clone(),
                url: contributor.website.clone(),
            }
        })
        .collect()
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
    stop_points
        .map(|stop_point_idx| {
            let stop_point = &model.stop_points[stop_point_idx];
            navitia_proto::GeographicalCoord {
                lat: stop_point.coord.lat,
                lon: stop_point.coord.lon,
            }
        })
        .collect()
}

fn get_timezone(
    vehicle_journey_idx: &Idx<VehicleJourney>,
    model: &transit_model::Model,
) -> Option<Timezone> {
    let route_id = &model.vehicle_journeys[*vehicle_journey_idx].route_id;
    let route = model.routes.get(route_id)?;
    let line = model.lines.get(&route.line_id)?;
    let network = model.networks.get(&line.network_id)?;
    network.timezone
}

fn distance_coord_to_coord(
    from: &navitia_proto::GeographicalCoord,
    to: &navitia_proto::GeographicalCoord,
) -> f64 {
    let longitude_arc = (from.lon - to.lon) * N_DEG_TO_RAD;
    let latitude_arc = (from.lat - to.lat) * N_DEG_TO_RAD;
    let latitude_h = (latitude_arc * 0.5).sin();
    let latitude_h = latitude_h * latitude_h;
    let longitude_h = (longitude_arc * 0.5).sin();
    let longitude_h = longitude_h * longitude_h;
    let tmp = (from.lat * N_DEG_TO_RAD).cos() * (to.lat * N_DEG_TO_RAD).cos();
    EARTH_RADIUS_IN_METERS * 2.0 * (latitude_h + tmp * longitude_h).sqrt().asin()
}
