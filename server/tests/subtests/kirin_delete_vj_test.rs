// Copyright  (C) 2021, Kisio Digital and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Kisio Digital (www.kisio.com).
// Help us simplify mobility and open public transport:
// a non ending quest to the responsive locomotion way of traveling!
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

pub use loki_server;
use loki_server::{chaos_proto, navitia_proto, server_config::ServerConfig};

use chaos_proto::gtfs_realtime as kirin_proto;
use launch::loki::{chrono::NaiveDate, NaiveDateTime};
use protobuf::Message;

pub async fn remove_add_modify_vj_test(config: &ServerConfig) {
    // the ntfs (in tests/a_small_ntfs) contains just one trip
    // with a vehicle_journey named "matin"
    // dparting from "massy" at 8h and arriving to "paris" at 9h
    // on day 2021-01-01
    let date = NaiveDate::from_ymd(2021, 1, 1);
    let request_datetime = date.and_hms(8, 0, 0);

    // initial request, on base schedule
    let base_request =
        crate::make_journeys_request("stop_point:massy", "stop_point:paris", request_datetime);

    // same request, but on the realtime level
    let realtime_request = {
        let mut request = base_request.clone();
        request
            .journeys
            .as_mut()
            .unwrap()
            .set_realtime_level(navitia_proto::RtLevel::Realtime);
        request
    };

    // let's first check that we do get a response
    {
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            base_request.clone(),
        )
        .await;
        // info!("{:#?}", journeys_response);
        // check that we have a journey, that uses the only trip in the ntfs
        assert_eq!(
            first_section_vj_name(&journeys_response.journeys[0]),
            "vehicle_journey:matin"
        );

        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            realtime_request.clone(),
        )
        .await;
        // info!("{:#?}", journeys_response);
        // check that we have a journey, that uses the only trip in the ntfs
        assert_eq!(
            first_section_vj_name(&journeys_response.journeys[0]),
            "vehicle_journey:matin"
        );
    }

    // first, we modify the trip
    {
        let realtime_message = create_disruption(
            "matin",
            date,
            vec![
                ("massy", date.and_hms(9, 0, 0)),
                ("paris", date.and_hms(10, 0, 0)),
            ],
            kirin_proto::Alert_Effect::MODIFIED_SERVICE,
        );
        crate::send_realtime_message_and_wait_until_reception(config, realtime_message).await;

        // on base schedule, the trip arrive at 9h
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            base_request.clone(),
        )
        .await;
        assert_eq!(
            arrival_time(&journeys_response.journeys[0]),
            date.and_hms(9, 0, 0)
        );

        // on the realtime level, the trip should now arrive at 10h
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            realtime_request.clone(),
        )
        .await;
        assert_eq!(
            arrival_time(&journeys_response.journeys[0]),
            date.and_hms(10, 0, 0)
        );
    }

    // let's delete the trip
    {
        let realtime_message = create_no_service_disruption("matin", date);
        crate::send_realtime_message_and_wait_until_reception(config, realtime_message).await;

        // on the realtime level, we should get no journey in the response
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            realtime_request.clone(),
        )
        .await;
        assert_eq!(journeys_response.journeys.len(), 0);

        // with the same request on the 'base schedule' level
        // we should get a journey in the response
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            base_request.clone(),
        )
        .await;
        assert_eq!(
            first_section_vj_name(&journeys_response.journeys[0]),
            "vehicle_journey:matin"
        );
    }

    // let's now 'add' the removed trip with flag ADDITIONAL_SERVICE
    // this should not add anything, since the vehicle exists in base schedule
    {
        let realtime_message = create_disruption(
            "matin",
            date,
            vec![
                ("massy", date.and_hms(12, 0, 0)),
                ("paris", date.and_hms(13, 0, 0)),
            ],
            kirin_proto::Alert_Effect::ADDITIONAL_SERVICE,
        );
        crate::send_realtime_message_and_wait_until_reception(config, realtime_message).await;

        // since nothing should be added, we should get
        // no journey for the request on the realtime level
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            realtime_request.clone(),
        )
        .await;
        assert_eq!(journeys_response.journeys.len(), 0);
    }

    // let's now add a new vehicle named 'midi'
    {
        let realtime_message = create_disruption(
            "midi",
            date,
            vec![
                ("massy", date.and_hms(12, 0, 0)),
                ("paris", date.and_hms(13, 0, 0)),
            ],
            kirin_proto::Alert_Effect::ADDITIONAL_SERVICE,
        );
        crate::send_realtime_message_and_wait_until_reception(config, realtime_message).await;

        // this new "midi" vehicle should be used on the realtime level
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            realtime_request.clone(),
        )
        .await;
        assert_eq!(
            first_section_vj_name(&journeys_response.journeys[0]),
            "vehicle_journey:midi"
        );

        // with the same request on the 'base schedule' level
        // we should still use the "matin" vehicle
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            base_request.clone(),
        )
        .await;
        assert_eq!(
            first_section_vj_name(&journeys_response.journeys[0]),
            "vehicle_journey:matin"
        );
    }

    // let's now modify the real time vehicle 'midi'
    // since this is a real time vehicle, we should be able to modify it
    // with the ADDITIONAL_SERVICE effect
    {
        let realtime_message = create_disruption(
            "midi",
            date,
            vec![
                ("massy", date.and_hms(13, 0, 0)),
                ("paris", date.and_hms(14, 0, 0)),
            ],
            kirin_proto::Alert_Effect::ADDITIONAL_SERVICE,
        );
        crate::send_realtime_message_and_wait_until_reception(config, realtime_message).await;

        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            realtime_request.clone(),
        )
        .await;
        assert_eq!(
            arrival_time(&journeys_response.journeys[0]),
            date.and_hms(14, 0, 0)
        );
    }

    // we can also modify the real time vehicle "midi"
    // with the MODIFIED_SERVICE effect
    {
        let realtime_message = create_disruption(
            "midi",
            date,
            vec![
                ("massy", date.and_hms(13, 0, 0)),
                ("paris", date.and_hms(15, 0, 0)),
            ],
            kirin_proto::Alert_Effect::MODIFIED_SERVICE,
        );
        crate::send_realtime_message_and_wait_until_reception(config, realtime_message).await;

        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            realtime_request.clone(),
        )
        .await;
        assert_eq!(
            arrival_time(&journeys_response.journeys[0]),
            date.and_hms(15, 0, 0)
        );
    }
}

fn create_no_service_disruption(
    vehicle_journey_id: &str,
    date: NaiveDate,
) -> kirin_proto::FeedMessage {
    create_disruption_inner(
        vehicle_journey_id,
        date,
        kirin_proto::Alert_Effect::NO_SERVICE,
        Vec::new(),
    )
}

fn create_disruption(
    vehicle_journey_id: &str,
    date: NaiveDate,
    stop_times: Vec<(&str, NaiveDateTime)>,
    effect: kirin_proto::Alert_Effect,
) -> kirin_proto::FeedMessage {
    let stop_time_event_status = chaos_proto::kirin::StopTimeEventStatus::SCHEDULED;
    let stop_time_updates: Vec<_> = stop_times
        .into_iter()
        .map(|(stop_name, time)| {
            let field_number = chaos_proto::kirin::exts::stop_time_event_status.field_number;
            let mut stop_time_event = kirin_proto::TripUpdate_StopTimeEvent::default();
            stop_time_event.set_time(time.timestamp());
            stop_time_event
                .mut_unknown_fields()
                .add_varint(field_number, stop_time_event_status as u64);

            let mut trip_update = kirin_proto::TripUpdate_StopTimeUpdate::default();
            trip_update.set_stop_id(stop_name.to_string());
            trip_update.set_arrival(stop_time_event.clone());
            trip_update.set_departure(stop_time_event);
            trip_update
        })
        .collect();

    create_disruption_inner(vehicle_journey_id, date, effect, stop_time_updates)
}

fn create_disruption_inner(
    vehicle_journey_id: &str,
    date: NaiveDate,
    effect: kirin_proto::Alert_Effect,
    stop_times: Vec<kirin_proto::TripUpdate_StopTimeUpdate>,
) -> kirin_proto::FeedMessage {
    use protobuf::ProtobufEnum;

    let mut trip_update = kirin_proto::TripUpdate::default();

    // set the "effect" field to NO_SERVICE
    let field_number = chaos_proto::kirin::exts::effect.field_number;
    trip_update
        .mut_unknown_fields()
        //.add_fixed32(field_number, effect.value() as u32);
        //.add_fixed64(field_number, effect.value() as u64);
        .add_varint(field_number, effect.value() as u64);

    trip_update.stop_time_update.extend(stop_times.into_iter());

    // set trip_description
    let mut trip_descriptor = kirin_proto::TripDescriptor::default();
    trip_descriptor.set_trip_id(vehicle_journey_id.to_string());
    trip_descriptor.set_start_date(date.format("%Y%m%d").to_string());
    trip_update.set_trip(trip_descriptor);

    // put the update in a feed_entity
    let mut feed_entity = kirin_proto::FeedEntity::default();
    feed_entity.set_id(format!("test_delete_{}_{}", vehicle_journey_id, date));
    feed_entity.set_trip_update(trip_update);

    let mut feed_header = kirin_proto::FeedHeader::new();
    feed_header.set_gtfs_realtime_version("1.0".to_string());

    let mut feed_message = kirin_proto::FeedMessage::new();
    feed_message.mut_entity().push(feed_entity);
    feed_message.set_header(feed_header);

    feed_message
}

fn first_section_vj_name(journey: &navitia_proto::Journey) -> &str {
    journey.sections[0]
        .pt_display_informations
        .as_ref()
        .unwrap()
        .uris
        .as_ref()
        .unwrap()
        .vehicle_journey
        .as_ref()
        .unwrap()
}

fn arrival_time(journey: &navitia_proto::Journey) -> NaiveDateTime {
    let timestamp = journey.arrival_date_time();
    NaiveDateTime::from_timestamp(timestamp as i64, 0)
}
