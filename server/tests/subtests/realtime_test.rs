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

use crate::{arrival_time, first_section_vj_name};

// try to remove/add/modify a vehicle in the base schedule
pub async fn remove_add_modify_base_vj_test(config: &ServerConfig) {
    // the ntfs (in tests/a_small_ntfs) contains just one trip
    // with a vehicle_journey named "matin"
    // departing from "massy" at 8h and arriving to "paris" at 9h
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

    // first, we modify the "matin" trip
    {
        let realtime_message = create_disruption(
            "matin",
            date,
            vec![
                ("massy", date.and_hms(9, 0, 0)),
                ("paris", date.and_hms(10, 0, 0)),
                ("cdg", date.and_hms(10, 30, 0)),
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
        // on base schedule, we expect a linked impact
        // because the journey in response is impacted by a disruption
        assert_eq!(
            journeys_response.impacts[0].uri.as_ref().unwrap(),
            "test_delete_matin_2021-01-01"
        );
        assert_eq!(
            journeys_response.impacts[0].impacted_objects[0]
                .pt_object
                .as_ref()
                .unwrap()
                .uri,
            "matin"
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
        // on the realtime level, an impact should be returned
        // because the vehicle in journey response was created by a disruption
        //assert_eq!(journeys_response.impacts.len(), 1);
    }

    // let's delete the "matin" trip
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
    // this should *not* add anything, since the vehicle does exists in base schedule
    {
        let realtime_message = create_disruption(
            "matin",
            date,
            vec![
                ("massy", date.and_hms(12, 0, 0)),
                ("paris", date.and_hms(13, 0, 0)),
                ("cdg", date.and_hms(13, 30, 0)),
            ],
            kirin_proto::Alert_Effect::ADDITIONAL_SERVICE,
        );
        crate::send_realtime_message_and_wait_until_reception(config, realtime_message).await;

        // since nothing should be added, we should get
        // no journey for the request on the realtime level
        // and no linked impact either
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            realtime_request.clone(),
        )
        .await;
        assert_eq!(journeys_response.journeys.len(), 0);
        assert_eq!(journeys_response.impacts.len(), 0);
    }

    // let's now 'add' the removed base trip "matin" with flag SIGNIFICANT_DELAYS
    // this should add back the trip, since the vehicle exists in base schedule
    {
        let realtime_message = create_disruption(
            "matin",
            date,
            vec![
                ("massy", date.and_hms(8, 0, 0)),
                ("paris", date.and_hms(9, 0, 0)),
                ("cdg", date.and_hms(10, 30, 0)),
            ],
            kirin_proto::Alert_Effect::SIGNIFICANT_DELAYS,
        );
        crate::send_realtime_message_and_wait_until_reception(config, realtime_message).await;

        // since we added the previously removed base vehicle , we should get
        // a journey for the request on the realtime level
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            realtime_request.clone(),
        )
        .await;
        assert_eq!(
            first_section_vj_name(&journeys_response.journeys[0]),
            "vehicle_journey:matin"
        );
    }
}

// try to add/remove/modify a vehicle which is NOT in the base schedule
pub async fn remove_add_modify_new_vj_test(config: &ServerConfig) {
    // the ntfs (in tests/a_small_ntfs) contains just one trip
    // with a vehicle_journey named "matin"
    // departing from "massy" at 8h and arriving to "paris" at 9h
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

    // let's delete the "matin" trip
    {
        let realtime_message = create_no_service_disruption("matin", date);
        crate::send_realtime_message_and_wait_until_reception(config, realtime_message).await;

        // on the realtime level, we should get no journey in the response
        // and no linked_impact because journey_response.journeys[] is empty
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            realtime_request.clone(),
        )
        .await;
        assert_eq!(journeys_response.journeys.len(), 0);

        // with the same request on the 'base schedule' level
        // we should get a journey in the response and a linked impact to previously sent disruption
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            base_request.clone(),
        )
        .await;
        assert_eq!(
            first_section_vj_name(&journeys_response.journeys[0]),
            "vehicle_journey:matin"
        );
        assert_eq!(
            journeys_response.impacts[0].impacted_objects[0]
                .pt_object
                .as_ref()
                .unwrap()
                .uri,
            "matin"
        );
    }

    // let's now add a new vehicle named 'midi'
    {
        let realtime_message = create_disruption(
            "midi",
            date,
            vec![
                ("massy", date.and_hms(12, 0, 0)),
                ("paris", date.and_hms(13, 0, 0)),
                ("cdg", date.and_hms(13, 30, 0)),
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

    // let's now modify the new vehicle 'midi'
    // since this is a new vehicle, we should be able to modify it
    // with the ADDITIONAL_SERVICE effect
    {
        let realtime_message = create_disruption(
            "midi",
            date,
            vec![
                ("massy", date.and_hms(13, 0, 0)),
                ("paris", date.and_hms(14, 0, 0)),
                ("cdg", date.and_hms(14, 30, 0)),
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

    // Since "midi" is a new vehicle, we should
    // *NOT* be able to modify it with the MODIFIED_SERVICE effect
    {
        let realtime_message = create_disruption(
            "midi",
            date,
            vec![
                ("massy", date.and_hms(13, 0, 0)),
                ("paris", date.and_hms(15, 0, 0)),
                ("cdg", date.and_hms(15, 30, 0)),
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
            date.and_hms(14, 0, 0) //same arrival time as before the realtime message
        );
    }

    // let's delete the "midi" trip
    {
        let realtime_message = create_no_service_disruption("midi", date);
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

    // let's now 'add' the removed trip with flag SIGNIFICANT_DELAYS
    // this should *not* add anything, since the vehicle does not exists in base schedule
    {
        let realtime_message = create_disruption(
            "midi",
            date,
            vec![
                ("massy", date.and_hms(12, 0, 0)),
                ("paris", date.and_hms(13, 0, 0)),
                ("cdg", date.and_hms(13, 30, 0)),
            ],
            kirin_proto::Alert_Effect::SIGNIFICANT_DELAYS,
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

    // let's now 'add' the removed base trip "matin" with flag ADDITIONAL_SERVICE
    // this should add back the trip, since the vehicle does not exists in base schedule
    {
        let realtime_message = create_disruption(
            "midi",
            date,
            vec![
                ("massy", date.and_hms(9, 0, 0)),
                ("paris", date.and_hms(10, 0, 0)),
                ("cdg", date.and_hms(11, 30, 0)),
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
        assert_eq!(
            first_section_vj_name(&journeys_response.journeys[0]),
            "vehicle_journey:midi"
        );
    }
}

// try to add/remove/modify a vehicle with and id that exists in
// the base schedule,
// but on a day on which this vehicle is NOT valid
pub async fn remove_add_modify_base_vj_on_invalid_day_test(config: &ServerConfig) {
    // the ntfs (in tests/a_small_ntfs) contains just one trip
    // with a vehicle_journey named "matin"
    // departing from "massy" at 8h and arriving to "paris" at 9h
    // on day 2021-01-01
    // the ntfs also contains the date 2021-01-02
    // on which the vehicle_journey "matin" is NOT valid
    let date = NaiveDate::from_ymd(2021, 1, 2);
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

    // let's first check that we do NOT get a response
    {
        // no response on base schedule
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            base_request.clone(),
        )
        .await;
        assert!(journeys_response.journeys.is_empty());

        // no response on the real time level
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            realtime_request.clone(),
        )
        .await;
        assert!(journeys_response.journeys.is_empty());
    }

    // let's try to add a vehicle named 'matin'
    // on 2021-01-02 (a day on which the base vehicle 'matin' is not valid)
    // with the flag MODIFIED_SERVICE
    // this should NOT add anything, since the vehicle is not present in the base schedule
    {
        let realtime_message = create_disruption(
            "matin",
            date,
            vec![
                ("massy", date.and_hms(12, 0, 0)),
                ("paris", date.and_hms(13, 0, 0)),
                ("cdg", date.and_hms(13, 30, 0)),
            ],
            kirin_proto::Alert_Effect::MODIFIED_SERVICE,
        );
        crate::send_realtime_message_and_wait_until_reception(config, realtime_message).await;

        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            realtime_request.clone(),
        )
        .await;
        assert!(journeys_response.journeys.is_empty());
    }

    // let's now add a vehicle named 'matin'
    // on 2021-01-02 (a day on which the base vehicle 'matin' is not valid)
    // with the flag ADDITIONNAL_SERVICE
    // this should add something, since the vehicle is not present in the base schedule
    {
        let realtime_message = create_disruption(
            "matin",
            date,
            vec![
                ("massy", date.and_hms(12, 0, 0)),
                ("paris", date.and_hms(13, 0, 0)),
                ("cdg", date.and_hms(13, 30, 0)),
            ],
            kirin_proto::Alert_Effect::ADDITIONAL_SERVICE,
        );
        crate::send_realtime_message_and_wait_until_reception(config, realtime_message).await;

        // this "matin" vehicle should be used on the realtime level
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            realtime_request.clone(),
        )
        .await;
        assert_eq!(
            first_section_vj_name(&journeys_response.journeys[0]),
            "vehicle_journey:matin"
        );

        // with the same request on the 'base schedule' level
        // we should still get no response
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            base_request.clone(),
        )
        .await;
        assert!(journeys_response.journeys.is_empty());
    }

    // let's now modify the vehicle 'matin'
    // since this is a new vehicle, we should be able to modify it
    // with the ADDITIONAL_SERVICE effect
    {
        let realtime_message = create_disruption(
            "matin",
            date,
            vec![
                ("massy", date.and_hms(13, 0, 0)),
                ("paris", date.and_hms(14, 0, 0)),
                ("cdg", date.and_hms(14, 30, 0)),
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

    // Since "matin" is a new vehicle, we should
    // *NOT* be able to modify it with the MODIFIED_SERVICE effect
    {
        let realtime_message = create_disruption(
            "matin",
            date,
            vec![
                ("massy", date.and_hms(13, 0, 0)),
                ("paris", date.and_hms(15, 0, 0)),
                ("cdg", date.and_hms(15, 30, 0)),
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
            date.and_hms(14, 0, 0) //same arrival time as before the realtime message
        );
    }

    // let's delete the "matin" trip
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
        // we should also get no journey
        let journeys_response = crate::send_request_and_wait_for_response(
            &config.requests_socket,
            base_request.clone(),
        )
        .await;
        assert_eq!(journeys_response.journeys.len(), 0);
    }

    // let's now 'add' the removed trip with flag SIGNIFICANT_DELAYS
    // this should *not* add anything, since the vehicle does not exists in base schedule
    {
        let realtime_message = create_disruption(
            "matin",
            date,
            vec![
                ("massy", date.and_hms(12, 0, 0)),
                ("paris", date.and_hms(13, 0, 0)),
                ("cdg", date.and_hms(13, 30, 0)),
            ],
            kirin_proto::Alert_Effect::SIGNIFICANT_DELAYS,
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

    // let's now 'add' the removed trip "midi" with flag ADDITIONAL_SERVICE
    // this should add back the trip, since the vehicle does not exists in base schedule
    {
        let realtime_message = create_disruption(
            "matin",
            date,
            vec![
                ("massy", date.and_hms(9, 0, 0)),
                ("paris", date.and_hms(10, 0, 0)),
                ("cdg", date.and_hms(10, 30, 0)),
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
        assert_eq!(
            first_section_vj_name(&journeys_response.journeys[0]),
            "vehicle_journey:matin"
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
    let timestamp = NaiveDate::from_ymd(2022, 1, 1)
        .and_hms(12, 0, 0)
        .timestamp();
    feed_header.set_timestamp(u64::try_from(timestamp).unwrap());

    let mut feed_message = kirin_proto::FeedMessage::new();
    feed_message.mut_entity().push(feed_entity);
    feed_message.set_header(feed_header);

    feed_message
}
