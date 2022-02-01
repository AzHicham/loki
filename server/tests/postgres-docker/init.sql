
-- We create tables needed by choas to store disruptions

create type impact_status as enum ('published', 'archived');
create type severity_effect as enum ('no_service', 'reduced_service', 'significant_delays', 'detour', 'additional_service', 'modified_service', 'other_effect', 'unknown_effect', 'stop_moved');
create type channel_type_enum as enum ('web', 'sms', 'email', 'mobile', 'notification', 'twitter', 'facebook', 'title', 'beacon');
create type disruption_status as enum ('published', 'archived', 'draft');
create type status as enum ('waiting', 'handling', 'error', 'done');
create type pt_object_type as enum ('network', 'stop_area', 'line', 'line_section', 'route', 'stop_point', 'rail_section');
create type disruption_type_enum as enum ('unexpected');

create table alembic_version
  (
      version_num varchar(32) not null
  );
create table pt_object
(
    created_at timestamp not null,
    updated_at timestamp,
    id         uuid      not null
        primary key,
    type       pt_object_type,
    uri        text
);
create index ix_pt_object_type on pt_object (type);
create index pt_object_uri_idx on pt_object (uri);
create table line_section
(
    created_at      timestamp,
    updated_at      timestamp,
    id              uuid not null
        primary key,
    line_object_id  uuid not null
        references pt_object,
    start_object_id uuid not null
        references pt_object,
    end_object_id   uuid not null
        references pt_object,
    object_id       uuid
        references pt_object
);
create index ix_line_section_object_id on line_section (object_id);
create table associate_line_section_route_object
(
    line_section_id uuid not null
        references line_section,
    route_object_id uuid not null
        references pt_object,
    constraint line_section_route_object_pk
        primary key (line_section_id, route_object_id)
);
create table client
(
    created_at  timestamp not null,
    updated_at  timestamp,
    id          uuid      not null
        primary key,
    client_code text      not null
        unique
);
create table severity
(
    id         uuid              not null
        primary key,
    created_at timestamp         not null,
    updated_at timestamp,
    wording    text              not null,
    color      text,
    is_visible boolean           not null,
    effect     severity_effect,
    priority   integer default 0 not null,
    client_id  uuid              not null
        references client
);
create table cause
(
    id          uuid      not null
        primary key,
    created_at  timestamp not null,
    updated_at  timestamp,
    wording     text      not null,
    is_visible  boolean   not null,
    client_id   uuid      not null
        references client,
    category_id uuid
);
create index cause_category_idx on cause (category_id);
create table channel
(
    created_at   timestamp             not null,
    updated_at   timestamp,
    id           uuid                  not null
        primary key,
    name         text                  not null,
    max_size     integer,
    content_type text,
    is_visible   boolean               not null,
    client_id    uuid                  not null
        references client,
    required     boolean default false not null
);
create index channel_name_idx on channel (name);
create table tag
(
    created_at timestamp not null,
    updated_at timestamp,
    id         uuid      not null
        primary key,
    name       text      not null,
    is_visible boolean   not null,
    client_id  uuid      not null
        references client,
    unique (name, client_id)
);
create index client_client_code_idx
    on client (client_code);
create table contributor
(
    created_at       timestamp not null,
    updated_at       timestamp,
    id               uuid      not null
        primary key,
    contributor_code text      not null
        unique
);
create table disruption
(
    created_at             timestamp                                                not null,
    updated_at             timestamp,
    id                     uuid                                                     not null
        primary key,
    reference              text,
    note                   text,
    status                 disruption_status default 'published'::disruption_status not null,
    end_publication_date   timestamp,
    start_publication_date timestamp,
    cause_id               uuid
        references cause,
    client_id              uuid                                                     not null
        references client,
    contributor_id         uuid                                                     not null
        references contributor,
    version                integer           default 1                              not null,
    author                 text,
    type                   disruption_type_enum
);
create index ix_disruption_status on disruption (status);
create index ix_disruption_contrib_status on disruption (contributor_id, status);
create index disruption_created_at_idx on disruption (created_at);
create index disruption_client_id_idx on disruption (client_id);
create table impact
(
    created_at         timestamp                                        not null,
    updated_at         timestamp,
    id                 uuid                                             not null
        primary key,
    disruption_id      uuid
        references disruption,
    status             impact_status default 'published'::impact_status not null,
    severity_id        uuid
        references severity,
    send_notifications boolean       default false                      not null,
    version            integer       default 1                          not null,
    notification_date  timestamp
);
create index ix_impact_status on impact (status);
create index ix_impact_disruption_status on impact (disruption_id, status);
create index testvle_ix_impact_severity_id on impact (severity_id);
create index test_ix_impact_disruption on impact (disruption_id);
create table application_periods
(
    created_at timestamp not null,
    updated_at timestamp,
    id         uuid      not null
        primary key,
    start_date timestamp,
    end_date   timestamp,
    impact_id  uuid      not null
        references impact
);
create index ix_application_periods_impact_id on application_periods (impact_id);
create index applicationperiods_start_date_idx on application_periods (start_date);
create index applicationperiods_end_date_idx on application_periods (end_date);
create table message
(
    created_at timestamp not null,
    updated_at timestamp,
    id         uuid      not null
        primary key,
    text       text      not null,
    impact_id  uuid
        references impact,
    channel_id uuid
        references channel,
    constraint impact_channel_id
        unique (impact_id, channel_id)
);
create index ix_message_channel_impact_id on message (channel_id, impact_id);
create index message_channel_id_idx on message (channel_id);
create table associate_disruption_tag
(
    tag_id        uuid not null
        references tag,
    disruption_id uuid not null
        references disruption,
    constraint tag_disruption_pk
        primary key (tag_id, disruption_id)
);
create table associate_impact_pt_object
(
    impact_id    uuid not null
        references impact,
    pt_object_id uuid not null
        references pt_object,
    constraint impact_pt_object_pk
        primary key (impact_id, pt_object_id)
);
create index associate_impact_pt_object_impact_id_idx on associate_impact_pt_object (impact_id);
create table associate_disruption_pt_object
(
    disruption_id uuid not null
        references disruption,
    pt_object_id  uuid not null
        references pt_object,
    constraint disruption_pt_object_pk
        primary key (disruption_id, pt_object_id)
);
create table category
(
    created_at timestamp not null,
    updated_at timestamp,
    id         uuid      not null
        primary key,
    name       text      not null,
    is_visible boolean   not null,
    client_id  uuid      not null
        references client,
    unique (name, client_id)
);
create index category_name_idx on category (name);
create table wording
(
    created_at timestamp not null,
    updated_at timestamp,
    id         uuid      not null
        primary key,
    key        text      not null,
    value      text      not null
);
create index wording_key_idx on wording (key);
create table associate_wording_cause
(
    wording_id uuid not null
        references wording,
    cause_id   uuid not null
        references cause,
    constraint wording_cause_pk
        primary key (wording_id, cause_id)
);
create index associate_wording_cause_wording_id_idx on associate_wording_cause (wording_id);
create index associate_wording_cause_cause_id_idx on associate_wording_cause (cause_id);
create table associate_wording_severity
(
    wording_id  uuid not null
        references wording,
    severity_id uuid not null
        references severity,
    constraint wording_severity_pk
        primary key (wording_id, severity_id)
);
create table pattern
(
    created_at     timestamp not null,
    updated_at     timestamp,
    id             uuid      not null
        primary key,
    start_date     date,
    end_date       date,
    weekly_pattern bit(7)    not null,
    impact_id      uuid
        references impact,
    timezone       varchar(255)
);
create index ix_pattern_impact_id on pattern (impact_id);
create table time_slot
(
    created_at timestamp not null,
    updated_at timestamp,
    id         uuid      not null
        primary key,
    begin      time,
    "end"      time,
    pattern_id uuid
        references pattern
);
create index ix_time_slot_pattern_id on time_slot (pattern_id);
create table channel_type
(
    created_at timestamp                                          not null,
    updated_at timestamp,
    id         uuid                                               not null
        primary key,
    channel_id uuid
        references channel,
    name       channel_type_enum default 'web'::channel_type_enum not null
);
create index ix_channel_type_name on channel_type (name);
create table associate_wording_line_section
(
    wording_id      uuid not null
        references wording,
    line_section_id uuid not null
        references line_section,
    constraint wording_line_section_pk
        primary key (wording_id, line_section_id)
);
create table property
(
    created_at timestamp not null,
    updated_at timestamp,
    id         uuid      not null
        primary key,
    client_id  uuid      not null
        references client,
    key        text      not null,
    type       text      not null,
    constraint property_type_key_client_id_uc
        unique (type, key, client_id)
);
create table associate_disruption_property
(
    value         text not null,
    disruption_id uuid not null
        references disruption,
    property_id   uuid not null
        references property,
    primary key (disruption_id, property_id, value)
);
create table meta
(
    id    uuid not null
        constraint meta_pk
            primary key,
    key   text not null,
    value text not null
);
create table associate_message_meta
(
    message_id uuid not null
        references message,
    meta_id    uuid not null
        references meta,
    constraint message_meta_pk
        primary key (message_id, meta_id)
);
create table export
(
    id                 uuid                             not null
        primary key,
    client_id          uuid                             not null
        references client,
    created_at         timestamp                        not null,
    updated_at         timestamp,
    process_start_date timestamp,
    start_date         timestamp                        not null,
    end_date           timestamp                        not null,
    file_path          text,
    status             status default 'waiting'::status not null,
    time_zone          text   default 'UTC'::text       not null
);
create table rail_section
(
    id                 uuid      not null
        primary key,
    created_at         timestamp not null,
    updated_at         timestamp,
    line_object_id     uuid
        references pt_object,
    start_object_id    uuid      not null
        references pt_object,
    end_object_id      uuid      not null
        references pt_object,
    blocked_stop_areas text,
    object_id          uuid
        references pt_object
);
create table associate_rail_section_route_object
(
    rail_section_id uuid not null
        references rail_section,
    route_object_id uuid not null
        references pt_object,
    constraint rail_section_route_object_pk
        primary key (rail_section_id, route_object_id)
);

-- End of choas tables creation

-------------------------------------
-- Now we are creating a disruption
-- The order of inserts is important, because some Uuid must be created and then used as reference in other tables

-- Insert a client code needed as reference for other tables (disruption, cause, category tables)
INSERT INTO public.client (created_at, updated_at, id, client_code)
VALUES ('2021-02-22 13:18:34.000000', null, 'cccccccc-cccc-cccc-cccc-cccccccccccc', 'tours');


-- Insert a category named 'Cat name' with Uuid '771e17d6-7510-11eb-81a1-005056a40962'
INSERT INTO public.category (created_at, updated_at, id, name, is_visible, client_id)
VALUES ('2021-02-22 13:18:34.000000', null, '771e17d6-7510-11eb-81a1-005056a40962',
        'Cat name', true, 'cccccccc-cccc-cccc-cccc-cccccccccccc');

-- Insert a cause with wording 'cause_wording' and Uuid 'cccccccc-0000-0000-0000-cccccccccccc'
INSERT INTO public.cause (id, created_at, updated_at, wording, is_visible, client_id, category_id)
VALUES ('cccccccc-0000-0000-0000-cccccccccccc', '2016-05-31 09:12:31.568992', '2017-02-15 14:12:22.575422',
        'cause_wording', true, 'cccccccc-cccc-cccc-cccc-cccccccccccc', '771e17d6-7510-11eb-81a1-005056a40962');

-- Insert a contributor with code 'test_realtime_topic' and Uuid 'dabce516-76e7-11e9-a489-005056a40962'
-- the code is IMPORTANT because it's the rt_topic used in our realtime_tests
INSERT INTO public.contributor (created_at, updated_at, id, contributor_code)
VALUES ('2019-05-15 08:02:59.000000', null, 'dabce516-76e7-11e9-a489-005056a40962', 'test_realtime_topic');

-- Insert a disruption with Uuid 'dddddddd-dddd-dddd-dddd-dddddddddddd'
-- with publication_period : ['2021-01-01 08:00:00.000000', '2021-01-31 08:00:00.000000']
INSERT INTO public.disruption (created_at, updated_at, id, reference, note, status, end_publication_date,
                               start_publication_date, cause_id, client_id, contributor_id, version, author, type)
VALUES ('2021-01-01 08:00:00.000000', '2021-01-01 08:00:00.000000', 'dddddddd-dddd-dddd-dddd-dddddddddddd', 'test', null, 'published',
        '2021-01-31 08:00:00.000000', '2021-01-01 08:00:00.000000', 'cccccccc-0000-0000-0000-cccccccccccc',
        'cccccccc-cccc-cccc-cccc-cccccccccccc', 'dabce516-76e7-11e9-a489-005056a40962', 1, null, null);

-- Insert a tag named 'prolongation'
INSERT INTO public.tag (created_at, updated_at, id, name, is_visible, client_id)
VALUES ('2021-01-24 15:25:00.000000', null, '53207848-5e58-11eb-bc4d-005056a40962', 'prolongation',
        true, 'cccccccc-cccc-cccc-cccc-cccccccccccc');

-- Insert a link between previously inserted tag & disruption
INSERT INTO public.associate_disruption_tag (tag_id, disruption_id)
VALUES ('53207848-5e58-11eb-bc4d-005056a40962', 'dddddddd-dddd-dddd-dddd-dddddddddddd');

-- Insert a severity with :
-- wording : 'accident'
-- color : '#99DD66'
-- effect : 'no_service'
-- priority : 4
INSERT INTO public.severity (id, created_at, updated_at, wording, color, is_visible, effect, priority, client_id)
VALUES ('d94ba49e-5ec1-11e4-8d47-005056a40962', '2014-10-28 16:45:39.528055', '2015-10-21 13:32:34.591280', 'accident',
        '#99DD66', false, 'no_service', 4, 'cccccccc-cccc-cccc-cccc-cccccccccccc');

-- Insert an impact linked to previously inserted disruption and Uuid 'ffffffff-ffff-ffff-ffff-ffffffffffff'
-- and with last update time : '2018-08-28 15:50:08.000000'
INSERT INTO public.impact (created_at, updated_at, id, disruption_id, status, severity_id, send_notifications, version, notification_date)
VALUES ('2018-08-28 15:45:08.000000', '2018-08-28 15:50:08.000000', 'ffffffff-ffff-ffff-ffff-ffffffffffff',
        'dddddddd-dddd-dddd-dddd-dddddddddddd', 'published', 'd94ba49e-5ec1-11e4-8d47-005056a40962', true, 1, '2018-08-03 16:28:37.000000');

-- Insert an application_periods linked to previously inserted impact
-- application_periods : [2021-01-01 14:00:00.000000, 2021-01-02 22:00:00.000000]
INSERT INTO public.application_periods (created_at, updated_at, id, start_date, end_date, impact_id)
VALUES ('2021-01-01 14:00:00.000000', null, '849813dc-5682-11eb-b8c6-005056a40962',
        '2021-01-01 14:00:00.000000', '2021-01-02 22:00:00.000000', 'ffffffff-ffff-ffff-ffff-ffffffffffff');

-- Insert affected pt_object :
-- A line with id 'line:rer_c'
INSERT INTO public.pt_object (created_at, updated_at, id, type, uri)
VALUES ('2014-07-22 06:27:18.771922', null, 'ffffffff-1169-11e4-a924-005056a40962', 'line', 'line:rer_c');

-- Insert link between previously inserted affected pt_object and impact
INSERT INTO public.associate_impact_pt_object (impact_id, pt_object_id)
VALUES ('ffffffff-ffff-ffff-ffff-ffffffffffff', 'ffffffff-1169-11e4-a924-005056a40962');

-- Insert a channel with :
-- name: 'web et mobile
-- content_type: 'text/html'
INSERT INTO public.channel (created_at, updated_at, id, name, max_size, content_type, is_visible, client_id, required)
VALUES ('2015-09-29 11:34:05.026707', null, 'fd4cec38-669d-11e5-b2c1-005056a40962', 'web et mobile', 5000,
        'text/html', true, 'cccccccc-cccc-cccc-cccc-cccccccccccc', false);

-- Insert a channel_type named 'web' linked to previously created channel
INSERT INTO public.channel_type (created_at, updated_at, id, channel_id, name)
VALUES ('2015-08-13 08:06:49.641730', null, '3fd1e558-4192-11e5-bd14-005056a40962', 'fd4cec38-669d-11e5-b2c1-005056a40962', 'web');

-- Insert a message linked to previously created impact and channel
-- and with test: 'Test Message'
INSERT INTO public.message (created_at, updated_at, id, text, impact_id, channel_id)
VALUES ('2021-01-14 16:06:53.000000', null, '8498f3a6-5682-11eb-b8c6-005056a40962', 'Test Message',
        'ffffffff-ffff-ffff-ffff-ffffffffffff', 'fd4cec38-669d-11e5-b2c1-005056a40962');

-- Insert a disruption.property with type: 'Property Test'
INSERT INTO public.property (created_at, updated_at, id, client_id, key, type)
VALUES ('2018-03-29 11:33:31.247305', null, '01e6a074-3345-11e8-82eb-005056a40962',
        'cccccccc-cccc-cccc-cccc-cccccccccccc', 'ccb9e71f-619c-4972-97cd-ae506d31852d', 'Property Test');

-- Insert link between previously created disruption.property and disruption
INSERT INTO public.associate_disruption_property (value, disruption_id, property_id)
VALUES ('property value test', 'dddddddd-dddd-dddd-dddd-dddddddddddd', '01e6a074-3345-11e8-82eb-005056a40962');

-- Insert a disruption.property linked to impact with :
-- period: ['2021-01-01', '2021-08-31']
-- weekly pattern: '1111100'
INSERT INTO public.pattern (created_at, updated_at, id, start_date, end_date, weekly_pattern, impact_id, timezone)
VALUES ('2021-01-01 14:00:00.000000', null, '9742a3f8-8012-11eb-906d-005056a40962', '2021-01-01', '2021-01-02',
        B'1101100', 'ffffffff-ffff-ffff-ffff-ffffffffffff', 'Europe/Paris');

-- Insert a time_slot linked to pattern with :
-- time_period: ['09:00:00', '12:00:00']
INSERT INTO public.time_slot (created_at, updated_at, id, begin, "end", pattern_id)
VALUES ('2021-03-08 13:31:30.000000', null, '9742bec4-8012-11eb-906d-005056a40962',
        '14:00:00', '22:00:00', '9742a3f8-8012-11eb-906d-005056a40962');
