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

use chaos_proto::{chaos::exts, gtfs_realtime as gtfs_proto};
use launch::loki::{
    chrono::{NaiveDate, Utc},
    models::real_time_disruption::TimePeriod,
    NaiveDateTime,
};
use protobuf::Message;

#[derive(Debug)]
enum PtObject<'a> {
    Network(&'a str),
    Line(&'a str),
    Route(&'a str),
    Trip(&'a str),
}

// try to remove all vehicle of a network
// but on a period that don't intersect with calendar validity_period
pub async fn delete_network_on_invalid_period_test(config: &ServerConfig) {
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

    // let's delete all Trip of "my_network" Network
    // between 2021-02-01 and 2021-02-01
    let dt_period = TimePeriod::new(
        NaiveDateTime::parse_from_str("20210201T000000", "%Y%m%dT%H%M%S").unwrap(),
        NaiveDateTime::parse_from_str("20210201T230000", "%Y%m%dT%H%M%S").unwrap(),
    )
    .unwrap();
    let send_realtime_message_datetime = Utc::now().naive_utc();
    let realtime_message =
        create_no_service_disruption(&PtObject::Network("my_network"), &dt_period);
    crate::send_realtime_message_and_wait_until_reception(config, realtime_message).await;

    // wait until realtime message is taken into account
    crate::wait_until_realtime_updated_after(
        &config.requests_socket,
        &send_realtime_message_datetime,
    )
    .await;

    // let's make the same request, but on the realtime level
    // we should get a journey in the response
    // because the disruption previously sent had no effect
    // due to application period
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
        assert_eq!(journeys_response.journeys.len(), 1);
    }
}

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
    let dt_period = TimePeriod::new(
        NaiveDateTime::parse_from_str("20210101T000000", "%Y%m%dT%H%M%S").unwrap(),
        NaiveDateTime::parse_from_str("20210101T230000", "%Y%m%dT%H%M%S").unwrap(),
    )
    .unwrap();
    let send_realtime_message_datetime = Utc::now().naive_utc();
    let realtime_message = create_no_service_disruption(&PtObject::Trip("matin"), &dt_period);
    crate::send_realtime_message_and_wait_until_reception(config, realtime_message).await;

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
    pt_object: &PtObject,
    application_period: &TimePeriod,
) -> gtfs_proto::FeedMessage {
    let id = "baa0eefe-0340-41e1-a2a9-5a660755d54c".to_string();

    let mut entity = chaos_proto::chaos::PtObject::new();
    match pt_object {
        PtObject::Network(id) => {
            entity.set_pt_object_type(chaos_proto::chaos::PtObject_Type::network);
            entity.set_uri(id.to_string());
        }
        PtObject::Route(id) => {
            entity.set_pt_object_type(chaos_proto::chaos::PtObject_Type::route);
            entity.set_uri(id.to_string());
        }
        PtObject::Line(id) => {
            entity.set_pt_object_type(chaos_proto::chaos::PtObject_Type::line);
            entity.set_uri(id.to_string());
        }
        PtObject::Trip(id) => {
            entity.set_pt_object_type(chaos_proto::chaos::PtObject_Type::trip);
            entity.set_uri(id.to_string());
        }
    }

    let mut period = gtfs_proto::TimeRange::default();
    period.set_start(application_period.start().timestamp() as u64);
    period.set_end(application_period.end().timestamp() as u64);

    let mut channel = chaos_proto::chaos::Channel::default();
    channel.set_id("disruption test sample".to_string());
    channel.set_name("web".to_string());
    channel.set_content_type("html".to_string());
    channel.set_max_size(250);
    channel
        .mut_types()
        .push(chaos_proto::chaos::Channel_Type::web);

    let mut message = chaos_proto::chaos::Message::default();
    message.set_text("disruption test sample".to_string());
    message.set_channel(channel);

    let mut severity = chaos_proto::chaos::Severity::default();
    severity.set_id("severity id for NO_SERVICE".to_string());
    severity.set_wording("severity wording for NO_SERVICE".to_string());
    severity.set_color("#FF0000".to_string());
    severity.set_priority(10);
    severity.set_effect(gtfs_proto::Alert_Effect::NO_SERVICE);

    let mut impact = chaos_proto::chaos::Impact::default();
    impact.set_id(format!("impact_{}", id));
    impact.set_created_at(Utc::now().timestamp() as u64);
    impact.set_updated_at(Utc::now().timestamp() as u64);
    impact.mut_informed_entities().push(entity);
    impact.mut_application_periods().push(period.clone());
    impact.mut_messages().push(message);
    impact.set_severity(severity);

    let mut cause = chaos_proto::chaos::Cause::default();
    cause.set_id("disruption cause test".to_string());
    cause.set_wording("disruption cause test".to_string());

    let mut disruption = chaos_proto::chaos::Disruption::default();
    disruption.set_id(id.clone());
    disruption.set_reference("ChaosDisruptionTest".to_string());
    disruption.set_publication_period(period.clone());
    disruption.set_cause(cause);
    disruption.mut_impacts().push(impact);

    // put the update in a feed_entity
    let mut feed_entity = gtfs_proto::FeedEntity::new();
    feed_entity.set_id(id);
    let field_number = exts::disruption.field_number;
    let vec: Vec<u8> = disruption.write_to_bytes().expect("cannot write message");
    feed_entity
        .mut_unknown_fields()
        .add_length_delimited(field_number, vec);

    let mut feed_header = gtfs_proto::FeedHeader::new();
    feed_header.set_gtfs_realtime_version("1.0".to_string());
    let timestamp = NaiveDate::from_ymd(2022, 1, 1)
        .and_hms(12, 0, 0)
        .timestamp();
    feed_header.set_timestamp(u64::try_from(timestamp).unwrap());

    let mut feed_message = gtfs_proto::FeedMessage::new();
    feed_message.mut_entity().push(feed_entity);
    feed_message.set_header(feed_header);

    feed_message
}
