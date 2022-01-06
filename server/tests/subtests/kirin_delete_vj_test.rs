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
use launch::loki::{
    chrono::{NaiveDate, Utc},
    NaiveDateTime,
};
use protobuf::Message;

pub async fn delete_vj_test(config: &ServerConfig) {
    let datetime =
        NaiveDateTime::parse_from_str("2021-01-01 08:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

    // initial request
    let journey_request =
        crate::make_journeys_request("stop_point:massy", "stop_point:paris", datetime);

    // let's first check that we do get a response
    {
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            journey_request.clone(),
        )
        .await;
        // info!("{:#?}", journeys_response);
        // check that we have a journey, that uses the only trip in the ntfs
        assert_eq!(
            journeys_response.journeys[0].sections[0]
                .pt_display_informations
                .as_ref()
                .unwrap()
                .uris
                .as_ref()
                .unwrap()
                .vehicle_journey
                .as_ref()
                .unwrap(),
            "vehicle_journey:matin"
        );
    }

    // let's delete the only trip
    let send_realtime_message_datetime = Utc::now().naive_utc();
    let realtime_message = create_no_service_disruption(
        "matin",
        NaiveDate::parse_from_str("20210101", "%Y%m%d").unwrap(),
    );
    crate::send_realtime_message(config, realtime_message).await;

    // wait until realtime message is taken into account
    crate::wait_until_realtime_updated_after(
        &config.requests_socket,
        &send_realtime_message_datetime,
    )
    .await;

    // let's make the same request, but on the realtime level
    // we should get no journey in the response
    {
        let mut realtime_request = journey_request.clone();
        realtime_request
            .journeys
            .as_mut()
            .unwrap()
            .set_realtime_level(navitia_proto::RtLevel::Realtime);
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            realtime_request.clone(),
        )
        .await;
        assert_eq!(journeys_response.journeys.len(), 0);
    }
    // with the same request on the 'base schedule' level
    // we should get a journey in the response
    {
        let mut realtime_request = journey_request.clone();
        realtime_request
            .journeys
            .as_mut()
            .unwrap()
            .set_realtime_level(navitia_proto::RtLevel::BaseSchedule);
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            realtime_request.clone(),
        )
        .await;
        assert_eq!(
            journeys_response.journeys[0].sections[0]
                .pt_display_informations
                .as_ref()
                .unwrap()
                .uris
                .as_ref()
                .unwrap()
                .vehicle_journey
                .as_ref()
                .unwrap(),
            "vehicle_journey:matin"
        );
    }
}

fn create_no_service_disruption(
    vehicle_journey_id: &str,
    date: NaiveDate,
) -> kirin_proto::FeedMessage {
    use protobuf::ProtobufEnum;

    let mut trip_update = kirin_proto::TripUpdate::default();

    // set the "effect" field to NO_SERVICE
    let field_number = chaos_proto::kirin::exts::effect.field_number;
    let effect = kirin_proto::Alert_Effect::NO_SERVICE;
    trip_update
        .mut_unknown_fields()
        //.add_fixed32(field_number, effect.value() as u32);
        //.add_fixed64(field_number, effect.value() as u64);
        .add_varint(field_number, effect.value() as u64);

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
