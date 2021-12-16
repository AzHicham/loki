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

use launch::loki::{
    self,
    models::{ModelRefs, StopPointIdx, StopTimes, VehicleJourneyIdx},
    RealTimeLevel, RequestInput,
};

use loki::response::{TransferSection, VehicleSection, WaitingSection};

use loki::{
    chrono::{self, NaiveDate, NaiveDateTime},
    chrono_tz::{self, Tz as Timezone},
    geometry::distance_coord_to_coord,
};

use anyhow::{format_err, Context, Error};
use std::convert::TryFrom;

pub fn make_response(
    request_input: &RequestInput,
    journeys: Vec<loki::Response>,
    model: &ModelRefs<'_>,
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
    model: &ModelRefs<'_>,
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
        &request_input.real_time_level,
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
            let proto_section = make_public_transport_section(
                vehicle_section,
                model,
                section_id,
                &request_input.real_time_level,
            )?;
            proto.sections.push(proto_section);
        }
    }

    proto.co2_emission = compute_journey_co2_emission(proto.sections.as_slice());

    Ok(proto)
}

fn make_transfer_section(
    transfer_section: &TransferSection,
    model: &ModelRefs<'_>,
    section_id: String,
) -> Result<navitia_proto::Section, Error> {
    let mut proto = navitia_proto::Section {
        origin: Some(make_stop_point_pt_object(
            &transfer_section.from_stop_point,
            model,
        )?),
        destination: Some(make_stop_point_pt_object(
            &transfer_section.to_stop_point,
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
    model: &ModelRefs<'_>,
    section_id: String,
) -> Result<navitia_proto::Section, Error> {
    let mut proto = navitia_proto::Section {
        origin: Some(make_stop_point_pt_object(
            &waiting_section.stop_point,
            model,
        )?),
        destination: Some(make_stop_point_pt_object(
            &waiting_section.stop_point,
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
    model: &ModelRefs<'_>,
    section_id: String,
    real_time_level: &RealTimeLevel,
) -> Result<navitia_proto::Section, Error> {
    let vehicle_journey_idx = &vehicle_section.vehicle_journey;
    let date = &vehicle_section.day_for_vehicle_journey;
    let from_stoptime_idx = vehicle_section.from_stoptime_idx;
    let from_stop_point_idx = model
        .stop_point_at(
            vehicle_journey_idx,
            from_stoptime_idx,
            date,
            real_time_level,
        )
        .ok_or_else(|| {
            format_err!(
                "No stoptime at idx {} for vehicle journey {} on {}",
                from_stoptime_idx,
                model.vehicle_journey_name(vehicle_journey_idx),
                date
            )
        })?;

    let to_stoptime_idx = vehicle_section.to_stoptime_idx;
    let to_stop_point_idx = model
        .stop_point_at(vehicle_journey_idx, to_stoptime_idx, date, real_time_level)
        .ok_or_else(|| {
            format_err!(
                "No stoptime at idx {} for vehicle journey {} on {}",
                to_stoptime_idx,
                model.vehicle_journey_name(vehicle_journey_idx),
                date
            )
        })?;

    let stop_times = model
        .stop_times(
            vehicle_journey_idx,
            date,
            from_stoptime_idx,
            to_stoptime_idx,
            real_time_level,
        )
        .ok_or_else(|| {
            format_err!(
                "On vehicle journey {} on {} at {:?}, could not get stoptimes range [{}, {}] ",
                model.vehicle_journey_name(vehicle_journey_idx),
                date,
                real_time_level,
                from_stoptime_idx,
                to_stoptime_idx,
            )
        })?;

    let additional_informations = make_additional_informations(&stop_times);
    let shape = make_shape_from_stop_points(&stop_times, model);
    let length_f64 = compute_length_public_transport_section(shape.as_slice());
    let co2_emission = compute_section_co2_emission(length_f64, vehicle_journey_idx, model);

    let mut proto = navitia_proto::Section {
        origin: Some(make_stop_point_pt_object(&from_stop_point_idx, model)?),
        destination: Some(make_stop_point_pt_object(&to_stop_point_idx, model)?),
        pt_display_informations: Some(make_pt_display_info(
            &vehicle_section.vehicle_journey,
            *date,
            real_time_level,
            model,
        )),
        stop_date_times: make_stop_datetimes(&stop_times, model)?,
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
    stop_point_idx: &StopPointIdx,
    model: &ModelRefs<'_>,
) -> Result<navitia_proto::PtObject, Error> {
    let mut proto = navitia_proto::PtObject {
        name: model.stop_point_name(stop_point_idx).to_string(),
        uri: model.stop_point_uri(stop_point_idx),
        stop_point: Some(make_stop_point(stop_point_idx, model)),
        ..Default::default()
    };
    proto.set_embedded_type(navitia_proto::NavitiaType::StopPoint);

    Ok(proto)
}

fn make_stop_point(stop_point_idx: &StopPointIdx, model: &ModelRefs) -> navitia_proto::StopPoint {
    let has_coord = model.coord(stop_point_idx);

    let coord_proto = has_coord
        .as_ref()
        .map(|coord| navitia_proto::GeographicalCoord {
            lat: coord.lat,
            lon: coord.lon,
        });

    let has_street_name = model.street_name(stop_point_idx);

    // we create Some(navitia_proto::Address) only if we have a street name for this stop point
    let proto_address = has_street_name.map(|street_name| navitia_proto::Address {
        uri: has_coord
            .as_ref()
            .map(|coord| format!("{};{}", coord.lat, coord.lon)),
        house_number: model
            .house_numer(stop_point_idx)
            .map(|number_str| number_str.parse::<i32>().unwrap_or_default()),
        coord: coord_proto.clone(),
        name: Some(street_name.to_string()),
        label: Some(street_name.to_string()),
        ..Default::default()
    });

    let stop_point_name = model.stop_point_name(stop_point_idx);
    let proto = navitia_proto::StopPoint {
        name: Some(stop_point_name.to_string()),
        // uri: Some(stop_point.id.clone()),
        uri: Some(model.stop_point_uri(stop_point_idx)),
        coord: coord_proto,
        address: proto_address,
        label: Some(stop_point_name.to_string()),
        stop_area: make_stop_area(stop_point_idx, model),
        codes: model.codes(stop_point_idx).map_or(Vec::new(), |iter| {
            iter.map(|(key, value)| navitia_proto::Code {
                r#type: key.clone(),
                value: value.clone(),
            })
            .collect()
        }),
        platform_code: model.platform_code(stop_point_idx).map(|s| s.to_string()),
        fare_zone: model
            .fare_zone_id(stop_point_idx)
            .map(|s| navitia_proto::FareZone {
                name: Some(s.to_string()),
            }),
        ..Default::default()
    };

    proto
}

fn make_stop_area(
    stop_point_idx: &StopPointIdx,
    model: &ModelRefs,
) -> Option<navitia_proto::StopArea> {
    let stop_area_name = model.stop_area_name(stop_point_idx);
    if let Some(stop_area) = model.stop_area(stop_area_name) {
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
        return Some(proto);
    }

    None
}

fn make_pt_display_info(
    vehicle_journey_idx: &VehicleJourneyIdx,
    date: NaiveDate,
    real_time_level: &RealTimeLevel,
    model: &ModelRefs,
) -> navitia_proto::PtDisplayInfo {
    let proto = navitia_proto::PtDisplayInfo {
        network: Some(model.network_name(vehicle_journey_idx).to_string()),
        code: model.line_code(vehicle_journey_idx).map(|s| s.to_string()),
        headsign: model
            .headsign(vehicle_journey_idx, &date, real_time_level)
            .map(|s| s.to_string()),
        direction: model
            .direction(vehicle_journey_idx, &date)
            .map(|s| s.to_string()),
        color: model
            .line_color(vehicle_journey_idx, &date)
            .map(|color| format!("{}", color)),
        commercial_mode: Some(model.commercial_mode_name(vehicle_journey_idx).to_string()),
        physical_mode: Some(model.physical_mode_name(vehicle_journey_idx).to_string()),
        uris: Some(navitia_proto::Uris {
            vehicle_journey: Some(format!(
                "vehicle_journey:{}",
                model.vehicle_journey_name(vehicle_journey_idx)
            )),
            line: Some(format!("line:{}", model.line_name(vehicle_journey_idx))),
            route: Some(format!("route:{}", model.route_name(vehicle_journey_idx))),
            commercial_mode: Some(format!(
                "commercial_mode:{}",
                model.commercial_mode_name(vehicle_journey_idx)
            )),
            physical_mode: Some(format!(
                "physical_mode:{}",
                model.physical_mode_name(vehicle_journey_idx)
            )),
            network: Some(format!(
                "network:{}",
                model.network_name(vehicle_journey_idx)
            )),
            ..Default::default()
        }),
        name: Some(model.line_name(vehicle_journey_idx).to_string()),
        text_color: model
            .text_color(vehicle_journey_idx, &date)
            .map(|text_color| format!("{}", text_color)),
        trip_short_name: model
            .trip_short_name(vehicle_journey_idx, &date)
            .map(|s| s.to_string()),
        ..Default::default()
    };

    proto
}

fn make_stop_datetimes(
    stop_times: &StopTimes,
    model: &ModelRefs,
) -> Result<Vec<navitia_proto::StopDateTime>, Error> {
    let mut result = Vec::new();
    match stop_times {
        StopTimes::Base(stop_times, date, timezone) => {
            for stop_time in stop_times.iter() {
                let arrival_seconds = i64::from(stop_time.arrival_time.total_seconds());
                let arrival = to_utc_timestamp(*timezone, *date, arrival_seconds)?;
                let departure_seconds = i64::from(stop_time.departure_time.total_seconds());
                let departure = to_utc_timestamp(*timezone, *date, departure_seconds)?;
                let stop_point_idx = StopPointIdx::Base(stop_time.stop_point_idx);
                let proto = navitia_proto::StopDateTime {
                    arrival_date_time: Some(arrival),
                    departure_date_time: Some(departure),
                    stop_point: Some(make_stop_point(&stop_point_idx, model)),
                    ..Default::default()
                };
                result.push(proto);
            }
        }
        StopTimes::New(stop_times, date) => {
            for stop_time in stop_times.iter() {
                let arrival_seconds = i64::from(stop_time.arrival_time.total_seconds());
                let arrival = to_utc_timestamp(chrono_tz::UTC, *date, arrival_seconds)?;
                let depature_seconds = i64::from(stop_time.departure_time.total_seconds());
                let departure = to_utc_timestamp(chrono_tz::UTC, *date, depature_seconds)?;
                let proto = navitia_proto::StopDateTime {
                    arrival_date_time: Some(arrival),
                    departure_date_time: Some(departure),
                    stop_point: Some(make_stop_point(&stop_time.stop, model)),
                    ..Default::default()
                };
                result.push(proto);
            }
        }
    }
    Ok(result)
}

fn make_additional_informations(
    stop_times: &StopTimes,
    /*stop_points: &Vec<StopPoint>,*/
) -> Vec<i32> {
    match stop_times {
        StopTimes::Base(stop_times, _, _) => {
            let mut result = Vec::new();

            let st_is_empty = stop_times.is_empty();
            let has_datetime_estimated = !st_is_empty
                && (stop_times.first().unwrap().datetime_estimated
                    || stop_times.last().unwrap().datetime_estimated);
            let has_odt = false;
            let is_zonal = false;

            if has_datetime_estimated {
                result.push(
                    navitia_proto::SectionAdditionalInformationType::HasDatetimeEstimated as i32,
                );
            }
            if is_zonal {
                result.push(navitia_proto::SectionAdditionalInformationType::OdtWithZone as i32);
            } else if has_odt && has_datetime_estimated {
                result
                    .push(navitia_proto::SectionAdditionalInformationType::OdtWithStopPoint as i32);
            } else if has_odt {
                result
                    .push(navitia_proto::SectionAdditionalInformationType::OdtWithStopTime as i32);
            }
            if result.is_empty() {
                result.push(navitia_proto::SectionAdditionalInformationType::Regular as i32);
            }

            result
        }
        StopTimes::New(_, _) => Vec::new(),
    }
}

fn to_utc_timestamp(
    timezone: Timezone,
    day: NaiveDate,
    time_in_day: i64, //nb of seconds since local day start
) -> Result<u64, Error> {
    use chrono::TimeZone;
    let local_datetime = day.and_hms(0, 0, 0) + chrono::Duration::seconds(time_in_day);
    let timezoned_datetime = timezone
        .from_local_datetime(&local_datetime)
        .earliest()
        .unwrap();
    let timestamp_i64 = timezoned_datetime.timestamp();
    TryFrom::try_from(timestamp_i64).with_context(|| {
        format!(
            "Unable to convert day {} time_in_day {} to u64 utc timestamp.",
            day, time_in_day
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
    vehicle_journey_idx: &VehicleJourneyIdx,
    model: &ModelRefs,
) -> Option<navitia_proto::Co2Emission> {
    let physical_mode_id = model.physical_mode_name(vehicle_journey_idx);

    model
        .base
        .physical_modes
        .get(physical_mode_id)
        .and_then(|physical_mode| physical_mode.co2_emission)
        .map(|co2_emission| navitia_proto::Co2Emission {
            unit: Some("gEC".to_string()),
            value: Some(f64::from(co2_emission) * length * 1e-3_f64),
        })
}

fn make_calendars(_model: &ModelRefs<'_>) -> Vec<navitia_proto::Calendar> {
    let proto_calendar: Vec<navitia_proto::Calendar> = Vec::new();
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

fn make_feed_publishers(model: &ModelRefs<'_>) -> Vec<navitia_proto::FeedPublisher> {
    model
        .base
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
    TryFrom::try_from(duration_i64).with_context(|| {
        format!(
            "Unable to convert duration between {} and {} to i32 seconds.",
            from_datetime, to_datetime
        )
    })
}

fn to_u64_timestamp(datetime: &NaiveDateTime) -> Result<u64, Error> {
    let timestamp_i64 = datetime.timestamp();
    TryFrom::try_from(timestamp_i64)
        .with_context(|| format!("Unable to convert  {} to u64 utc timestamp.", datetime))
}

fn make_shape_from_stop_points(
    stoptimes: &StopTimes,
    model: &ModelRefs,
) -> Vec<navitia_proto::GeographicalCoord> {
    match stoptimes {
        StopTimes::Base(base_stop_times, _, _) => base_stop_times
            .iter()
            .map(|stop_time| {
                let stop_point_idx = stop_time.stop_point_idx;
                let stop_point = &model.base.stop_points[stop_point_idx];
                navitia_proto::GeographicalCoord {
                    lat: stop_point.coord.lat,
                    lon: stop_point.coord.lon,
                }
            })
            .collect(),
        StopTimes::New(_, _) => Vec::new(),
    }
}
