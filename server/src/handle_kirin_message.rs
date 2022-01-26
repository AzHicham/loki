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

use anyhow::{bail, format_err, Context, Error};
use launch::loki::{
    chrono::{Duration, NaiveDate},
    models::{
        base_model::BaseModel,
        real_time_disruption::{
            Cause, ChannelType, Disruption, Effect, Impact, Impacted, Message, Severity, StopTime,
            TimePeriod, TripDisruption, VehicleJourneyId,
        },
    },
    time::SecondsSinceTimezonedDayStart,
    timetables::FlowDirection,
    NaiveDateTime,
};

use crate::{chaos_proto, handle_chaos_message::make_effect};

pub fn handle_kirin_protobuf(
    feed_entity: &chaos_proto::gtfs_realtime::FeedEntity,
    header_datetime: &NaiveDateTime,
    base_model: &BaseModel,
) -> Result<Disruption, Error> {
    let disruption_id = feed_entity.get_id().to_string();
    if feed_entity.has_trip_update().not() {
        bail!("Feed entity has no trip_update");
    }

    let (start_date, end_date) = base_model.validity_period();

    let publication_period =
        TimePeriod::new(start_date.and_hms(0, 0, 0), end_date.and_hms(24, 0, 0))
            .with_context(|| "BaseModel has a bad validity period".to_string())?;

    let trip_update = feed_entity.get_trip_update();
    let trip = trip_update.get_trip();

    let disruption = Disruption {
        id: disruption_id.clone(),
        reference: Some(disruption_id.clone()),
        contributor: chaos_proto::kirin::exts::contributor.get(trip),
        publication_period,
        cause: Cause::default(),
        tags: vec![],
        properties: vec![],
        impacts: vec![make_impact(
            trip_update,
            disruption_id,
            header_datetime,
            base_model,
        )?],
    };

    Ok(disruption)
}

fn make_impact(
    trip_update: &chaos_proto::gtfs_realtime::TripUpdate,
    disruption_id: String,
    header_datetime: &NaiveDateTime,
    base_model: &BaseModel,
) -> Result<Impact, Error> {
    let trip = trip_update.get_trip();
    let effect: Effect =
        if let Some(proto_effect) = chaos_proto::kirin::exts::effect.get(trip_update) {
            make_effect(proto_effect)
        } else {
            return Err(format_err!("TripUpdate has an empty effect."));
        };

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

    let severity = make_severity(effect);

    let stop_times = make_stop_times(trip_update, &reference_date)?;

    let base_application_period =
        if let Some(idx) = base_model.vehicle_journey_idx(&vehicle_journey_id) {
            base_model.trip_time_period(idx, &reference_date)
        } else {
            None
        };
    let model_validity_period = {
        let (start_date, end_date) = base_model.validity_period();
        TimePeriod::new(start_date.and_hms(0, 0, 0), end_date.and_hms(24, 0, 0))
            .with_context(|| "BaseModel has a bad validity period".to_string())?
    };

    let stop_times_time_period = make_time_period(&stop_times, &reference_date);

    // we want the application period to cover
    // - the base vehicle period (if any)
    // - the period of the stop_times in the kirin message (if any)
    // When both are absent, we use the validity period of the model,
    let application_period = match (base_application_period, stop_times_time_period) {
        (None, None) => model_validity_period,
        (Some(base_period), None) => base_period,
        (None, Some(stop_times_period)) => stop_times_period,
        (Some(base_period), Some(stop_times_period)) => {
            let start = std::cmp::min(base_period.start(), stop_times_period.end());
            let end = std::cmp::max(base_period.end(), stop_times_period.end());
            TimePeriod::new(start, end).unwrap_or(model_validity_period)
        }
    };

    let company_id = chaos_proto::kirin::exts::company_id.get(trip);
    let physical_mode_id =
        chaos_proto::kirin::exts::physical_mode_id.get(trip_update.get_vehicle());
    let headsign = chaos_proto::kirin::exts::headsign.get(trip_update);

    let mut impacted_pt_objects = Vec::new();

    let trip_id = VehicleJourneyId {
        id: vehicle_journey_id,
    };

    // Please see kirin-proto documentation to understand the following code
    // https://github.com/CanalTP/chaos-proto/blob/6b2fea75cdb39c7850571b01888b550881027068/kirin_proto_doc.rs#L67-L89
    use Effect::*;
    match effect {
        NoService => {
            impacted_pt_objects.push(Impacted::TripDeleted(trip_id, reference_date));
        }
        OtherEffect | UnknownEffect | ReducedService | SignificantDelays | Detour
        | ModifiedService => {
            let trip_disruption = TripDisruption {
                trip_id,
                trip_date: reference_date,
                stop_times,
                company_id,
                physical_mode_id,
                headsign,
            };
            impacted_pt_objects.push(Impacted::BaseTripUpdated(trip_disruption));
        }
        AdditionalService => {
            let trip_disruption = TripDisruption {
                trip_id,
                trip_date: reference_date,
                stop_times,
                company_id,
                physical_mode_id,
                headsign,
            };
            impacted_pt_objects.push(Impacted::NewTripUpdated(trip_disruption));
        }
        StopMoved => {
            bail!("Unhandled effect on FeedEntity: {:?}", effect);
        }
    }

    let informed_pt_objects = Vec::new();

    Ok(Impact {
        id: disruption_id,
        updated_at: *header_datetime,
        application_periods: vec![application_period],
        application_patterns: vec![],
        severity,
        messages: make_message(trip_update),
        impacted_pt_objects,
        informed_pt_objects,
    })
}

fn make_time_period(stop_times: &[StopTime], reference_date: &NaiveDate) -> Option<TimePeriod> {
    let min = stop_times
        .iter()
        .map(|stop_time| std::cmp::min(stop_time.arrival_time, stop_time.departure_time))
        .min()?;

    let max = stop_times
        .iter()
        .map(|stop_time| std::cmp::max(stop_time.arrival_time, stop_time.departure_time))
        .max()?;

    let start_datetime =
        reference_date.and_hms(0, 0, 0) + Duration::seconds(i64::from(min.total_seconds()));
    let end_datetime =
        reference_date.and_hms(0, 0, 0) + Duration::seconds(i64::from(max.total_seconds()));

    // we add one second to the end_datetime since a TimePeriod is an open interval at the end
    let end_datetime = end_datetime + Duration::seconds(1);
    TimePeriod::new(start_datetime, end_datetime).ok()
}

fn make_stop_times(
    trip_update: &chaos_proto::gtfs_realtime::TripUpdate,
    reference_date: &NaiveDate,
) -> Result<Vec<StopTime>, Error> {
    let stop_times =
        create_stop_times_from_proto(trip_update.get_stop_time_update(), reference_date)
            .with_context(|| "Could not handle stop times in kirin disruption.")?;

    Ok(stop_times)
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
        read_status(proto.get_arrival()).context("StopTime has a bad arrival status.")?
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

fn make_message(trip_update: &chaos_proto::gtfs_realtime::TripUpdate) -> Vec<Message> {
    if let Some(text) = chaos_proto::kirin::exts::trip_message.get(trip_update) {
        let message = Message {
            text,
            channel_id: Some("rt".to_string()),
            channel_name: "rt".to_string(),
            channel_content_type: None,
            channel_types: vec![ChannelType::Web, ChannelType::Mobile],
        };
        vec![message]
    } else {
        vec![]
    }
}

fn make_severity(effect: Effect) -> Severity {
    Severity {
        wording: Some(make_severity_wording(effect)),
        color: Some("#000000".to_string()),
        priority: Some(42),
        effect,
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
