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

use std::ops::Not;

use anyhow::{format_err, Context, Error};
use launch::loki::{
    chrono::NaiveDate,
    models::{
        base_model::BaseModel,
        real_time_disruption::{
            Cause, ChannelType, DateTimePeriod, Disrupt, Disruption, Effect, Impact, Message,
            PtObject, Severity, StopTime, Trip, Trip_, Update,
        },
        RealTimeModel,
    },
    time::SecondsSinceTimezonedDayStart,
    timetables::FlowDirection,
    NaiveDateTime,
};

use crate::chaos_proto;

pub fn handle_kirin_protobuf(
    feed_entity: &chaos_proto::gtfs_realtime::FeedEntity,
    base_model: &BaseModel,
    realtime_model: &RealTimeModel,
) -> Result<Disruption, Error> {
    let disruption_id = feed_entity.get_id().to_string();

    if feed_entity.has_trip_update().not() {
        return Err(format_err!("Feed entity has no trip_update"));
    }
    let trip_update = feed_entity.get_trip_update();

    let update = read_trip_update(trip_update, base_model, realtime_model).with_context(|| {
        format!(
            "Could not handle trip update of kirin disruption {}.",
            disruption_id
        )
    })?;

    let result = Disruption {
        id: disruption_id,
        updates: vec![update],
    };
    Ok(result)
}

fn read_trip_update(
    trip_update: &chaos_proto::gtfs_realtime::TripUpdate,
    base_model: &BaseModel,
    realtime_model: &RealTimeModel,
) -> Result<Update, Error> {
    if let Some(effect) = chaos_proto::kirin::exts::effect.get(trip_update) {
        use chaos_proto::gtfs_realtime::Alert_Effect::*;
        match effect {
            NO_SERVICE => {
                let trip = read_trip(trip_update.get_trip())?;
                Ok(Update::Delete(trip))
            }
            ADDITIONAL_SERVICE => {
                let trip = read_trip(trip_update.get_trip())?;
                let stop_times = create_stop_times_from_proto(
                    trip_update.get_stop_time_update(),
                    &trip.reference_date,
                )?;
                let trip_exists_in_base = {
                    let has_vj_idx = base_model.vehicle_journey_idx(&trip.vehicle_journey_id);
                    match has_vj_idx {
                        None => false,
                        Some(vj_idx) => base_model.trip_exists(vj_idx, trip.reference_date),
                    }
                };
                if trip_exists_in_base {
                    return Err(format_err!(
                        "Additional service for trip {:?} that exists in the base schedule.",
                        trip
                    ));
                }

                let trip_exists_in_realtime = realtime_model.contains_new_vehicle_journey(
                    trip.vehicle_journey_id.as_str(),
                    &trip.reference_date,
                );
                if trip_exists_in_realtime {
                    Ok(Update::Modify(trip, stop_times))
                } else {
                    Ok(Update::Add(trip, stop_times))
                }
            }
            REDUCED_SERVICE | SIGNIFICANT_DELAYS | DETOUR | MODIFIED_SERVICE => {
                let trip = read_trip(trip_update.get_trip())?;
                let stop_times = create_stop_times_from_proto(
                    trip_update.get_stop_time_update(),
                    &trip.reference_date,
                )
                .with_context(|| "Could not handle stop times in kirin disruption.")?;
                Ok(Update::Modify(trip, stop_times))
            }

            OTHER_EFFECT | UNKNOWN_EFFECT | STOP_MOVED => {
                Err(format_err!("Unhandle effect on FeedEntity: {:?}", effect))
            }
        }
    } else {
        Err(format_err!("No effect on FeedEntity."))
    }
}

fn read_trip(trip_descriptor: &chaos_proto::gtfs_realtime::TripDescriptor) -> Result<Trip, Error> {
    let vehicle_journey_id = {
        if trip_descriptor.has_trip_id().not() {
            return Err(format_err!("TripDescriptor has an empty trip_id."));
        }
        trip_descriptor.get_trip_id().to_string()
    };

    let reference_date = {
        if trip_descriptor.has_start_date().not() {
            return Err(format_err!("TripDescriptor has an empty start_time."));
        }
        let start_date = trip_descriptor.get_start_date();
        NaiveDate::parse_from_str(start_date, "%Y%m%d").with_context(|| {
            format!(
                "TripDescriptor has a start date {} that could not be parsed.",
                start_date
            )
        })?
    };

    let result = Trip {
        vehicle_journey_id,
        reference_date,
    };
    Ok(result)
}

fn create_stop_times_from_proto(
    proto: &[chaos_proto::gtfs_realtime::TripUpdate_StopTimeUpdate],
    reference_date: &NaiveDate,
) -> Result<Vec<StopTime>, Error> {
    proto
        .iter()
        .map(|p| create_stop_time_from_proto(p, reference_date))
        .collect()
}

fn create_stop_time_from_proto(
    proto: &chaos_proto::gtfs_realtime::TripUpdate_StopTimeUpdate,
    reference_date: &NaiveDate,
) -> Result<StopTime, Error> {
    let has_arrival_time = if proto.has_arrival() {
        let arrival_time = read_time(proto.get_arrival(), reference_date)
            .context("StopTime has a bad arrival time")?;
        Some(arrival_time)
    } else {
        None
    };

    let has_departure_time = if proto.has_departure() {
        let departure_time = read_time(proto.get_departure(), reference_date)
            .context("StopTime has a bad departure time")?;
        Some(departure_time)
    } else {
        None
    };

    let (arrival_time, departure_time) = match (has_arrival_time, has_departure_time) {
        (Some(arrival_time), Some(departure_time)) => (arrival_time, departure_time),
        (Some(arrival_time), None) => (arrival_time, arrival_time),
        (None, Some(departure_time)) => (departure_time, departure_time),
        (None, None) => {
            return Err(format_err!(
                "StopTime does not have an arrival time nor a departure time."
            ));
        }
    };

    let can_board = if proto.has_departure() {
        read_status(proto.get_departure()).context("StopTime has a bad departure status.")?
    } else {
        false
    };

    let can_debark = if proto.has_arrival() {
        read_status(proto.get_arrival()).context("StopTime has a bad arrical status.")?
    } else {
        false
    };

    let flow_direction = match (can_board, can_debark) {
        (true, true) => FlowDirection::BoardAndDebark,
        (true, false) => FlowDirection::BoardOnly,
        (false, true) => FlowDirection::DebarkOnly,
        (false, false) => FlowDirection::NoBoardDebark,
    };

    if proto.has_stop_id().not() {
        return Err(format_err!("StopTime does not have a stop_id."));
    }
    let stop_id = proto.get_stop_id().to_string();

    let stop_time = StopTime {
        stop_id,
        arrival_time,
        departure_time,
        flow_direction,
    };

    Ok(stop_time)
}

fn read_time(
    proto: &chaos_proto::gtfs_realtime::TripUpdate_StopTimeEvent,
    reference_date: &NaiveDate,
) -> Result<SecondsSinceTimezonedDayStart, Error> {
    if proto.has_time().not() {
        return Err(format_err!("The protobuf time field is empty."));
    }
    // this is a unix timestamp
    let time_i64 = proto.get_time();
    let naive_datetime = NaiveDateTime::from_timestamp_opt(time_i64, 0).ok_or_else(|| {
        format_err!(
            "Could not parse the time value {} as a unix timestamp.",
            time_i64
        )
    })?;

    let reference_date_at_midnight = reference_date.and_hms(0, 0, 0);
    let duration_from_ref = naive_datetime.signed_duration_since(reference_date_at_midnight);
    let duration_i64 = duration_from_ref.num_seconds();
    SecondsSinceTimezonedDayStart::from_seconds_i64(duration_i64).ok_or_else(|| {
        format_err!(
            "Could not translate the duration of {} seconds to SecondsSinceTimezonedDayStart.",
            duration_i64
        )
    })
}

fn read_status(
    proto: &chaos_proto::gtfs_realtime::TripUpdate_StopTimeEvent,
) -> Result<bool, Error> {
    use chaos_proto::kirin::StopTimeEventStatus::*;
    if let Some(stop_time_event_status) =
        chaos_proto::kirin::exts::stop_time_event_status.get(proto)
    {
        match stop_time_event_status {
            SCHEDULED | ADDED | ADDED_FOR_DETOUR => Ok(true),

            DELETED_FOR_DETOUR | DELETED => Ok(false),

            NO_DATA => Err(format_err!("No_data in stop time event status.")),
        }
    } else {
        Ok(false)
    }
}

pub fn create_disruption(
    feed_entity: &chaos_proto::gtfs_realtime::FeedEntity,
    header_datetime: NaiveDateTime,
    model_validity_period: &(NaiveDate, NaiveDate),
) -> Result<Disrupt, Error> {
    let disruption_id = feed_entity.get_id().to_string();
    if feed_entity.has_trip_update().not() {
        return Err(format_err!("Feed entity has no trip_update"));
    }
    // application_period == publication_period
    let application_period = DateTimePeriod {
        start: model_validity_period.0.and_hms(0, 0, 0),
        end: model_validity_period.1.and_hms(23, 59, 59),
    };
    let trip_update = feed_entity.get_trip_update();
    let trip = trip_update.get_trip();

    let disruption = Disrupt {
        id: disruption_id.clone(),
        reference: Some(disruption_id.clone()),
        contributor: chaos_proto::kirin::exts::contributor
            .get(trip)
            .unwrap_or_else(|| "".to_string()),
        publication_period: application_period,
        created_at: Some(header_datetime),
        updated_at: Some(header_datetime),
        cause: Cause::default(),
        tags: vec![],
        impacts: vec![make_impact(trip_update, disruption_id, header_datetime)?],
    };

    Ok(disruption)
}

fn make_impact(
    trip_update: &chaos_proto::gtfs_realtime::TripUpdate,
    disruption_id: String,
    header_datetime: NaiveDateTime,
) -> Result<Impact, Error> {
    let trip = trip_update.get_trip();
    let effect: Effect = chaos_proto::kirin::exts::effect
        .get(trip_update)
        .map(|e| e.into())
        .unwrap_or(Effect::UnknownEffect);

    let vehicle_journey_id = {
        if trip.has_trip_id().not() {
            return Err(format_err!("TripDescriptor has an empty trip_id."));
        }
        trip.get_trip_id().to_string()
    };

    let reference_date = {
        if trip.has_start_date().not() {
            return Err(format_err!("TripDescriptor has an empty start_time."));
        }
        let start_date = trip.get_start_date();
        NaiveDate::parse_from_str(start_date, "%Y%m%d").with_context(|| {
            format!(
                "TripDescriptor has a start date {} that could not be parsed.",
                start_date
            )
        })?
    };

    let stop_times = make_stop_times(trip_update, effect.clone(), &reference_date)?;
    let stop_times = match stop_times.is_empty() {
        true => None,
        false => Some(stop_times),
    };

    Ok(Impact {
        id: disruption_id.clone(),
        company_id: chaos_proto::kirin::exts::company_id.get(trip),
        physical_mode_id: chaos_proto::kirin::exts::physical_mode_id.get(trip_update.get_vehicle()),
        headsign: chaos_proto::kirin::exts::headsign.get(trip_update),
        created_at: Some(header_datetime),
        updated_at: Some(header_datetime),
        application_periods: vec![DateTimePeriod {
            start: reference_date.and_hms(0, 0, 0),
            end: reference_date.and_hms(12, 59, 59),
        }],
        application_patterns: vec![],
        severity: make_severity(effect, disruption_id, header_datetime),
        messages: make_message(trip_update, header_datetime),
        pt_objects: vec![PtObject::Trip_(Trip_ {
            id: vehicle_journey_id,
            created_at: Some(header_datetime),
            updated_at: Some(header_datetime),
        })],
        vehicle_info: stop_times,
    })
}

fn make_stop_times(
    trip_update: &chaos_proto::gtfs_realtime::TripUpdate,
    effect: Effect,
    reference_date: &NaiveDate,
) -> Result<Vec<StopTime>, Error> {
    match effect {
        Effect::NoService => Ok(vec![]),
        Effect::AdditionalService => {
            let stop_times =
                create_stop_times_from_proto(trip_update.get_stop_time_update(), reference_date)?;
            Ok(stop_times)
        }
        Effect::ReducedService
        | Effect::SignificantDelays
        | Effect::Detour
        | Effect::ModifiedService => {
            let stop_times =
                create_stop_times_from_proto(trip_update.get_stop_time_update(), reference_date)
                    .with_context(|| "Could not handle stop times in kirin disruption.")?;
            Ok(stop_times)
        }
        Effect::OtherEffect | Effect::UnknownEffect | Effect::StopMoved => {
            Err(format_err!("Unhandled effect on FeedEntity: {:?}", effect))
        }
    }
}

fn make_message(
    trip_update: &chaos_proto::gtfs_realtime::TripUpdate,
    header_datetime: NaiveDateTime,
) -> Vec<Message> {
    if let Some(text) = chaos_proto::kirin::exts::trip_message.get(trip_update) {
        let message = Message {
            text,
            channel_id: "rt".to_string(),
            channel_name: "rt".to_string(),
            channel_content_type: "".to_string(),
            channel_types: vec![ChannelType::Web, ChannelType::Mobile],
            created_at: Some(header_datetime),
            updated_at: Some(header_datetime),
        };
        vec![message]
    } else {
        vec![]
    }
}

fn make_severity(effect: Effect, disruption_id: String, timestamp: NaiveDateTime) -> Severity {
    Severity {
        id: disruption_id,
        wording: make_severity_wording(effect.clone()),
        color: "#000000".to_string(),
        priority: 42,
        effect,
        created_at: Some(timestamp),
        updated_at: Some(timestamp),
    }
}

fn make_severity_wording(effect: Effect) -> String {
    match effect {
        Effect::NoService => "trip canceled",
        Effect::SignificantDelays => "trip delayed",
        Effect::Detour => "detour",
        Effect::ModifiedService => "trip modified",
        Effect::ReducedService => "reduced service",
        Effect::AdditionalService => "additional service",
        Effect::OtherEffect => "other effect",
        Effect::StopMoved => "stop moved",
        Effect::UnknownEffect => "unknown effect",
    }
    .to_string()
}
