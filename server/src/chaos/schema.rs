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

use diesel::{allow_tables_to_appear_in_same_query, joinable, table};

table! {
    application_periods (id) {
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        id -> Uuid,
        start_date -> Nullable<Timestamp>,
        end_date -> Nullable<Timestamp>,
        impact_id -> Uuid,
    }
}

table! {
    associate_disruption_property (disruption_id, property_id, value) {
        value -> Text,
        disruption_id -> Uuid,
        property_id -> Uuid,
    }
}

table! {
    associate_disruption_pt_object (disruption_id, pt_object_id) {
        disruption_id -> Uuid,
        pt_object_id -> Uuid,
    }
}

table! {
    associate_disruption_tag (tag_id, disruption_id) {
        tag_id -> Uuid,
        disruption_id -> Uuid,
    }
}

table! {
    associate_impact_pt_object (impact_id, pt_object_id) {
        impact_id -> Uuid,
        pt_object_id -> Uuid,
    }
}

table! {
    associate_line_section_route_object (line_section_id, route_object_id) {
        line_section_id -> Uuid,
        route_object_id -> Uuid,
    }
}

table! {
    associate_message_meta (message_id, meta_id) {
        message_id -> Uuid,
        meta_id -> Uuid,
    }
}

table! {
    associate_rail_section_route_object (rail_section_id, route_object_id) {
        rail_section_id -> Uuid,
        route_object_id -> Uuid,
    }
}

table! {
    associate_wording_cause (wording_id, cause_id) {
        wording_id -> Uuid,
        cause_id -> Uuid,
    }
}

table! {
    associate_wording_line_section (wording_id, line_section_id) {
        wording_id -> Uuid,
        line_section_id -> Uuid,
    }
}

table! {
    associate_wording_severity (wording_id, severity_id) {
        wording_id -> Uuid,
        severity_id -> Uuid,
    }
}

table! {
    category (id) {
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        id -> Nullable<Uuid>,
        name -> Text,
        is_visible -> Bool,
        client_id -> Uuid,
    }
}

table! {
    cause (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        wording -> Text,
        is_visible -> Bool,
        client_id -> Uuid,
        category_id -> Nullable<Uuid>,
    }
}

table! {
    channel (id) {
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        id -> Uuid,
        name -> Text,
        max_size -> Nullable<Int4>,
        content_type -> Nullable<Text>,
        is_visible -> Bool,
        client_id -> Uuid,
        required -> Bool,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::chaos::sql_types::channel_type_enum;
    channel_type (id) {
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        id -> Uuid,
        channel_id -> Nullable<Uuid>,
        name -> channel_type_enum,
    }
}

table! {
    client (id) {
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        id -> Uuid,
        client_code -> Text,
    }
}

table! {
    contributor (id) {
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        id -> Uuid,
        contributor_code -> Text,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::chaos::sql_types::{disruption_type_enum, disruption_status};
    disruption (id) {
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        id -> Uuid,
        reference -> Nullable<Text>,
        note -> Nullable<Text>,
        status -> disruption_status,
        end_publication_date -> Nullable<Timestamp>,
        start_publication_date -> Nullable<Timestamp>,
        cause_id -> Nullable<Uuid>,
        client_id -> Uuid,
        contributor_id -> Uuid,
        version -> Int4,
        author -> Nullable<Text>,
        #[sql_name = "type"]
        type_ -> Nullable<disruption_type_enum>,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::chaos::sql_types::Status;
    export (id) {
        id -> Uuid,
        client_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        process_start_date -> Nullable<Timestamp>,
        start_date -> Timestamp,
        end_date -> Timestamp,
        file_path -> Nullable<Text>,
        status -> Status,
        time_zone -> Text,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::chaos::sql_types::impact_status;
    impact (id) {
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        id -> Uuid,
        disruption_id -> Nullable<Uuid>,
        status -> impact_status,
        severity_id -> Nullable<Uuid>,
        send_notifications -> Bool,
        version -> Int4,
        notification_date -> Nullable<Timestamp>,
    }
}

table! {
    line_section (id) {
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        id -> Uuid,
        line_object_id -> Uuid,
        start_object_id -> Uuid,
        end_object_id -> Uuid,
        object_id -> Nullable<Uuid>,
    }
}

table! {
    message (id) {
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        id -> Uuid,
        text -> Text,
        impact_id -> Nullable<Uuid>,
        channel_id -> Nullable<Uuid>,
    }
}

table! {
    meta (id) {
        id -> Uuid,
        key -> Text,
        value -> Text,
    }
}

table! {
    pattern (id) {
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        id -> Uuid,
        start_date -> Nullable<Date>,
        end_date -> Nullable<Date>,
        weekly_pattern -> Bit,
        impact_id -> Nullable<Uuid>,
        timezone -> Nullable<Varchar>,
    }
}

table! {
    property (id) {
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        id -> Uuid,
        client_id -> Uuid,
        key -> Text,
        #[sql_name = "type"]
        type_ -> Text,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::chaos::sql_types::pt_object_type;
    pt_object (id) {
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        id -> Uuid,
        #[sql_name = "type"]
        type_ -> Nullable<pt_object_type>,
        uri -> Nullable<Text>,
    }
}

table! {
    rail_section (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        line_object_id -> Nullable<Uuid>,
        start_object_id -> Uuid,
        end_object_id -> Uuid,
        blocked_stop_areas -> Nullable<Text>,
        object_id -> Nullable<Uuid>,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::chaos::sql_types::severity_effect;
    severity (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        wording -> Text,
        color -> Nullable<Text>,
        is_visible -> Bool,
        effect -> Nullable<severity_effect>,
        priority -> Int4,
        client_id -> Uuid,
    }
}

table! {
    spatial_ref_sys (srid) {
        srid -> Int4,
        auth_name -> Nullable<Varchar>,
        auth_srid -> Nullable<Int4>,
        srtext -> Nullable<Varchar>,
        proj4text -> Nullable<Varchar>,
    }
}

table! {
    tag (id) {
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        id -> Uuid,
        name -> Text,
        is_visible -> Bool,
        client_id -> Uuid,
    }
}

table! {
    time_slot (id) {
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        id -> Uuid,
        begin -> Nullable<Time>,
        end -> Nullable<Time>,
        pattern_id -> Nullable<Uuid>,
    }
}

table! {
    wording (id) {
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        id -> Uuid,
        key -> Text,
        value -> Text,
    }
}

joinable!(application_periods -> impact (impact_id));
joinable!(associate_disruption_property -> disruption (disruption_id));
joinable!(associate_disruption_property -> property (property_id));
joinable!(associate_disruption_pt_object -> disruption (disruption_id));
joinable!(associate_disruption_pt_object -> pt_object (pt_object_id));
joinable!(associate_disruption_tag -> disruption (disruption_id));
joinable!(associate_disruption_tag -> tag (tag_id));
joinable!(associate_impact_pt_object -> impact (impact_id));
joinable!(associate_impact_pt_object -> pt_object (pt_object_id));
joinable!(associate_line_section_route_object -> line_section (line_section_id));
joinable!(associate_line_section_route_object -> pt_object (route_object_id));
joinable!(associate_message_meta -> message (message_id));
joinable!(associate_message_meta -> meta (meta_id));
joinable!(associate_rail_section_route_object -> pt_object (route_object_id));
joinable!(associate_rail_section_route_object -> rail_section (rail_section_id));
joinable!(associate_wording_cause -> cause (cause_id));
joinable!(associate_wording_cause -> wording (wording_id));
joinable!(associate_wording_line_section -> line_section (line_section_id));
joinable!(associate_wording_line_section -> wording (wording_id));
joinable!(associate_wording_severity -> severity (severity_id));
joinable!(associate_wording_severity -> wording (wording_id));
joinable!(category -> client (client_id));
joinable!(cause -> client (client_id));
joinable!(channel -> client (client_id));
joinable!(channel_type -> channel (channel_id));
joinable!(disruption -> cause (cause_id));
joinable!(disruption -> client (client_id));
joinable!(disruption -> contributor (contributor_id));
joinable!(export -> client (client_id));
joinable!(impact -> disruption (disruption_id));
joinable!(impact -> severity (severity_id));
joinable!(message -> channel (channel_id));
joinable!(message -> impact (impact_id));
joinable!(pattern -> impact (impact_id));
joinable!(property -> client (client_id));
joinable!(severity -> client (client_id));
joinable!(tag -> client (client_id));
joinable!(time_slot -> pattern (pattern_id));

allow_tables_to_appear_in_same_query!(
    application_periods,
    associate_disruption_property,
    associate_disruption_pt_object,
    associate_disruption_tag,
    associate_impact_pt_object,
    associate_line_section_route_object,
    associate_message_meta,
    associate_rail_section_route_object,
    associate_wording_cause,
    associate_wording_line_section,
    associate_wording_severity,
    category,
    cause,
    channel,
    channel_type,
    client,
    contributor,
    disruption,
    export,
    impact,
    line_section,
    message,
    meta,
    pattern,
    property,
    pt_object,
    rail_section,
    severity,
    spatial_ref_sys,
    tag,
    time_slot,
    wording,
);
