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
    models::{Coord, ModelRefs, StopPointIdx, StopTimes, Timezone, VehicleJourneyIdx},
    RealTimeLevel, RequestInput,
};

use loki::response::{TransferSection, VehicleSection, WaitingSection};

use loki::{
    chrono::{self, NaiveDate, NaiveDateTime},
    geometry::distance_coord_to_coord,
};

use anyhow::{format_err, Context, Error};
use launch::loki::{
    chrono::Timelike,
    models::real_time_disruption::{
        ApplicationPattern, ChannelType, DateTimePeriod, Disruption, DisruptionProperty, Effect,
        Impact, Impacted, Informed, LineSectionDisruption, Message, RailSectionDisruption,
        Severity, TimeSlot,
    },
    transit_model::objects::{Line, Network, Route, StopArea},
};
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
                "No stoptime at idx {:?} for vehicle journey {} on {}",
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
                "No stoptime at idx {:?} for vehicle journey {} on {}",
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
                "On vehicle journey {} on {} at {:?}, could not get stoptimes range [{:?}, {:?}] ",
                model.vehicle_journey_name(vehicle_journey_idx),
                date,
                real_time_level,
                from_stoptime_idx,
                to_stoptime_idx,
            )
        })?;

    let additional_informations =
        make_additional_informations(vehicle_section, real_time_level, model);
    let shape = make_shape_from_stop_points(stop_times.clone(), model);
    let length_f64 = compute_length_public_transport_section(shape.as_slice());
    let co2_emission = compute_section_co2_emission(length_f64, vehicle_journey_idx, model);

    let timezone = model.timezone(vehicle_journey_idx, date);

    let mut proto = navitia_proto::Section {
        origin: Some(make_stop_point_pt_object(&from_stop_point_idx, model)?),
        destination: Some(make_stop_point_pt_object(&to_stop_point_idx, model)?),
        pt_display_informations: Some(make_pt_display_info(
            &vehicle_section.vehicle_journey,
            *date,
            real_time_level,
            model,
        )),
        stop_date_times: make_stop_datetimes(stop_times, timezone, *date, model)?,
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

pub fn make_stop_point(
    stop_point_idx: &StopPointIdx,
    model: &ModelRefs,
) -> navitia_proto::StopPoint {
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
            .house_number(stop_point_idx)
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
    let coord =
        model
            .stop_area_coord(stop_area_name)
            .map(|coord| navitia_proto::GeographicalCoord {
                lat: coord.lat,
                lon: coord.lon,
            });
    let proto = navitia_proto::StopArea {
        name: Some(stop_area_name.to_string()),
        uri: model.stop_area_uri(stop_area_name),
        coord,
        label: Some(stop_area_name.to_string()),
        codes: model
            .stop_area_codes(stop_area_name)
            .map(|codes| {
                codes
                    .map(|(key, value)| navitia_proto::Code {
                        r#type: key.clone(),
                        value: value.clone(),
                    })
                    .collect()
            })
            .unwrap_or_else(Vec::new),
        timezone: model
            .stop_area_timezone(stop_area_name)
            .map(|timezone| timezone.to_string()),
        ..Default::default()
    };
    Some(proto)
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
    stop_times: StopTimes,
    timezone: Timezone,
    date: NaiveDate,
    model: &ModelRefs,
) -> Result<Vec<navitia_proto::StopDateTime>, Error> {
    let mut result = Vec::new();
    for stop_time in stop_times {
        let arrival_seconds = i64::from(stop_time.debark_time.total_seconds());
        let arrival = to_utc_timestamp(timezone, date, arrival_seconds)?;
        let departure_seconds = i64::from(stop_time.board_time.total_seconds());
        let departure = to_utc_timestamp(timezone, date, departure_seconds)?;
        let stop_point_idx = stop_time.stop;
        let proto = navitia_proto::StopDateTime {
            arrival_date_time: Some(arrival),
            departure_date_time: Some(departure),
            stop_point: Some(make_stop_point(&stop_point_idx, model)),
            ..Default::default()
        };
        result.push(proto);
    }
    Ok(result)
}

fn make_additional_informations(
    vehicle_section: &VehicleSection,
    real_time_level: &RealTimeLevel,
    models: &ModelRefs<'_>,
) -> Vec<i32> {
    let vehicle_journey_idx = &vehicle_section.vehicle_journey;
    let date = &vehicle_section.day_for_vehicle_journey;
    let from_stoptime_idx = vehicle_section.from_stoptime_idx;
    let to_stoptime_idx = vehicle_section.to_stoptime_idx;

    let mut result = Vec::new();

    let has_datetime_estimated = models.has_datetime_estimated(
        vehicle_journey_idx,
        date,
        from_stoptime_idx,
        to_stoptime_idx,
        real_time_level,
    );

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
            let from = Coord {
                lon: from_to.0.lon,
                lat: from_to.0.lat,
            };
            let to = Coord {
                lon: from_to.1.lon,
                lat: from_to.1.lat,
            };
            acc + distance_coord_to_coord(&from, &to)
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
    model
        .co2_emission(vehicle_journey_idx)
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
        .contributors()
        .map(|contributor| navitia_proto::FeedPublisher {
            id: contributor.id,
            name: Some(contributor.name),
            license: contributor.license,
            url: contributor.url,
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
    stoptimes: StopTimes,
    model: &ModelRefs,
) -> Vec<navitia_proto::GeographicalCoord> {
    match stoptimes {
        StopTimes::Base(stop_times) => stop_times
            .filter_map(|stop_time| {
                let stop_point_idx = &stop_time.stop;
                let coord = &model.coord(stop_point_idx)?;
                Some(navitia_proto::GeographicalCoord {
                    lat: coord.lat,
                    lon: coord.lon,
                })
            })
            .collect(),
        StopTimes::New(stop_times) => stop_times
            .filter_map(|stop_time| {
                let stop_point_idx = &stop_time.stop;
                let coord = &model.coord(stop_point_idx)?;
                Some(navitia_proto::GeographicalCoord {
                    lat: coord.lat,
                    lon: coord.lon,
                })
            })
            .collect(),
    }
}

fn make_impact(
    impact: &Impact,
    disruption: &Disruption,
    model: &ModelRefs<'_>,
) -> navitia_proto::Impact {
    let mut impacted_objects: Vec<navitia_proto::ImpactedObject> = impact
        .impacted_pt_objects
        .iter()
        .filter_map(|i| make_impacted_object_from_impacted(i, model).ok())
        .collect();
    for informed in &impact.informed_pt_objects {
        if let Ok(object) = make_impacted_object_from_informed(informed, model) {
            impacted_objects.push(object)
        }
    }

    let mut proto = navitia_proto::Impact {
        uri: Some(impact.id.clone()),
        disruption_uri: Some(disruption.id.clone()),
        application_periods: impact.application_periods.iter().map(make_period).collect(),
        updated_at: u64::try_from(impact.updated_at.timestamp()).ok(),
        tags: disruption.tags.iter().map(|t| t.name.clone()).collect(),
        cause: Some(disruption.cause.wording.clone()),
        messages: impact.messages.iter().map(make_message).collect(),
        severity: Some(make_severity(&impact.severity)),
        contributor: disruption.contributor.clone(),
        impacted_objects,
        category: Some(disruption.cause.category.clone()),
        application_patterns: impact
            .application_patterns
            .iter()
            .filter_map(|ap| make_application_pattern(ap).ok())
            .collect(),
        properties: disruption.properties.iter().map(make_property).collect(),
        ..Default::default()
    };
    proto.set_status(navitia_proto::ActiveStatus::Active);

    proto
}

fn make_impacted_object_from_impacted(
    object: &Impacted,
    model: &ModelRefs<'_>,
) -> Result<navitia_proto::ImpactedObject, Error> {
    let pt_object = match object {
        Impacted::RouteDeleted(route) => make_route_pt_object(&route.id, model),
        Impacted::LineDeleted(line) => make_line_pt_object(&line.id, model),
        Impacted::NetworkDeleted(network) => make_network_pt_object(&network.id, model),
        Impacted::LineSection(line_section) => make_line_pt_object(&line_section.line.id, model),
        Impacted::RailSection(rail_section) => make_line_pt_object(&rail_section.line.id, model),
        Impacted::StopAreaDeleted(stop_area) => make_stop_area_pt_object(&stop_area.id, model),
        Impacted::StopPointDeleted(stop_point) => {
            if let Some(stop_point_id) = model.stop_point_idx(&stop_point.id) {
                make_stop_point_pt_object(&stop_point_id, model)
            } else {
                return Err(format_err!(
                    "StopPoint.id: {} not found in BaseModel",
                    stop_point.id
                ));
            }
        }
        _ => return Err(format_err!("***")),
    };
    let impacted_section = match object {
        Impacted::LineSection(line_section) => Some(make_line_section_impact(line_section, model)?),
        _ => None,
    };
    let impacted_rail_section = match object {
        Impacted::RailSection(rail_section) => Some(make_rail_section_impact(rail_section, model)?),
        _ => None,
    };
    Ok(navitia_proto::ImpactedObject {
        pt_object: Some(pt_object?),
        impacted_stops: vec![],
        impacted_section,
        impacted_rail_section,
    })
}

fn make_impacted_object_from_informed(
    object: &Informed,
    model: &ModelRefs<'_>,
) -> Result<navitia_proto::ImpactedObject, Error> {
    let pt_object = match object {
        Informed::Trip(trip) => {
            return Err(format_err!(
                "Cannot create ImpactedObject from Informed::Trip"
            ))
        }
        Informed::Route(route) => make_route_pt_object(&route.id, model),
        Informed::Line(line) => make_line_pt_object(&line.id, model),
        Informed::Network(network) => make_network_pt_object(&network.id, model),
        Informed::StopArea(stop_area) => make_stop_area_pt_object(&stop_area.id, model),
        Informed::StopPoint(stop_point) => {
            if let Some(stop_point_id) = model.stop_point_idx(&stop_point.id) {
                make_stop_point_pt_object(&stop_point_id, model)
            } else {
                return Err(format_err!(
                    "StopPoint.id: {} not found in BaseModel",
                    stop_point.id
                ));
            }
        }
        Informed::Unknown => {
            return Err(format_err!(
                "Cannot create ImpactedObject from Informed::Unknown"
            ))
        }
    };

    Ok(navitia_proto::ImpactedObject {
        pt_object: Some(pt_object?),
        impacted_stops: vec![],
        impacted_section: None,
        impacted_rail_section: None,
    })
}

fn make_property(property: &DisruptionProperty) -> navitia_proto::DisruptionProperty {
    navitia_proto::DisruptionProperty {
        key: property.key.clone(),
        r#type: property.type_.clone(),
        value: property.value.clone(),
    }
}

fn make_period(period: &DateTimePeriod) -> navitia_proto::Period {
    navitia_proto::Period {
        begin: u64::try_from(period.start().timestamp()).ok(),
        end: u64::try_from(period.end().timestamp()).ok(),
    }
}

fn make_severity(severity: &Severity) -> navitia_proto::Severity {
    let mut proto = navitia_proto::Severity {
        name: severity.wording.clone(),
        color: severity.color.clone(),
        priority: severity.priority,
        ..Default::default()
    };
    proto.set_effect(make_effect(severity.effect));
    proto
}

fn make_effect(effect: Effect) -> navitia_proto::severity::Effect {
    match effect {
        Effect::NoService => navitia_proto::severity::Effect::NoService,
        Effect::ReducedService => navitia_proto::severity::Effect::ReducedService,
        Effect::SignificantDelays => navitia_proto::severity::Effect::SignificantDelays,
        Effect::Detour => navitia_proto::severity::Effect::Detour,
        Effect::AdditionalService => navitia_proto::severity::Effect::AdditionalService,
        Effect::ModifiedService => navitia_proto::severity::Effect::ModifiedService,
        Effect::OtherEffect => navitia_proto::severity::Effect::OtherEffect,
        Effect::UnknownEffect => navitia_proto::severity::Effect::UnknownEffect,
        Effect::StopMoved => navitia_proto::severity::Effect::StopMoved,
    }
}

fn make_message(message: &Message) -> navitia_proto::MessageContent {
    let mut channel = navitia_proto::Channel {
        id: message.channel_id.clone(),
        name: Some(message.channel_name.clone()),
        content_type: message.channel_content_type.clone(),
        channel_types: vec![],
    };
    for channel_type in &message.channel_types {
        channel.push_channel_types(make_channel_type(channel_type))
    }

    navitia_proto::MessageContent {
        text: Some(message.text.clone()),
        channel: Some(channel),
    }
}

fn make_channel_type(channel_type: &ChannelType) -> navitia_proto::channel::ChannelType {
    match channel_type {
        ChannelType::Web => navitia_proto::channel::ChannelType::Web,
        ChannelType::Sms => navitia_proto::channel::ChannelType::Sms,
        ChannelType::Email => navitia_proto::channel::ChannelType::Email,
        ChannelType::Mobile => navitia_proto::channel::ChannelType::Mobile,
        ChannelType::Notification => navitia_proto::channel::ChannelType::Notification,
        ChannelType::Twitter => navitia_proto::channel::ChannelType::Twitter,
        ChannelType::Facebook => navitia_proto::channel::ChannelType::Facebook,
        ChannelType::UnknownType => navitia_proto::channel::ChannelType::UnknownType,
        ChannelType::Title => navitia_proto::channel::ChannelType::Title,
        ChannelType::Beacon => navitia_proto::channel::ChannelType::Beacon,
    }
}

fn make_application_pattern(
    pattern: &ApplicationPattern,
) -> Result<navitia_proto::ApplicationPattern, Error> {
    let app_period = DateTimePeriod::new(
        pattern.begin_date.and_hms(0, 0, 0),
        pattern.end_date.and_hms(0, 0, 0),
    )?;
    Ok(navitia_proto::ApplicationPattern {
        application_period: make_period(&app_period),
        time_slots: pattern.time_slots.iter().map(make_time_slot).collect(),
        ..Default::default()
    })
}

fn make_time_slot(time_slot: &TimeSlot) -> navitia_proto::TimeSlot {
    navitia_proto::TimeSlot {
        begin: time_slot.begin.num_seconds_from_midnight(),
        end: time_slot.end.num_seconds_from_midnight(),
    }
}

fn make_network_pt_object(
    network_id: &str,
    model: &ModelRefs<'_>,
) -> Result<navitia_proto::PtObject, Error> {
    if let Some(network) = model.network(network_id) {
        let proto_network = make_network(network, model);

        let mut proto = navitia_proto::PtObject {
            name: network.name.clone(),
            uri: network.id.clone(),
            network: Some(proto_network),
            ..Default::default()
        };
        proto.set_embedded_type(navitia_proto::NavitiaType::Network);
        Ok(proto)
    } else {
        Err(format_err!(
            "Network.id: {} not found in BaseModel",
            network_id
        ))
    }
}

fn make_line_pt_object(
    line_id: &str,
    model: &ModelRefs<'_>,
) -> Result<navitia_proto::PtObject, Error> {
    if let Some(line) = model.line(line_id) {
        let proto_line = make_line(line, model);

        let mut proto = navitia_proto::PtObject {
            name: line.name.clone(),
            uri: line.id.clone(),
            line: Some(proto_line),
            ..Default::default()
        };
        proto.set_embedded_type(navitia_proto::NavitiaType::Line);
        Ok(proto)
    } else {
        Err(format_err!("Line.id: {} not found in BaseModel", line_id))
    }
}

fn make_route_pt_object(
    route_id: &str,
    model: &ModelRefs<'_>,
) -> Result<navitia_proto::PtObject, Error> {
    if let Some(route) = model.route(route_id) {
        let proto_route = make_route(route, model);
        let mut proto = navitia_proto::PtObject {
            name: route.name.clone(),
            uri: route.id.clone(),
            route: Some(Box::new(proto_route)),
            ..Default::default()
        };
        proto.set_embedded_type(navitia_proto::NavitiaType::Route);
        Ok(proto)
    } else {
        Err(format_err!("Route.id: {} not found in BaseModel", route_id))
    }
}

pub fn make_route(route: &Route, model: &ModelRefs) -> navitia_proto::Route {
    let direction = if let Some(destination_id) = &route.destination_id {
        if let Ok(direction) = make_stop_area_pt_object(destination_id, model) {
            Some(Box::new(direction))
        } else {
            None
        }
    } else {
        None
    };

    navitia_proto::Route {
        name: Some(route.name.clone()),
        uri: Some(route.id.clone()),
        codes: route.codes.iter().map(make_pt_object_code).collect(),
        direction_type: route.direction_type.clone(),
        direction,
        ..Default::default()
    }
}

pub fn make_line(line: &Line, model: &ModelRefs) -> navitia_proto::Line {
    navitia_proto::Line {
        name: Some(line.name.clone()),
        uri: Some(line.id.clone()),
        code: line.code.clone(),
        codes: line.codes.iter().map(make_pt_object_code).collect(),
        color: line.color.as_ref().map(|color| format!("{}", color)),
        text_color: line
            .text_color
            .as_ref()
            .map(|text_color| format!("{}", text_color)),
        commercial_mode: make_commercial_mode(&line.commercial_mode_id, model),
        opening_time: line.opening_time.map(|t| t.total_seconds()),
        closing_time: line.closing_time.map(|t| t.total_seconds()),
        ..Default::default()
    }
}

pub fn make_network(network: &Network, model: &ModelRefs) -> navitia_proto::Network {
    navitia_proto::Network {
        name: Some(network.name.clone()),
        uri: Some(network.id.clone()),
        codes: network.codes.iter().map(make_pt_object_code).collect(),
        ..Default::default()
    }
}

pub fn make_stop_area_pt_object(
    id: &str,
    model: &ModelRefs,
) -> Result<navitia_proto::PtObject, Error> {
    if let Some(stop_area) = model.stop_area(id) {
        let proto_stop_area = make_stop_area_(&stop_area, model);

        let mut proto = navitia_proto::PtObject {
            name: stop_area.name.clone(),
            uri: stop_area.id.clone(),
            stop_area: Some(proto_stop_area),
            ..Default::default()
        };
        proto.set_embedded_type(navitia_proto::NavitiaType::StopArea);
        Ok(proto)
    } else {
        Err(format_err!("StopArea.id: {} not found in BaseModel", id))
    }
}

pub fn make_stop_area_(stop_area: &StopArea, model: &ModelRefs) -> navitia_proto::StopArea {
    navitia_proto::StopArea {
        name: Some(stop_area.name.clone()),
        uri: Some(stop_area.id.clone()),
        coord: Some(navitia_proto::GeographicalCoord {
            lat: stop_area.coord.lat,
            lon: stop_area.coord.lon,
        }),
        label: Some(stop_area.name.clone()),
        codes: stop_area.codes.iter().map(make_pt_object_code).collect(),
        timezone: stop_area.timezone.map(|timezone| timezone.to_string()),
        ..Default::default()
    }
}

pub fn make_line_section_impact(
    line_section: &LineSectionDisruption,
    model: &ModelRefs,
) -> Result<navitia_proto::LineSectionImpact, Error> {
    Ok(navitia_proto::LineSectionImpact {
        from: make_stop_area_pt_object(&line_section.start.id, model).ok(),
        to: make_stop_area_pt_object(&line_section.end.id, model).ok(),
        routes: line_section
            .routes
            .iter()
            .filter_map(|r| model.route(&r.id).map(|route| make_route(route, model)))
            .collect(),
    })
}

pub fn make_rail_section_impact(
    rail_section: &RailSectionDisruption,
    model: &ModelRefs,
) -> Result<navitia_proto::RailSectionImpact, Error> {
    Ok(navitia_proto::RailSectionImpact {
        from: make_stop_area_pt_object(&rail_section.start.id, model).ok(),
        to: make_stop_area_pt_object(&rail_section.end.id, model).ok(),
        routes: rail_section
            .routes
            .iter()
            .filter_map(|r| model.route(&r.id).map(|route| make_route(route, model)))
            .collect(),
    })
}

pub fn make_pt_object_code(code: &(String, String)) -> navitia_proto::Code {
    navitia_proto::Code {
        r#type: code.0.clone(),
        value: code.1.clone(),
    }
}

pub fn make_commercial_mode(
    commercial_mode_id: &str,
    model: &ModelRefs,
) -> Option<navitia_proto::CommercialMode> {
    model
        .commercial_mode(commercial_mode_id)
        .map(|c| navitia_proto::CommercialMode {
            uri: Some(c.id.clone()),
            name: Some(c.name.clone()),
        })
}
