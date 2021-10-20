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

use failure::{format_err, Error};
use launch::loki::{
    chrono::NaiveDate,
    realtime::disruption::{Disruption, StopTime, Trip, Update},
    time::SecondsSinceUTCDayStart,
    timetables::FlowDirection,
    NaiveDateTime,
};

use crate::chaos_proto;

pub fn handle_kirin_protobuf(
    feed_entity: &chaos_proto::gtfs_realtime::FeedEntity,
) -> Result<Disruption, Error> {
    let disruption_id = feed_entity.get_id().to_string();

    if feed_entity.has_trip_update().not() {
        return Err(format_err!("Feed entity has no trip_update"));
    }
    let trip_update = feed_entity.get_trip_update();

    let update = read_trip_update(trip_update).map_err(|err| {
        format_err!(
            "Could not handle Kirin disruption {} because : {}",
            disruption_id,
            err
        )
    })?;

    let result = Disruption {
        id: disruption_id,
        updates: vec![update],
    };
    Ok(result)
}

fn read_trip_update(trip_update: &chaos_proto::gtfs_realtime::TripUpdate) -> Result<Update, Error> {
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
                Ok(Update::Add(trip, stop_times))
            }
            REDUCED_SERVICE | SIGNIFICANT_DELAYS | DETOUR | MODIFIED_SERVICE => {
                let trip = read_trip(trip_update.get_trip())?;
                let stop_times = create_stop_times_from_proto(
                    trip_update.get_stop_time_update(),
                    &trip.reference_date,
                )?;
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
        NaiveDate::parse_from_str(start_date, "%Y%m%d").map_err(|err| {
            format_err!(
                "TripDescriptor has a start date {} that could not be parsed : {}",
                start_date,
                err
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
            .map_err(|err| format_err!("StopTime has a bad arrival time : {}", err))?;
        Some(arrival_time)
    } else {
        None
    };

    let has_departure_time = if proto.has_departure() {
        let departure_time = read_time(proto.get_departure(), reference_date)
            .map_err(|err| format_err!("StopTime has a bad departure time : {}", err))?;
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
        read_status(proto.get_departure())
            .map_err(|err| format_err!("StopTime has a bad departure status : {}", err))?
    } else {
        false
    };

    let can_debark = if proto.has_arrival() {
        read_status(proto.get_arrival())
            .map_err(|err| format_err!("StopTime has a bad arrival status : {}", err))?
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
) -> Result<SecondsSinceUTCDayStart, Error> {
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
    SecondsSinceUTCDayStart::from_seconds_i64(duration_i64).ok_or_else(|| {
        format_err!(
            "Could not translate the duration of {} seconds to SecondsSinceUTCDayStart.",
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
