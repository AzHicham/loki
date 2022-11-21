// Copyright  (C) 2020, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Hove (www.kisio.com).
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

use anyhow::{bail, format_err, Context, Error};
use loki_launch::loki::{
    chrono::{Duration, NaiveDate, NaiveTime},
    models::{
        base_model::{strip_id_prefix, BaseModel, PREFIX_ID_STOP_POINT, PREFIX_ID_VEHICLE_JOURNEY},
        real_time_disruption::{
            kirin_disruption::{self, KirinDisruption, UpdateData, UpdateType},
            time_periods::TimePeriod,
            Effect, VehicleJourneyId,
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
) -> Result<KirinDisruption, Error> {
    let disruption_id = feed_entity
        .id
        .as_ref()
        .ok_or_else(|| format_err!("'FeedEntity' has no 'id'"))?
        .to_string();

    let trip_update = feed_entity
        .trip_update
        .as_ref()
        .ok_or_else(|| format_err!("Feed entity has no trip_update"))?;
    let trip_descriptor = trip_update
        .trip
        .as_ref()
        .ok_or_else(|| format_err!("'TripUpdate' has no 'trip'"))?;
    let contributor = chaos_proto::kirin::exts::contributor.get(trip_descriptor);

    let effect: Effect =
        if let Some(proto_effect) = chaos_proto::kirin::exts::effect.get(trip_update) {
            let proto_effect = proto_effect
                .enum_value()
                .map_err(|value| format_err!("'{}' is not a valid 'alert::Effect'", value))?;
            make_effect(proto_effect)
        } else {
            bail!("TripUpdate has an empty effect.");
        };

    let vehicle_journey_id = if let Some(trip_id) = trip_descriptor.trip_id.as_ref() {
        strip_id_prefix(trip_id, PREFIX_ID_VEHICLE_JOURNEY).to_string()
    } else {
        bail!("TripDescriptor has an empty trip_id.")
    };

    let reference_date = if let Some(start_date) = trip_descriptor.start_date.as_ref() {
        NaiveDate::parse_from_str(start_date, "%Y%m%d").with_context(|| {
            format!(
                "TripDescriptor has a start date '{}' that could not be parsed.",
                start_date
            )
        })?
    } else {
        bail!("TripDescriptor has an empty start_time.");
    };

    let stop_times = make_stop_times(trip_update, reference_date)?;

    let base_application_period =
        if let Some(idx) = base_model.vehicle_journey_idx(&vehicle_journey_id) {
            base_model.trip_time_period(idx, reference_date)
        } else {
            None
        };
    let model_validity_period = {
        let (start_date, end_date) = base_model.validity_period();
        let start_time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // 00:00:00 is a valid time
        let end_time = NaiveTime::from_hms_opt(23, 59, 59).unwrap(); // 23:59:59 is a valid time
        TimePeriod::new(start_date.and_time(start_time), end_date.and_time(end_time))
            .with_context(|| "BaseModel has a bad validity period".to_string())?
    };

    let stop_times_time_period = make_time_period(&stop_times, reference_date);

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

    let company_id = chaos_proto::kirin::exts::company_id.get(trip_descriptor);
    let vehicle = trip_update
        .vehicle
        .as_ref()
        .ok_or_else(|| format_err!("'TripUpdate' has no 'VehicleDescriptor'"))?;
    let physical_mode_id = chaos_proto::kirin::exts::physical_mode_id.get(vehicle);
    let headsign = chaos_proto::kirin::exts::headsign.get(trip_update);

    let trip_id = VehicleJourneyId {
        id: vehicle_journey_id,
    };

    // Please see kirin-proto documentation to understand the following code
    // https://github.com/hove-io/chaos-proto/blob/6b2fea75cdb39c7850571b01888b550881027068/kirin_proto_doc.rs#L67-L89
    use Effect::*;
    let update = match effect {
        NoService => UpdateType::TripDeleted(),
        OtherEffect | UnknownEffect | ReducedService | SignificantDelays | Detour
        | ModifiedService => UpdateType::BaseTripUpdated(UpdateData {
            stop_times,
            company_id,
            physical_mode_id,
            headsign,
        }),
        AdditionalService => UpdateType::NewTripUpdated(UpdateData {
            stop_times,
            company_id,
            physical_mode_id,
            headsign,
        }),
        StopMoved => {
            bail!("Unhandled effect on FeedEntity: {:?}", effect);
        }
    };

    let message = chaos_proto::kirin::exts::trip_message.get(trip_update);

    Ok(KirinDisruption {
        id: disruption_id,
        contributor,
        message,
        updated_at: *header_datetime,
        application_period,
        effect,
        trip_id,
        trip_date: reference_date,
        update,
    })
}

fn make_time_period(
    stop_times: &[kirin_disruption::StopTime],
    reference_date: NaiveDate,
) -> Option<TimePeriod> {
    let min = stop_times
        .iter()
        .map(|stop_time| std::cmp::min(stop_time.arrival_time, stop_time.departure_time))
        .min()?;

    let max = stop_times
        .iter()
        .map(|stop_time| std::cmp::max(stop_time.arrival_time, stop_time.departure_time))
        .max()?;

    let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // 00:00:00 is a valid time

    let start_datetime =
        reference_date.and_time(midnight) + Duration::seconds(i64::from(min.total_seconds()));
    let end_datetime =
        reference_date.and_time(midnight) + Duration::seconds(i64::from(max.total_seconds()));

    // we add one second to the end_datetime since a TimePeriod is an open interval at the end
    let end_datetime = end_datetime + Duration::seconds(1);
    TimePeriod::new(start_datetime, end_datetime).ok()
}

fn make_stop_times(
    trip_update: &chaos_proto::gtfs_realtime::TripUpdate,
    reference_date: NaiveDate,
) -> Result<Vec<kirin_disruption::StopTime>, Error> {
    let stop_times = create_stop_times_from_proto(&trip_update.stop_time_update, reference_date)
        .with_context(|| "Could not handle stop times in kirin disruption.")?;

    Ok(stop_times)
}

fn create_stop_times_from_proto(
    proto: &[chaos_proto::gtfs_realtime::trip_update::StopTimeUpdate],
    reference_date: NaiveDate,
) -> Result<Vec<kirin_disruption::StopTime>, Error> {
    proto
        .iter()
        .map(|p| create_stop_time_from_proto(p, reference_date))
        .collect()
}

fn create_stop_time_from_proto(
    proto: &chaos_proto::gtfs_realtime::trip_update::StopTimeUpdate,
    reference_date: NaiveDate,
) -> Result<kirin_disruption::StopTime, Error> {
    let departure = proto.departure.as_ref();
    let arrival = proto.arrival.as_ref();
    let has_arrival_time = arrival
        .map(|arrival| {
            read_time(arrival, reference_date).context("StopTime has a bad arrival time")
        })
        .transpose()?;

    let has_departure_time = departure
        .map(|departure| {
            read_time(departure, reference_date).context("StopTime has a bad departure time")
        })
        .transpose()?;

    let (arrival_time, departure_time) = match (has_arrival_time, has_departure_time) {
        (Some(arrival_time), Some(departure_time)) => (arrival_time, departure_time),
        (Some(arrival_time), None) => (arrival_time, arrival_time),
        (None, Some(departure_time)) => (departure_time, departure_time),
        (None, None) => {
            bail!("StopTime does not have an arrival time nor a departure time.");
        }
    };

    let can_board = departure
        .map(|departure| read_status(departure).context("StopTime has a bad departure status."))
        .transpose()?
        .unwrap_or(false);
    let can_debark = arrival
        .map(|arrival| read_status(arrival).context("StopTime has a bad arrival status."))
        .transpose()?
        .unwrap_or(false);

    let flow_direction = match (can_board, can_debark) {
        (true, true) => FlowDirection::BoardAndDebark,
        (true, false) => FlowDirection::BoardOnly,
        (false, true) => FlowDirection::DebarkOnly,
        (false, false) => FlowDirection::NoBoardDebark,
    };

    let stop_id = if let Some(stop_id) = &proto.stop_id {
        strip_id_prefix(stop_id, PREFIX_ID_STOP_POINT).to_string()
    } else {
        bail!("StopTime does not have a stop_id.");
    };

    let stop_time = kirin_disruption::StopTime {
        stop_id,
        arrival_time,
        departure_time,
        flow_direction,
    };

    Ok(stop_time)
}

fn read_time(
    proto: &chaos_proto::gtfs_realtime::trip_update::StopTimeEvent,
    reference_date: NaiveDate,
) -> Result<SecondsSinceTimezonedDayStart, Error> {
    // this is a unix timestamp
    let time_i64 = proto
        .time
        .ok_or_else(|| format_err!("'StopTimeEvent' has no 'time'"))?;
    let naive_datetime = NaiveDateTime::from_timestamp_opt(time_i64, 0).ok_or_else(|| {
        format_err!(
            "Could not parse the time value {} as a unix timestamp.",
            time_i64,
        )
    })?;

    let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // 00:00:00 is a valid time

    let reference_date_at_midnight = reference_date.and_time(midnight);
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
    proto: &chaos_proto::gtfs_realtime::trip_update::StopTimeEvent,
) -> Result<bool, Error> {
    use chaos_proto::kirin::StopTimeEventStatus::*;
    if let Some(stop_time_event_status) =
        chaos_proto::kirin::exts::stop_time_event_status.get(proto)
    {
        let stop_time_event_status = stop_time_event_status
            .enum_value()
            .map_err(|value| format_err!("'{}' is not a valid 'StopTimeEventStatus'", value))?;
        match stop_time_event_status {
            SCHEDULED | ADDED | ADDED_FOR_DETOUR => Ok(true),

            DELETED_FOR_DETOUR | DELETED => Ok(false),

            NO_DATA => bail!("No_data in stop time event status."),
        }
    } else {
        Ok(false)
    }
}
