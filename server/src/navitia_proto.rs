
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Code {
    #[prost(string, required, tag="1")]
    pub r#type: std::string::String,
    #[prost(string, required, tag="2")]
    pub value: std::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Period {
    #[prost(uint64, optional, tag="1")]
    pub begin: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="2")]
    pub end: ::std::option::Option<u64>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Property {
    #[prost(string, optional, tag="1")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub value: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Channel {
    #[prost(string, optional, tag="1")]
    pub id: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="3")]
    pub content_type: ::std::option::Option<std::string::String>,
    #[prost(enumeration="channel::ChannelType", repeated, packed="false", tag="4")]
    pub channel_types: ::std::vec::Vec<i32>,
}
pub mod channel {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum ChannelType {
        Web = 1,
        Sms = 2,
        Email = 3,
        Mobile = 4,
        Notification = 5,
        Twitter = 6,
        Facebook = 7,
        UnknownType = 8,
        Title = 9,
        Beacon = 10,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MessageContent {
    #[prost(string, optional, tag="1")]
    pub text: ::std::option::Option<std::string::String>,
    #[prost(message, optional, tag="4")]
    pub channel: ::std::option::Option<Channel>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Message {
    #[prost(string, optional, tag="1")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub message: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="3")]
    pub title: ::std::option::Option<std::string::String>,
    #[prost(uint64, optional, tag="4")]
    pub start_application_date: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="5")]
    pub end_application_date: ::std::option::Option<u64>,
    #[prost(string, optional, tag="6")]
    pub start_application_daily_hour: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="7")]
    pub end_application_daily_hour: ::std::option::Option<std::string::String>,
    #[prost(enumeration="MessageStatus", optional, tag="8")]
    pub message_status: ::std::option::Option<i32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Severity {
    #[prost(string, optional, tag="1")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub color: ::std::option::Option<std::string::String>,
    #[prost(enumeration="severity::Effect", optional, tag="3", default="UnknownEffect")]
    pub effect: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="4")]
    pub priority: ::std::option::Option<i32>,
}
pub mod severity {
    /// copied from chaos-proto/gtfs-realtime.proto
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Effect {
        NoService = 1,
        ReducedService = 2,
        /// We don't care about INsignificant delays: they are hard to detect, have
        /// little impact on the user, and would clutter the results as they are too
        /// frequent.
        SignificantDelays = 3,
        Detour = 4,
        AdditionalService = 5,
        ModifiedService = 6,
        OtherEffect = 7,
        UnknownEffect = 8,
        StopMoved = 9,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StopTimeUpdate {
    #[prost(message, optional, tag="1")]
    pub amended_stop_time: ::std::option::Option<StopTime>,
    #[prost(message, optional, tag="2")]
    pub base_stop_time: ::std::option::Option<StopTime>,
    #[prost(string, optional, tag="3")]
    pub cause: ::std::option::Option<std::string::String>,
    #[prost(enumeration="StopTimeUpdateStatus", optional, tag="4")]
    pub effect: ::std::option::Option<i32>,
    #[prost(message, optional, tag="5")]
    pub stop_point: ::std::option::Option<StopPoint>,
    #[prost(enumeration="StopTimeUpdateStatus", optional, tag="6")]
    pub departure_status: ::std::option::Option<i32>,
    #[prost(enumeration="StopTimeUpdateStatus", optional, tag="7")]
    pub arrival_status: ::std::option::Option<i32>,
    #[prost(bool, optional, tag="8")]
    pub is_detour: ::std::option::Option<bool>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LineSectionImpact {
    #[prost(message, optional, tag="1")]
    pub from: ::std::option::Option<PtObject>,
    #[prost(message, optional, tag="2")]
    pub to: ::std::option::Option<PtObject>,
    #[prost(message, repeated, tag="3")]
    pub routes: ::std::vec::Vec<Route>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ImpactedObject {
    #[prost(message, optional, tag="1")]
    pub pt_object: ::std::option::Option<PtObject>,
    #[prost(message, repeated, tag="2")]
    pub impacted_stops: ::std::vec::Vec<StopTimeUpdate>,
    #[prost(message, optional, tag="3")]
    pub impacted_section: ::std::option::Option<LineSectionImpact>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DisruptionProperty {
    #[prost(string, required, tag="1")]
    pub key: std::string::String,
    #[prost(string, required, tag="2")]
    pub r#type: std::string::String,
    #[prost(string, required, tag="3")]
    pub value: std::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Impact {
    #[prost(string, optional, tag="1")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub disruption_uri: ::std::option::Option<std::string::String>,
    #[prost(message, repeated, tag="10")]
    pub application_periods: ::std::vec::Vec<Period>,
    #[prost(enumeration="ActiveStatus", optional, tag="11")]
    pub status: ::std::option::Option<i32>,
    #[prost(uint64, optional, tag="12")]
    pub updated_at: ::std::option::Option<u64>,
    #[prost(string, repeated, tag="13")]
    pub tags: ::std::vec::Vec<std::string::String>,
    #[prost(string, optional, tag="14")]
    pub cause: ::std::option::Option<std::string::String>,
    #[prost(message, repeated, tag="15")]
    pub messages: ::std::vec::Vec<MessageContent>,
    #[prost(message, optional, tag="16")]
    pub severity: ::std::option::Option<Severity>,
    #[prost(string, optional, tag="17")]
    pub contributor: ::std::option::Option<std::string::String>,
    #[prost(message, repeated, tag="18")]
    pub impacted_objects: ::std::vec::Vec<ImpactedObject>,
    #[prost(string, optional, tag="19")]
    pub category: ::std::option::Option<std::string::String>,
    #[prost(message, repeated, tag="20")]
    pub properties: ::std::vec::Vec<DisruptionProperty>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GeographicalCoord {
    #[prost(double, required, tag="1")]
    pub lon: f64,
    #[prost(double, required, tag="2")]
    pub lat: f64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AdministrativeRegion {
    #[prost(string, optional, tag="2")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="3")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub zip_code: ::std::option::Option<std::string::String>,
    #[prost(int32, optional, tag="5")]
    pub level: ::std::option::Option<i32>,
    #[prost(message, optional, tag="6")]
    pub coord: ::std::option::Option<GeographicalCoord>,
    #[prost(string, optional, tag="7")]
    pub label: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="8")]
    pub insee: ::std::option::Option<std::string::String>,
    #[prost(message, repeated, tag="9")]
    pub main_stop_areas: ::std::vec::Vec<StopArea>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Comment {
    #[prost(string, optional, tag="1")]
    pub value: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub r#type: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StopArea {
    #[prost(string, optional, tag="3")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(message, optional, tag="5")]
    pub coord: ::std::option::Option<GeographicalCoord>,
    #[prost(message, repeated, tag="10")]
    pub administrative_regions: ::std::vec::Vec<AdministrativeRegion>,
    #[prost(message, repeated, tag="8")]
    pub stop_points: ::std::vec::Vec<StopPoint>,
    #[prost(message, repeated, tag="9")]
    pub messages: ::std::vec::Vec<Message>,
    #[prost(string, repeated, tag="25")]
    pub impact_uris: ::std::vec::Vec<std::string::String>,
    #[prost(message, repeated, tag="23")]
    pub comments: ::std::vec::Vec<Comment>,
    #[prost(message, repeated, tag="12")]
    pub codes: ::std::vec::Vec<Code>,
    #[prost(string, optional, tag="15")]
    pub timezone: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="16")]
    pub label: ::std::option::Option<std::string::String>,
    #[prost(message, repeated, tag="21")]
    pub commercial_modes: ::std::vec::Vec<CommercialMode>,
    #[prost(message, repeated, tag="22")]
    pub physical_modes: ::std::vec::Vec<PhysicalMode>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StopPoint {
    #[prost(string, optional, tag="3")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(message, repeated, tag="10")]
    pub administrative_regions: ::std::vec::Vec<AdministrativeRegion>,
    #[prost(string, optional, tag="5")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(message, optional, tag="6")]
    pub coord: ::std::option::Option<GeographicalCoord>,
    #[prost(message, optional, tag="7")]
    pub stop_area: ::std::option::Option<StopArea>,
    #[prost(message, optional, tag="8")]
    pub has_equipments: ::std::option::Option<HasEquipments>,
    #[prost(message, repeated, tag="9")]
    pub messages: ::std::vec::Vec<Message>,
    #[prost(string, repeated, tag="22")]
    pub impact_uris: ::std::vec::Vec<std::string::String>,
    #[prost(message, repeated, tag="16")]
    pub comments: ::std::vec::Vec<Comment>,
    #[prost(message, repeated, tag="12")]
    pub codes: ::std::vec::Vec<Code>,
    #[prost(message, optional, tag="13")]
    pub address: ::std::option::Option<Address>,
    #[prost(string, optional, tag="14")]
    pub platform_code: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="15")]
    pub label: ::std::option::Option<std::string::String>,
    #[prost(message, repeated, tag="17")]
    pub commercial_modes: ::std::vec::Vec<CommercialMode>,
    #[prost(message, repeated, tag="18")]
    pub physical_modes: ::std::vec::Vec<PhysicalMode>,
    #[prost(message, optional, tag="19")]
    pub fare_zone: ::std::option::Option<FareZone>,
    #[prost(message, repeated, tag="20")]
    pub equipment_details: ::std::vec::Vec<EquipmentDetails>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LineString {
    #[prost(message, repeated, tag="1")]
    pub coordinates: ::std::vec::Vec<GeographicalCoord>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MultiLineString {
    #[prost(message, repeated, tag="1")]
    pub lines: ::std::vec::Vec<LineString>,
    #[prost(double, optional, tag="2")]
    pub length: ::std::option::Option<f64>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Line {
    #[prost(string, optional, tag="3")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="5")]
    pub code: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="9")]
    pub color: ::std::option::Option<std::string::String>,
    #[prost(message, repeated, tag="10")]
    pub routes: ::std::vec::Vec<Route>,
    #[prost(message, optional, tag="11")]
    pub commercial_mode: ::std::option::Option<CommercialMode>,
    #[prost(message, repeated, tag="12")]
    pub physical_modes: ::std::vec::Vec<PhysicalMode>,
    #[prost(message, optional, tag="13")]
    pub network: ::std::option::Option<Network>,
    #[prost(message, repeated, tag="14")]
    pub messages: ::std::vec::Vec<Message>,
    #[prost(string, repeated, tag="26")]
    pub impact_uris: ::std::vec::Vec<std::string::String>,
    #[prost(message, repeated, tag="22")]
    pub comments: ::std::vec::Vec<Comment>,
    #[prost(message, repeated, tag="16")]
    pub codes: ::std::vec::Vec<Code>,
    #[prost(message, optional, tag="17")]
    pub geojson: ::std::option::Option<MultiLineString>,
    #[prost(uint32, optional, tag="18")]
    pub opening_time: ::std::option::Option<u32>,
    #[prost(uint32, optional, tag="19")]
    pub closing_time: ::std::option::Option<u32>,
    #[prost(message, repeated, tag="21")]
    pub properties: ::std::vec::Vec<Property>,
    #[prost(message, repeated, tag="23")]
    pub line_groups: ::std::vec::Vec<LineGroup>,
    #[prost(string, optional, tag="24")]
    pub text_color: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LineGroup {
    #[prost(string, optional, tag="1")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(message, repeated, tag="3")]
    pub comments: ::std::vec::Vec<Comment>,
    #[prost(message, repeated, tag="4")]
    pub lines: ::std::vec::Vec<Line>,
    #[prost(message, optional, tag="5")]
    pub main_line: ::std::option::Option<Line>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Route {
    #[prost(string, optional, tag="3")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(bool, optional, tag="5")]
    pub is_frequence: ::std::option::Option<bool>,
    #[prost(message, optional, tag="7")]
    pub line: ::std::option::Option<Line>,
    #[prost(message, repeated, tag="8")]
    pub journey_patterns: ::std::vec::Vec<JourneyPattern>,
    #[prost(message, repeated, tag="9")]
    pub messages: ::std::vec::Vec<Message>,
    #[prost(string, repeated, tag="18")]
    pub impact_uris: ::std::vec::Vec<std::string::String>,
    #[prost(message, repeated, tag="10")]
    pub codes: ::std::vec::Vec<Code>,
    #[prost(message, repeated, tag="1")]
    pub stop_points: ::std::vec::Vec<StopPoint>,
    #[prost(message, optional, boxed, tag="12")]
    pub direction: ::std::option::Option<::std::boxed::Box<PtObject>>,
    #[prost(message, optional, tag="13")]
    pub geojson: ::std::option::Option<MultiLineString>,
    #[prost(message, repeated, tag="14")]
    pub physical_modes: ::std::vec::Vec<PhysicalMode>,
    #[prost(message, repeated, tag="15")]
    pub comments: ::std::vec::Vec<Comment>,
    #[prost(string, optional, tag="17")]
    pub direction_type: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct JourneyPattern {
    #[prost(string, optional, tag="3")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(bool, optional, tag="5")]
    pub is_frequence: ::std::option::Option<bool>,
    #[prost(message, optional, tag="6")]
    pub physical_mode: ::std::option::Option<PhysicalMode>,
    #[prost(message, optional, boxed, tag="7")]
    pub route: ::std::option::Option<::std::boxed::Box<Route>>,
    #[prost(message, repeated, tag="8")]
    pub journey_pattern_points: ::std::vec::Vec<JourneyPatternPoint>,
    #[prost(message, repeated, tag="9")]
    pub messages: ::std::vec::Vec<Message>,
    #[prost(string, repeated, tag="22")]
    pub impact_uris: ::std::vec::Vec<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Destination {
    #[prost(string, required, tag="1")]
    pub uri: std::string::String,
    #[prost(string, optional, tag="2")]
    pub destination: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Note {
    #[prost(string, required, tag="1")]
    pub uri: std::string::String,
    #[prost(string, optional, tag="2")]
    pub note: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="3")]
    pub comment_type: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Properties {
    #[prost(enumeration="properties::AdditionalInformation", repeated, packed="false", tag="1")]
    pub additional_informations: ::std::vec::Vec<i32>,
    #[prost(message, repeated, tag="5")]
    pub notes: ::std::vec::Vec<Note>,
    #[prost(message, repeated, tag="6")]
    pub exceptions: ::std::vec::Vec<CalendarException>,
    #[prost(message, optional, tag="7")]
    pub destination: ::std::option::Option<Destination>,
    #[prost(string, optional, tag="8")]
    pub vehicle_journey_id: ::std::option::Option<std::string::String>,
}
pub mod properties {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum AdditionalInformation {
        PickUpOnly = 1,
        DropOffOnly = 2,
        OnDemandTransport = 3,
        DateTimeEstimated = 4,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HasEquipments {
    #[prost(enumeration="has_equipments::Equipment", repeated, packed="false", tag="1")]
    pub has_equipments: ::std::vec::Vec<i32>,
}
pub mod has_equipments {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Equipment {
        HasWheelchairAccessibility = 1,
        HasBikeAccepted = 2,
        HasAirConditioned = 3,
        HasVisualAnnouncement = 4,
        HasAudibleAnnouncement = 5,
        HasAppropriateEscort = 6,
        HasAppropriateSignage = 7,
        HasSchoolVehicle = 8,
        HasWheelchairBoarding = 9,
        HasSheltered = 10,
        HasElevator = 11,
        HasEscalator = 12,
        HasBikeDepot = 13,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StopDateTime {
    ///posix time
    #[prost(uint64, optional, tag="1")]
    pub arrival_date_time: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="2")]
    pub departure_date_time: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="5")]
    pub base_arrival_date_time: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="6")]
    pub base_departure_date_time: ::std::option::Option<u64>,
    #[prost(message, optional, tag="3")]
    pub stop_point: ::std::option::Option<StopPoint>,
    #[prost(message, optional, tag="4")]
    pub properties: ::std::option::Option<Properties>,
    #[prost(enumeration="RtLevel", optional, tag="7")]
    pub data_freshness: ::std::option::Option<i32>,
    #[prost(enumeration="MessageStatus", optional, tag="8")]
    pub departure_status: ::std::option::Option<i32>,
    #[prost(enumeration="MessageStatus", optional, tag="9")]
    pub arrival_status: ::std::option::Option<i32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StopTime {
    /// Local arrival
    #[prost(uint64, optional, tag="1")]
    pub arrival_time: ::std::option::Option<u64>,
    /// UTC arrival
    #[prost(uint64, optional, tag="10")]
    pub utc_arrival_time: ::std::option::Option<u64>,
    /// Local departure
    #[prost(uint64, optional, tag="3")]
    pub departure_time: ::std::option::Option<u64>,
    /// UTC departure
    #[prost(uint64, optional, tag="11")]
    pub utc_departure_time: ::std::option::Option<u64>,
    #[prost(message, optional, tag="4")]
    pub vehicle_journey: ::std::option::Option<VehicleJourney>,
    #[prost(message, optional, tag="5")]
    pub journey_pattern_point: ::std::option::Option<JourneyPatternPoint>,
    #[prost(bool, optional, tag="6")]
    pub pickup_allowed: ::std::option::Option<bool>,
    #[prost(bool, optional, tag="7")]
    pub drop_off_allowed: ::std::option::Option<bool>,
    #[prost(string, optional, tag="8")]
    pub headsign: ::std::option::Option<std::string::String>,
    #[prost(message, optional, tag="9")]
    pub stop_point: ::std::option::Option<StopPoint>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VehicleJourney {
    #[prost(string, optional, tag="3")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(message, repeated, tag="6")]
    pub stop_times: ::std::vec::Vec<StopTime>,
    #[prost(message, optional, boxed, tag="7")]
    pub route: ::std::option::Option<::std::boxed::Box<Route>>,
    #[prost(message, optional, boxed, tag="8")]
    pub journey_pattern: ::std::option::Option<::std::boxed::Box<JourneyPattern>>,
    #[prost(message, optional, tag="9")]
    pub trip: ::std::option::Option<Trip>,
    #[prost(message, repeated, tag="10")]
    pub messages: ::std::vec::Vec<Message>,
    #[prost(string, repeated, tag="32")]
    pub impact_uris: ::std::vec::Vec<std::string::String>,
    #[prost(bool, optional, tag="11")]
    pub is_adapted: ::std::option::Option<bool>,
    #[prost(message, optional, tag="12")]
    pub validity_pattern: ::std::option::Option<ValidityPattern>,
    #[prost(message, optional, tag="13")]
    pub adapted_validity_pattern: ::std::option::Option<ValidityPattern>,
    #[prost(string, optional, tag="14")]
    pub odt_message: ::std::option::Option<std::string::String>,
    #[prost(bool, optional, tag="16")]
    pub wheelchair_accessible: ::std::option::Option<bool>,
    #[prost(bool, optional, tag="17")]
    pub bike_accepted: ::std::option::Option<bool>,
    #[prost(bool, optional, tag="18")]
    pub air_conditioned: ::std::option::Option<bool>,
    #[prost(bool, optional, tag="19")]
    pub visual_announcement: ::std::option::Option<bool>,
    #[prost(bool, optional, tag="20")]
    pub audible_announcement: ::std::option::Option<bool>,
    #[prost(bool, optional, tag="21")]
    pub appropriate_escort: ::std::option::Option<bool>,
    #[prost(bool, optional, tag="22")]
    pub appropriate_signage: ::std::option::Option<bool>,
    #[prost(bool, optional, tag="23")]
    pub school_vehicle: ::std::option::Option<bool>,
    #[prost(message, repeated, tag="27")]
    pub comments: ::std::vec::Vec<Comment>,
    #[prost(message, repeated, tag="25")]
    pub codes: ::std::vec::Vec<Code>,
    #[prost(message, repeated, tag="26")]
    pub calendars: ::std::vec::Vec<Calendar>,
    #[prost(uint64, optional, tag="28")]
    pub start_time: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="29")]
    pub end_time: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="30")]
    pub headway_secs: ::std::option::Option<u64>,
    #[prost(string, optional, tag="33")]
    pub headsign: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Trip {
    #[prost(string, optional, tag="1")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub name: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct JourneyPatternPoint {
    #[prost(string, optional, tag="3")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(int32, optional, tag="4")]
    pub order: ::std::option::Option<i32>,
    #[prost(message, optional, tag="5")]
    pub stop_point: ::std::option::Option<StopPoint>,
    #[prost(message, optional, tag="6")]
    pub journey_pattern: ::std::option::Option<JourneyPattern>,
    #[prost(message, repeated, tag="7")]
    pub messages: ::std::vec::Vec<Message>,
    #[prost(string, repeated, tag="22")]
    pub impact_uris: ::std::vec::Vec<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Contributor {
    #[prost(string, optional, tag="1")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="3")]
    pub website: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub license: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Dataset {
    #[prost(string, required, tag="1")]
    pub uri: std::string::String,
    #[prost(message, required, tag="2")]
    pub contributor: Contributor,
    #[prost(uint64, required, tag="3")]
    pub start_validation_date: u64,
    #[prost(uint64, required, tag="4")]
    pub end_validation_date: u64,
    #[prost(enumeration="RtLevel", required, tag="5")]
    pub realtime_level: i32,
    #[prost(string, optional, tag="6")]
    pub desc: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="7")]
    pub system: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PoiType {
    #[prost(string, optional, tag="1")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub name: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Poi {
    #[prost(string, optional, tag="3")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(message, optional, tag="6")]
    pub coord: ::std::option::Option<GeographicalCoord>,
    #[prost(message, optional, tag="7")]
    pub poi_type: ::std::option::Option<PoiType>,
    #[prost(message, repeated, tag="10")]
    pub administrative_regions: ::std::vec::Vec<AdministrativeRegion>,
    #[prost(message, optional, tag="11")]
    pub address: ::std::option::Option<Address>,
    #[prost(message, repeated, tag="12")]
    pub properties: ::std::vec::Vec<Code>,
    #[prost(string, optional, tag="13")]
    pub label: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Network {
    #[prost(string, optional, tag="3")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(message, repeated, tag="6")]
    pub messages: ::std::vec::Vec<Message>,
    #[prost(string, repeated, tag="22")]
    pub impact_uris: ::std::vec::Vec<std::string::String>,
    #[prost(message, repeated, tag="7")]
    pub codes: ::std::vec::Vec<Code>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PhysicalMode {
    #[prost(string, optional, tag="3")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub name: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CommercialMode {
    #[prost(string, optional, tag="3")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub name: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Connection {
    #[prost(message, optional, tag="1")]
    pub origin: ::std::option::Option<StopPoint>,
    #[prost(message, optional, tag="2")]
    pub destination: ::std::option::Option<StopPoint>,
    #[prost(int32, optional, tag="3")]
    pub duration: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="4")]
    pub display_duration: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="5")]
    pub max_duration: ::std::option::Option<i32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Company {
    #[prost(string, optional, tag="3")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(message, repeated, tag="8")]
    pub codes: ::std::vec::Vec<Code>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Address {
    #[prost(string, optional, tag="3")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(message, optional, tag="6")]
    pub coord: ::std::option::Option<GeographicalCoord>,
    #[prost(message, repeated, tag="10")]
    pub administrative_regions: ::std::vec::Vec<AdministrativeRegion>,
    #[prost(int32, optional, tag="2")]
    pub house_number: ::std::option::Option<i32>,
    #[prost(string, optional, tag="16")]
    pub label: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CalendarException {
    #[prost(string, optional, tag="1")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub date: ::std::option::Option<std::string::String>,
    #[prost(enumeration="ExceptionType", optional, tag="3")]
    pub r#type: ::std::option::Option<i32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CalendarPeriod {
    #[prost(string, optional, tag="1")]
    pub begin: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub end: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WeekPattern {
    #[prost(bool, optional, tag="1")]
    pub monday: ::std::option::Option<bool>,
    #[prost(bool, optional, tag="2")]
    pub tuesday: ::std::option::Option<bool>,
    #[prost(bool, optional, tag="3")]
    pub wednesday: ::std::option::Option<bool>,
    #[prost(bool, optional, tag="4")]
    pub thursday: ::std::option::Option<bool>,
    #[prost(bool, optional, tag="5")]
    pub friday: ::std::option::Option<bool>,
    #[prost(bool, optional, tag="6")]
    pub saturday: ::std::option::Option<bool>,
    #[prost(bool, optional, tag="7")]
    pub sunday: ::std::option::Option<bool>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Calendar {
    #[prost(string, optional, tag="1")]
    pub uri: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(message, optional, tag="3")]
    pub week_pattern: ::std::option::Option<WeekPattern>,
    #[prost(message, repeated, tag="4")]
    pub active_periods: ::std::vec::Vec<CalendarPeriod>,
    #[prost(message, repeated, tag="5")]
    pub exceptions: ::std::vec::Vec<CalendarException>,
    #[prost(message, optional, tag="6")]
    pub validity_pattern: ::std::option::Option<ValidityPattern>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ValidityPattern {
    #[prost(string, optional, tag="1")]
    pub beginning_date: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub days: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LocationContext {
    #[prost(string, required, tag="1")]
    pub place: std::string::String,
    #[prost(int32, required, tag="2")]
    pub access_duration: i32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PtObject {
    #[prost(string, required, tag="1")]
    pub name: std::string::String,
    #[prost(string, required, tag="2")]
    pub uri: std::string::String,
    #[prost(enumeration="NavitiaType", optional, tag="3")]
    pub embedded_type: ::std::option::Option<i32>,
    #[prost(message, optional, tag="4")]
    pub stop_area: ::std::option::Option<StopArea>,
    #[prost(message, optional, tag="5")]
    pub poi: ::std::option::Option<Poi>,
    #[prost(message, optional, tag="6")]
    pub stop_point: ::std::option::Option<StopPoint>,
    #[prost(message, optional, tag="7")]
    pub address: ::std::option::Option<Address>,
    #[prost(message, optional, tag="9")]
    pub line: ::std::option::Option<Line>,
    #[prost(message, optional, tag="10")]
    pub network: ::std::option::Option<Network>,
    #[prost(message, optional, tag="11")]
    pub commercial_mode: ::std::option::Option<CommercialMode>,
    #[prost(message, optional, boxed, tag="12")]
    pub route: ::std::option::Option<::std::boxed::Box<Route>>,
    #[prost(message, optional, tag="13")]
    pub administrative_region: ::std::option::Option<AdministrativeRegion>,
    #[prost(int32, optional, tag="14")]
    pub distance: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="15")]
    pub quality: ::std::option::Option<i32>,
    #[prost(message, optional, tag="16")]
    pub company: ::std::option::Option<Company>,
    #[prost(message, optional, boxed, tag="17")]
    pub vehicle_journey: ::std::option::Option<::std::boxed::Box<VehicleJourney>>,
    #[prost(message, optional, tag="18")]
    pub calendar: ::std::option::Option<Calendar>,
    /// DEPRECATED
    #[prost(int32, optional, tag="19")]
    pub score: ::std::option::Option<i32>,
    #[prost(message, optional, tag="20")]
    pub trip: ::std::option::Option<Trip>,
    #[prost(int32, repeated, packed="false", tag="21")]
    pub scores: ::std::vec::Vec<i32>,
    #[prost(message, repeated, tag="22")]
    pub stop_points_nearby: ::std::vec::Vec<PtObject>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FareZone {
    #[prost(string, optional, tag="1")]
    pub name: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EquipmentDetails {
    #[prost(string, optional, tag="1")]
    pub id: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(enumeration="equipment_details::EquipmentType", optional, tag="3")]
    pub embedded_type: ::std::option::Option<i32>,
    #[prost(message, optional, tag="4")]
    pub current_availability: ::std::option::Option<CurrentAvailability>,
}
pub mod equipment_details {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum EquipmentType {
        Escalator = 1,
        Elevator = 2,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CurrentAvailability {
    #[prost(enumeration="current_availability::EquipmentStatus", optional, tag="1")]
    pub status: ::std::option::Option<i32>,
    #[prost(message, repeated, tag="2")]
    pub periods: ::std::vec::Vec<Period>,
    #[prost(string, optional, tag="3")]
    pub updated_at: ::std::option::Option<std::string::String>,
    #[prost(message, optional, tag="4")]
    pub cause: ::std::option::Option<EquipmentCause>,
    #[prost(message, optional, tag="5")]
    pub effect: ::std::option::Option<EquipmentEffect>,
}
pub mod current_availability {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum EquipmentStatus {
        Unknown = 0,
        Available = 1,
        Unavailable = 2,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EquipmentCause {
    #[prost(string, optional, tag="1")]
    pub label: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EquipmentEffect {
    #[prost(string, optional, tag="1")]
    pub label: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StopAreaEquipment {
    #[prost(message, optional, tag="1")]
    pub stop_area: ::std::option::Option<StopArea>,
    #[prost(message, repeated, tag="2")]
    pub equipment_details: ::std::vec::Vec<EquipmentDetails>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum NavitiaType {
    Line = 1,
    JourneyPattern = 2,
    VehicleJourney = 3,
    StopPoint = 4,
    StopArea = 5,
    Network = 6,
    PhysicalMode = 7,
    CommercialMode = 8,
    Connection = 9,
    JourneyPatternPoint = 10,
    Company = 11,
    Route = 12,
    Poi = 13,
    Contributor = 16,
    Address = 18,
    Poitype = 23,
    AdministrativeRegion = 22,
    Calendar = 25,
    LineGroup = 26,
    Impact = 27,
    Dataset = 28,
    Trip = 29,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Api {
    Places = 1,
    Ptreferential = 2,
    Planner = 4,
    PlacesNearby = 6,
    Status = 7,
    NextDepartures = 8,
    NextArrivals = 9,
    DepartureBoards = 10,
    RouteSchedules = 11,
    Isochrone = 13,
    Metadatas = 14,
    PlaceUri = 15,
    UnknownApi = 16,
    TrafficReports = 26,
    Calendars = 18,
    Nmplanner = 19,
    PtObjects = 20,
    PlaceCode = 21,
    Disruptions = 25,
    NearestStopPoints = 27,
    PtPlanner = 28,
    GraphicalIsochrone = 29,
    GeoStatus = 30,
    CarCo2Emission = 31,
    DirectPath = 32,
    HeatMap = 33,
    StreetNetworkRoutingMatrix = 34,
    OdtStopPoints = 35,
    MatchingRoutes = 36,
    LineReports = 37,
    EquipmentReports = 38,
    TerminusSchedules = 39,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ResponseStatus {
    None = 0,
    NoDepartureThisDay = 1,
    NoActiveModeThisDay = 2,
    NoActiveCirculationThisDay = 3,
    Terminus = 6,
    DateOutOfBound = 7,
    PartialTerminus = 8,
    Ok = 9,
    ActiveDisruption = 10,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ActiveStatus {
    Past = 0,
    Active = 1,
    Future = 2,
}
///message are the old disruption, remove as soon as possible
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum MessageStatus {
    Information = 0,
    Warning = 1,
    Disrupt = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum StopTimeUpdateStatus {
    Delayed = 0,
    Added = 1,
    Deleted = 2,
    Unchanged = 3,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum OdtLevel {
    Scheduled = 0,
    WithStops = 1,
    Zonal = 2,
    All = 3,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum RtLevel {
    BaseSchedule = 1,
    AdaptedSchedule = 2,
    Realtime = 3,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ExceptionType {
    Add = 0,
    Remove = 1,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CalendarsRequest {
    #[prost(string, optional, tag="1")]
    pub start_date: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub end_date: ::std::option::Option<std::string::String>,
    #[prost(int32, optional, tag="3")]
    pub depth: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="4")]
    pub start_page: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="5")]
    pub count: ::std::option::Option<i32>,
    #[prost(string, optional, tag="6")]
    pub filter: ::std::option::Option<std::string::String>,
    #[prost(string, repeated, tag="7")]
    pub forbidden_uris: ::std::vec::Vec<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TrafficReportsRequest {
    #[prost(uint64, optional, tag="8")]
    pub application_period_begin: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="9")]
    pub application_period_end: ::std::option::Option<u64>,
    /// to be removed
    #[prost(uint64, optional, tag="10")]
    pub current_datetime: ::std::option::Option<u64>,
    #[prost(int32, optional, tag="3")]
    pub depth: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="4")]
    pub start_page: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="5")]
    pub count: ::std::option::Option<i32>,
    #[prost(string, optional, tag="6")]
    pub filter: ::std::option::Option<std::string::String>,
    #[prost(string, repeated, tag="7")]
    pub forbidden_uris: ::std::vec::Vec<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LineReportsRequest {
    #[prost(int32, optional, tag="1")]
    pub depth: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="2")]
    pub start_page: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="3")]
    pub count: ::std::option::Option<i32>,
    #[prost(string, optional, tag="4")]
    pub filter: ::std::option::Option<std::string::String>,
    #[prost(string, repeated, tag="5")]
    pub forbidden_uris: ::std::vec::Vec<std::string::String>,
    #[prost(uint64, optional, tag="6")]
    pub since_datetime: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="7")]
    pub until_datetime: ::std::option::Option<u64>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PlacesRequest {
    #[prost(string, required, tag="1")]
    pub q: std::string::String,
    #[prost(enumeration="NavitiaType", repeated, packed="false", tag="2")]
    pub types: ::std::vec::Vec<i32>,
    #[prost(int32, required, tag="3")]
    pub depth: i32,
    #[prost(int32, required, tag="4")]
    pub count: i32,
    #[prost(string, repeated, tag="5")]
    pub admin_uris: ::std::vec::Vec<std::string::String>,
    #[prost(int32, optional, tag="6")]
    pub search_type: ::std::option::Option<i32>,
    #[prost(float, optional, tag="7", default="1")]
    pub main_stop_area_weight_factor: ::std::option::Option<f32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NextStopTimeRequest {
    #[prost(string, required, tag="1")]
    pub departure_filter: std::string::String,
    #[prost(string, required, tag="2")]
    pub arrival_filter: std::string::String,
    #[prost(uint64, optional, tag="3")]
    pub from_datetime: ::std::option::Option<u64>,
    #[prost(int32, required, tag="4")]
    pub duration: i32,
    #[prost(int32, required, tag="5")]
    pub depth: i32,
    #[prost(int32, required, tag="7")]
    pub nb_stoptimes: i32,
    /// to be removed
    #[prost(int32, optional, tag="8")]
    pub interface_version: ::std::option::Option<i32>,
    #[prost(int32, required, tag="9")]
    pub start_page: i32,
    #[prost(int32, required, tag="10")]
    pub count: i32,
    /// to be removed
    #[prost(int32, optional, tag="11")]
    pub max_date_times: ::std::option::Option<i32>,
    #[prost(string, repeated, tag="12")]
    pub forbidden_uri: ::std::vec::Vec<std::string::String>,
    #[prost(string, optional, tag="13")]
    pub calendar: ::std::option::Option<std::string::String>,
    /// to be removed
    #[prost(bool, optional, tag="14")]
    pub show_codes: ::std::option::Option<bool>,
    #[prost(uint64, optional, tag="15")]
    pub until_datetime: ::std::option::Option<u64>,
    /// to be removed
    #[prost(uint64, optional, tag="16")]
    pub current_datetime: ::std::option::Option<u64>,
    #[prost(enumeration="RtLevel", optional, tag="17")]
    pub realtime_level: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="18")]
    pub items_per_schedule: ::std::option::Option<i32>,
    #[prost(bool, optional, tag="19")]
    pub disable_geojson: ::std::option::Option<bool>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StreetNetworkParams {
    #[prost(string, optional, tag="1")]
    pub origin_mode: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub destination_mode: ::std::option::Option<std::string::String>,
    #[prost(double, optional, tag="3")]
    pub walking_speed: ::std::option::Option<f64>,
    #[prost(double, optional, tag="5")]
    pub bike_speed: ::std::option::Option<f64>,
    #[prost(double, optional, tag="7")]
    pub car_speed: ::std::option::Option<f64>,
    #[prost(double, optional, tag="9")]
    pub bss_speed: ::std::option::Option<f64>,
    #[prost(string, optional, tag="11")]
    pub origin_filter: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="12")]
    pub destination_filter: ::std::option::Option<std::string::String>,
    #[prost(int32, optional, tag="13")]
    pub max_walking_duration_to_pt: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="14")]
    pub max_bike_duration_to_pt: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="15")]
    pub max_bss_duration_to_pt: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="16")]
    pub max_car_duration_to_pt: ::std::option::Option<i32>,
    #[prost(bool, optional, tag="17", default="true")]
    pub enable_direct_path: ::std::option::Option<bool>,
    #[prost(double, optional, tag="18")]
    pub car_no_park_speed: ::std::option::Option<f64>,
    #[prost(int32, optional, tag="19")]
    pub max_car_no_park_duration_to_pt: ::std::option::Option<i32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct JourneysRequest {
    #[prost(message, repeated, tag="1")]
    pub origin: ::std::vec::Vec<LocationContext>,
    #[prost(message, repeated, tag="2")]
    pub destination: ::std::vec::Vec<LocationContext>,
    #[prost(uint64, repeated, packed="false", tag="3")]
    pub datetimes: ::std::vec::Vec<u64>,
    #[prost(bool, required, tag="4")]
    pub clockwise: bool,
    #[prost(string, repeated, tag="5")]
    pub forbidden_uris: ::std::vec::Vec<std::string::String>,
    #[prost(int32, required, tag="6")]
    pub max_duration: i32,
    #[prost(int32, required, tag="7")]
    pub max_transfers: i32,
    #[prost(message, optional, tag="8")]
    pub streetnetwork_params: ::std::option::Option<StreetNetworkParams>,
    #[prost(bool, optional, tag="9", default="false")]
    pub wheelchair: ::std::option::Option<bool>,
    /// to be removed
    #[prost(bool, optional, tag="11")]
    pub show_codes: ::std::option::Option<bool>,
    ///to be removed
    #[prost(bool, optional, tag="13")]
    pub details: ::std::option::Option<bool>,
    #[prost(enumeration="RtLevel", optional, tag="14")]
    pub realtime_level: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="15", default="0")]
    pub max_extra_second_pass: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="16", default="120")]
    pub walking_transfer_penalty: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="17")]
    pub direct_path_duration: ::std::option::Option<i32>,
    #[prost(bool, optional, tag="18")]
    pub bike_in_pt: ::std::option::Option<bool>,
    #[prost(string, repeated, tag="19")]
    pub allowed_id: ::std::vec::Vec<std::string::String>,
    /// meters
    #[prost(int32, optional, tag="20", default="0")]
    pub free_radius_from: ::std::option::Option<i32>,
    /// meters
    #[prost(int32, optional, tag="21", default="0")]
    pub free_radius_to: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="22")]
    pub min_nb_journeys: ::std::option::Option<i32>,
    #[prost(double, optional, tag="23", default="1.5")]
    pub night_bus_filter_max_factor: ::std::option::Option<f64>,
    /// seconds
    #[prost(int32, optional, tag="24", default="900")]
    pub night_bus_filter_base_factor: ::std::option::Option<i32>,
    /// seconds
    #[prost(uint32, optional, tag="25")]
    pub timeframe_duration: ::std::option::Option<u32>,
    #[prost(int32, optional, tag="26", default="1")]
    pub depth: ::std::option::Option<i32>,
    /// Needed for isochrone distributed
    #[prost(message, optional, tag="27")]
    pub isochrone_center: ::std::option::Option<LocationContext>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PlacesNearbyRequest {
    #[prost(string, required, tag="1")]
    pub uri: std::string::String,
    #[prost(double, required, tag="2")]
    pub distance: f64,
    #[prost(enumeration="NavitiaType", repeated, packed="false", tag="3")]
    pub types: ::std::vec::Vec<i32>,
    #[prost(int32, required, tag="4")]
    pub depth: i32,
    #[prost(int32, required, tag="5")]
    pub count: i32,
    #[prost(int32, required, tag="6")]
    pub start_page: i32,
    #[prost(string, optional, tag="7")]
    pub filter: ::std::option::Option<std::string::String>,
    #[prost(double, optional, tag="8")]
    pub stop_points_nearby_radius: ::std::option::Option<f64>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PlaceUriRequest {
    #[prost(string, required, tag="1")]
    pub uri: std::string::String,
    #[prost(int32, optional, tag="2", default="1")]
    pub depth: ::std::option::Option<i32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PlaceCodeRequest {
    #[prost(enumeration="place_code_request::Type", required, tag="1")]
    pub r#type: i32,
    #[prost(string, required, tag="2")]
    pub type_code: std::string::String,
    #[prost(string, required, tag="3")]
    pub code: std::string::String,
}
pub mod place_code_request {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Type {
        StopArea = 0,
        Network = 1,
        Company = 2,
        Line = 3,
        Route = 4,
        VehicleJourney = 5,
        StopPoint = 6,
        Calendar = 7,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PtRefRequest {
    #[prost(enumeration="NavitiaType", required, tag="1")]
    pub requested_type: i32,
    #[prost(string, required, tag="2")]
    pub filter: std::string::String,
    #[prost(int32, required, tag="3")]
    pub depth: i32,
    #[prost(int32, required, tag="4")]
    pub start_page: i32,
    #[prost(int32, required, tag="5")]
    pub count: i32,
    /// to be removed
    #[prost(bool, optional, tag="7")]
    pub show_codes: ::std::option::Option<bool>,
    #[prost(enumeration="OdtLevel", optional, tag="8")]
    pub odt_level: ::std::option::Option<i32>,
    #[prost(string, repeated, tag="6")]
    pub forbidden_uri: ::std::vec::Vec<std::string::String>,
    /// to be removed
    #[prost(uint64, optional, tag="9")]
    pub datetime: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="10")]
    pub since_datetime: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="11")]
    pub until_datetime: ::std::option::Option<u64>,
    #[prost(bool, optional, tag="12")]
    pub disable_geojson: ::std::option::Option<bool>,
    #[prost(enumeration="RtLevel", optional, tag="13")]
    pub realtime_level: ::std::option::Option<i32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CarCo2EmissionRequest {
    #[prost(message, optional, tag="1")]
    pub origin: ::std::option::Option<LocationContext>,
    #[prost(message, optional, tag="2")]
    pub destination: ::std::option::Option<LocationContext>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DirectPathRequest {
    #[prost(message, optional, tag="1")]
    pub origin: ::std::option::Option<LocationContext>,
    #[prost(message, optional, tag="2")]
    pub destination: ::std::option::Option<LocationContext>,
    #[prost(uint64, optional, tag="3")]
    pub datetime: ::std::option::Option<u64>,
    #[prost(bool, required, tag="4")]
    pub clockwise: bool,
    #[prost(message, optional, tag="5")]
    pub streetnetwork_params: ::std::option::Option<StreetNetworkParams>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StreetNetworkRoutingMatrixRequest {
    #[prost(message, repeated, tag="1")]
    pub origins: ::std::vec::Vec<LocationContext>,
    #[prost(message, repeated, tag="2")]
    pub destinations: ::std::vec::Vec<LocationContext>,
    #[prost(string, optional, tag="3")]
    pub mode: ::std::option::Option<std::string::String>,
    #[prost(float, optional, tag="4")]
    pub speed: ::std::option::Option<f32>,
    #[prost(int32, optional, tag="5")]
    pub max_duration: ::std::option::Option<i32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MatchingRoute {
    #[prost(string, optional, tag="1")]
    pub line_uri: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub start_stop_point_uri: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="3")]
    pub destination_code_key: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub destination_code: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Request {
    #[prost(enumeration="Api", required, tag="1")]
    pub requested_api: i32,
    #[prost(message, optional, tag="2")]
    pub places: ::std::option::Option<PlacesRequest>,
    #[prost(message, optional, tag="3")]
    pub next_stop_times: ::std::option::Option<NextStopTimeRequest>,
    #[prost(message, optional, tag="4")]
    pub places_nearby: ::std::option::Option<PlacesNearbyRequest>,
    #[prost(message, optional, tag="5")]
    pub journeys: ::std::option::Option<JourneysRequest>,
    #[prost(message, optional, tag="6")]
    pub ptref: ::std::option::Option<PtRefRequest>,
    #[prost(message, optional, tag="7")]
    pub place_uri: ::std::option::Option<PlaceUriRequest>,
    #[prost(message, optional, tag="13")]
    pub traffic_reports: ::std::option::Option<TrafficReportsRequest>,
    #[prost(message, optional, tag="9")]
    pub calendars: ::std::option::Option<CalendarsRequest>,
    #[prost(message, optional, tag="10")]
    pub pt_objects: ::std::option::Option<PtobjectRequest>,
    #[prost(message, optional, tag="11")]
    pub place_code: ::std::option::Option<PlaceCodeRequest>,
    #[prost(message, optional, tag="14")]
    pub nearest_stop_points: ::std::option::Option<NearestStopPointsRequest>,
    #[prost(uint64, optional, tag="15")]
    pub current_datetime: ::std::option::Option<u64>,
    #[prost(message, optional, tag="16")]
    pub isochrone: ::std::option::Option<GraphicalIsochroneRequest>,
    #[prost(message, optional, tag="17")]
    pub car_co2_emission: ::std::option::Option<CarCo2EmissionRequest>,
    #[prost(message, optional, tag="18")]
    pub direct_path: ::std::option::Option<DirectPathRequest>,
    #[prost(message, optional, tag="19")]
    pub heat_map: ::std::option::Option<HeatMapRequest>,
    #[prost(message, optional, tag="20")]
    pub sn_routing_matrix: ::std::option::Option<StreetNetworkRoutingMatrixRequest>,
    #[prost(message, optional, tag="21")]
    pub coord: ::std::option::Option<GeographicalCoord>,
    #[prost(message, optional, tag="23")]
    pub matching_routes: ::std::option::Option<MatchingRoute>,
    #[prost(message, optional, tag="24")]
    pub line_reports: ::std::option::Option<LineReportsRequest>,
    #[prost(string, optional, tag="12")]
    pub request_id: ::std::option::Option<std::string::String>,
    #[prost(bool, optional, tag="22")]
    pub disable_feedpublisher: ::std::option::Option<bool>,
    #[prost(bool, optional, tag="25", default="false")]
    pub disable_disruption: ::std::option::Option<bool>,
    ///after this date the request should be ignored as the caller will have timeouted
    ///the format of the date is: 20190101T120102,32910
    ///a string is used to be able to store milliseconds
    #[prost(string, optional, tag="26")]
    pub deadline: ::std::option::Option<std::string::String>,
    #[prost(message, optional, tag="27")]
    pub equipment_reports: ::std::option::Option<EquipmentReportsRequest>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NearestStopPointsRequest {
    #[prost(string, optional, tag="1")]
    pub place: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub mode: ::std::option::Option<std::string::String>,
    #[prost(double, optional, tag="3")]
    pub walking_speed: ::std::option::Option<f64>,
    #[prost(double, optional, tag="4")]
    pub bike_speed: ::std::option::Option<f64>,
    #[prost(double, optional, tag="5")]
    pub car_speed: ::std::option::Option<f64>,
    #[prost(double, optional, tag="6")]
    pub bss_speed: ::std::option::Option<f64>,
    #[prost(string, optional, tag="7")]
    pub filter: ::std::option::Option<std::string::String>,
    #[prost(int32, optional, tag="8")]
    pub max_duration: ::std::option::Option<i32>,
    #[prost(bool, optional, tag="9")]
    pub reverse: ::std::option::Option<bool>,
    #[prost(double, optional, tag="10")]
    pub car_no_park_speed: ::std::option::Option<f64>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GraphicalIsochroneRequest {
    #[prost(message, optional, tag="1")]
    pub journeys_request: ::std::option::Option<JourneysRequest>,
    #[prost(int32, repeated, packed="false", tag="2")]
    pub boundary_duration: ::std::vec::Vec<i32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HeatMapRequest {
    #[prost(message, optional, tag="1")]
    pub journeys_request: ::std::option::Option<JourneysRequest>,
    #[prost(int32, optional, tag="2")]
    pub resolution: ::std::option::Option<i32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PtobjectRequest {
    #[prost(string, required, tag="1")]
    pub q: std::string::String,
    #[prost(enumeration="NavitiaType", repeated, packed="false", tag="2")]
    pub types: ::std::vec::Vec<i32>,
    #[prost(int32, required, tag="3")]
    pub depth: i32,
    #[prost(int32, required, tag="4")]
    pub count: i32,
    #[prost(string, repeated, tag="5")]
    pub admin_uris: ::std::vec::Vec<std::string::String>,
    #[prost(int32, optional, tag="6")]
    pub search_type: ::std::option::Option<i32>,
    #[prost(bool, optional, tag="7")]
    pub disable_geojson: ::std::option::Option<bool>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EquipmentReportsRequest {
    #[prost(int32, optional, tag="1")]
    pub depth: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="2")]
    pub start_page: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="3")]
    pub count: ::std::option::Option<i32>,
    #[prost(string, optional, tag="4")]
    pub filter: ::std::option::Option<std::string::String>,
    #[prost(string, repeated, tag="5")]
    pub forbidden_uris: ::std::vec::Vec<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PathItem {
    #[prost(string, optional, tag="1")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(double, optional, tag="2")]
    pub length: ::std::option::Option<f64>,
    #[prost(int32, optional, tag="3")]
    pub direction: ::std::option::Option<i32>,
    #[prost(double, optional, tag="4")]
    pub duration: ::std::option::Option<f64>,
    #[prost(enumeration="CyclePathType", optional, tag="5")]
    pub cycle_path_type: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="6")]
    pub id: ::std::option::Option<i32>,
    #[prost(message, optional, tag="7")]
    pub coordinate: ::std::option::Option<GeographicalCoord>,
    #[prost(string, optional, tag="8")]
    pub instruction: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StreetNetwork {
    #[prost(double, optional, tag="1")]
    pub length: ::std::option::Option<f64>,
    #[prost(double, optional, tag="2")]
    pub duration: ::std::option::Option<f64>,
    #[prost(enumeration="StreetNetworkMode", optional, tag="3")]
    pub mode: ::std::option::Option<i32>,
    #[prost(message, repeated, tag="4")]
    pub path_items: ::std::vec::Vec<PathItem>,
    #[prost(message, repeated, tag="5")]
    pub coordinates: ::std::vec::Vec<GeographicalCoord>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PtDisplayInfo {
    #[prost(string, optional, tag="1")]
    pub network: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub code: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="3")]
    pub headsign: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub direction: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="5")]
    pub color: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="6")]
    pub commercial_mode: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="7")]
    pub physical_mode: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="8")]
    pub description: ::std::option::Option<std::string::String>,
    #[prost(message, optional, tag="9")]
    pub uris: ::std::option::Option<Uris>,
    #[prost(message, optional, tag="11")]
    pub has_equipments: ::std::option::Option<HasEquipments>,
    #[prost(string, optional, tag="12")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(message, repeated, tag="13")]
    pub messages: ::std::vec::Vec<Message>,
    #[prost(string, repeated, tag="18")]
    pub impact_uris: ::std::vec::Vec<std::string::String>,
    #[prost(message, repeated, tag="15")]
    pub notes: ::std::vec::Vec<Note>,
    #[prost(string, repeated, tag="16")]
    pub headsigns: ::std::vec::Vec<std::string::String>,
    #[prost(string, optional, tag="17")]
    pub text_color: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="19")]
    pub trip_short_name: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Uris {
    #[prost(string, optional, tag="1")]
    pub company: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub vehicle_journey: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="3")]
    pub line: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub route: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="5")]
    pub commercial_mode: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="6")]
    pub physical_mode: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="7")]
    pub network: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="8")]
    pub note: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="9")]
    pub journey_pattern: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Cost {
    #[prost(double, optional, tag="1")]
    pub value: ::std::option::Option<f64>,
    #[prost(string, optional, tag="2")]
    pub currency: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Ticket {
    #[prost(string, optional, tag="1")]
    pub id: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(message, optional, tag="3")]
    pub cost: ::std::option::Option<Cost>,
    #[prost(string, repeated, tag="4")]
    pub section_id: ::std::vec::Vec<std::string::String>,
    #[prost(bool, optional, tag="5")]
    pub found: ::std::option::Option<bool>,
    #[prost(string, optional, tag="6")]
    pub comment: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="7")]
    pub source_id: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Fare {
    #[prost(message, optional, tag="1")]
    pub total: ::std::option::Option<Cost>,
    #[prost(string, repeated, tag="2")]
    pub ticket_id: ::std::vec::Vec<std::string::String>,
    #[prost(bool, optional, tag="3")]
    pub found: ::std::option::Option<bool>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Co2Emission {
    #[prost(double, optional, tag="1")]
    pub value: ::std::option::Option<f64>,
    #[prost(string, optional, tag="2")]
    pub unit: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Durations {
    #[prost(int32, optional, tag="1")]
    pub total: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="2")]
    pub walking: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="3")]
    pub bike: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="4")]
    pub car: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="5")]
    pub ridesharing: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="6")]
    pub taxi: ::std::option::Option<i32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Distances {
    #[prost(int32, optional, tag="1")]
    pub walking: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="2")]
    pub bike: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="3")]
    pub car: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="4")]
    pub ridesharing: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="5")]
    pub taxi: ::std::option::Option<i32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IndividualRating {
    #[prost(double, optional, tag="1")]
    pub value: ::std::option::Option<f64>,
    #[prost(uint32, optional, tag="2")]
    pub count: ::std::option::Option<u32>,
    #[prost(double, optional, tag="3")]
    pub scale_min: ::std::option::Option<f64>,
    #[prost(double, optional, tag="4")]
    pub scale_max: ::std::option::Option<f64>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IndividualInformation {
    #[prost(string, optional, tag="1")]
    pub alias: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub image: ::std::option::Option<std::string::String>,
    #[prost(enumeration="GenderType", optional, tag="3")]
    pub gender: ::std::option::Option<i32>,
    #[prost(message, optional, tag="4")]
    pub rating: ::std::option::Option<IndividualRating>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SeatsDescription {
    #[prost(uint32, optional, tag="1")]
    pub total: ::std::option::Option<u32>,
    #[prost(uint32, optional, tag="2")]
    pub available: ::std::option::Option<u32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExternalLink {
    #[prost(string, optional, tag="1")]
    pub key: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub href: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RidesharingInformation {
    #[prost(string, optional, tag="1")]
    pub operator: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub network: ::std::option::Option<std::string::String>,
    #[prost(message, optional, tag="3")]
    pub driver: ::std::option::Option<IndividualInformation>,
    #[prost(message, optional, tag="4")]
    pub seats: ::std::option::Option<SeatsDescription>,
    #[prost(message, repeated, tag="5")]
    pub links: ::std::vec::Vec<ExternalLink>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FeedPublisher {
    #[prost(string, required, tag="1")]
    pub id: std::string::String,
    #[prost(string, optional, tag="2")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="3")]
    pub url: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub license: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Section {
    #[prost(enumeration="SectionType", optional, tag="1")]
    pub r#type: ::std::option::Option<i32>,
    #[prost(message, optional, tag="2")]
    pub origin: ::std::option::Option<PtObject>,
    #[prost(message, optional, tag="3")]
    pub destination: ::std::option::Option<PtObject>,
    /// Si c'est du TC
    #[prost(message, optional, tag="4")]
    pub pt_display_informations: ::std::option::Option<PtDisplayInfo>,
    #[prost(message, optional, tag="5")]
    pub uris: ::std::option::Option<Uris>,
    #[prost(message, optional, tag="9")]
    pub vehicle_journey: ::std::option::Option<VehicleJourney>,
    #[prost(message, repeated, tag="10")]
    pub stop_date_times: ::std::vec::Vec<StopDateTime>,
    /// Si c'est du routier
    #[prost(message, optional, tag="12")]
    pub street_network: ::std::option::Option<StreetNetwork>,
    #[prost(int32, optional, tag="30")]
    pub cycle_lane_length: ::std::option::Option<i32>,
    /// Si c'est de l'attente
    #[prost(enumeration="TransferType", optional, tag="13")]
    pub transfer_type: ::std::option::Option<i32>,
    /// If it is crowfly ridesharing (top-level)
    #[prost(message, repeated, tag="28")]
    pub ridesharing_journeys: ::std::vec::Vec<Journey>,
    /// If it is ridesharing section (low-level)
    #[prost(message, optional, tag="29")]
    pub ridesharing_information: ::std::option::Option<RidesharingInformation>,
    /// Dans tous les cas
    #[prost(message, repeated, tag="24")]
    pub shape: ::std::vec::Vec<GeographicalCoord>,
    #[prost(int32, optional, tag="15")]
    pub duration: ::std::option::Option<i32>,
    #[prost(uint64, optional, tag="16")]
    pub begin_date_time: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="17")]
    pub end_date_time: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="25")]
    pub base_begin_date_time: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="26")]
    pub base_end_date_time: ::std::option::Option<u64>,
    #[prost(enumeration="RtLevel", optional, tag="27")]
    pub realtime_level: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="19")]
    pub length: ::std::option::Option<i32>,
    #[prost(string, optional, tag="20")]
    pub id: ::std::option::Option<std::string::String>,
    #[prost(message, optional, tag="21")]
    pub co2_emission: ::std::option::Option<Co2Emission>,
    #[prost(enumeration="SectionAdditionalInformationType", repeated, packed="false", tag="22")]
    pub additional_informations: ::std::vec::Vec<i32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Journey {
    ///TODO: to be deleted after implemenation of durations
    #[prost(int32, optional, tag="1")]
    pub duration: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="2")]
    pub nb_transfers: ::std::option::Option<i32>,
    #[prost(uint64, optional, tag="3")]
    pub departure_date_time: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="4")]
    pub arrival_date_time: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="5")]
    pub requested_date_time: ::std::option::Option<u64>,
    #[prost(message, repeated, tag="6")]
    pub sections: ::std::vec::Vec<Section>,
    #[prost(message, optional, tag="7")]
    pub origin: ::std::option::Option<PtObject>,
    #[prost(message, optional, tag="8")]
    pub destination: ::std::option::Option<PtObject>,
    /// for jormungandr only
    #[prost(string, optional, tag="9")]
    pub r#type: ::std::option::Option<std::string::String>,
    #[prost(message, optional, tag="10")]
    pub fare: ::std::option::Option<Fare>,
    ///for jormungandr only
    #[prost(string, repeated, tag="11")]
    pub tags: ::std::vec::Vec<std::string::String>,
    #[prost(message, repeated, tag="12")]
    pub calendars: ::std::vec::Vec<Calendar>,
    #[prost(message, optional, tag="13")]
    pub co2_emission: ::std::option::Option<Co2Emission>,
    #[prost(string, optional, tag="14")]
    pub most_serious_disruption_effect: ::std::option::Option<std::string::String>,
    ///for jormungandr only. for log purpose we add an id to the journey
    #[prost(string, optional, tag="15")]
    pub internal_id: ::std::option::Option<std::string::String>,
    // for debug purpose we store some journey internal indicators

    #[prost(uint64, optional, tag="16")]
    pub sn_dur: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="17")]
    pub transfer_dur: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="18")]
    pub min_waiting_dur: ::std::option::Option<u64>,
    #[prost(uint32, optional, tag="19")]
    pub nb_vj_extentions: ::std::option::Option<u32>,
    #[prost(uint32, optional, tag="20")]
    pub nb_sections: ::std::option::Option<u32>,
    #[prost(message, optional, tag="21")]
    pub durations: ::std::option::Option<Durations>,
    #[prost(message, optional, tag="22")]
    pub distances: ::std::option::Option<Distances>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Planner {
    #[prost(message, repeated, tag="1")]
    pub journeys: ::std::vec::Vec<Journey>,
    #[prost(enumeration="ResponseType", optional, tag="2")]
    pub response_type: ::std::option::Option<i32>,
    #[prost(string, optional, tag="3")]
    pub before: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="4")]
    pub after: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GeoStatus {
    #[prost(string, optional, tag="1")]
    pub street_network_source: ::std::option::Option<std::string::String>,
    #[prost(int32, optional, tag="2")]
    pub nb_admins: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="3")]
    pub nb_admins_from_cities: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="4")]
    pub nb_ways: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="5")]
    pub nb_addresses: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="6")]
    pub nb_poi: ::std::option::Option<i32>,
    #[prost(string, optional, tag="7")]
    pub poi_source: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Status {
    #[prost(string, required, tag="1")]
    pub publication_date: std::string::String,
    #[prost(string, required, tag="2")]
    pub start_production_date: std::string::String,
    #[prost(string, required, tag="3")]
    pub end_production_date: std::string::String,
    #[prost(int32, optional, tag="4")]
    pub data_version: ::std::option::Option<i32>,
    #[prost(string, optional, tag="6")]
    pub navitia_version: ::std::option::Option<std::string::String>,
    #[prost(string, repeated, tag="7")]
    pub data_sources: ::std::vec::Vec<std::string::String>,
    #[prost(string, optional, tag="8")]
    pub last_load_at: ::std::option::Option<std::string::String>,
    #[prost(bool, optional, tag="9")]
    pub last_load_status: ::std::option::Option<bool>,
    #[prost(bool, optional, tag="10")]
    pub loaded: ::std::option::Option<bool>,
    #[prost(int32, optional, tag="11")]
    pub nb_threads: ::std::option::Option<i32>,
    #[prost(bool, optional, tag="12")]
    pub is_connected_to_rabbitmq: ::std::option::Option<bool>,
    #[prost(string, optional, tag="13")]
    pub status: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="14")]
    pub last_rt_data_loaded: ::std::option::Option<std::string::String>,
    #[prost(bool, optional, tag="16")]
    pub is_realtime_loaded: ::std::option::Option<bool>,
    #[prost(string, optional, tag="17")]
    pub dataset_created_at: ::std::option::Option<std::string::String>,
    #[prost(string, repeated, tag="18")]
    pub rt_contributors: ::std::vec::Vec<std::string::String>,
    #[prost(bool, optional, tag="19")]
    pub disruption_error: ::std::option::Option<bool>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ScheduleStopTime {
    #[prost(message, optional, tag="2")]
    pub properties: ::std::option::Option<Properties>,
    /// date time is split because sometimes
    /// we want only time and no dates for the schedule
    /// Note: to define a null schedule time, time must be equal to it's max value (max uint64 value)
    ///
    ///time is the number of seconds since midnight
    #[prost(uint64, optional, tag="3")]
    pub time: ::std::option::Option<u64>,
    ///date is a posix time stamp to midnight
    #[prost(uint64, optional, tag="4")]
    pub date: ::std::option::Option<u64>,
    #[prost(enumeration="ResponseStatus", optional, tag="5")]
    pub dt_status: ::std::option::Option<i32>,
    #[prost(enumeration="RtLevel", optional, tag="6")]
    pub realtime_level: ::std::option::Option<i32>,
    #[prost(uint64, optional, tag="7")]
    pub base_date_time: ::std::option::Option<u64>,
    #[prost(string, repeated, tag="8")]
    pub impact_uris: ::std::vec::Vec<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RouteScheduleRow {
    #[prost(message, required, tag="1")]
    pub stop_point: StopPoint,
    #[prost(message, repeated, tag="2")]
    pub date_times: ::std::vec::Vec<ScheduleStopTime>,
    #[prost(string, repeated, tag="3")]
    pub stop_times: ::std::vec::Vec<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Header {
    #[prost(message, required, tag="1")]
    pub pt_display_informations: PtDisplayInfo,
    #[prost(enumeration="SectionAdditionalInformationType", repeated, packed="false", tag="3")]
    pub additional_informations: ::std::vec::Vec<i32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Table {
    #[prost(message, repeated, tag="1")]
    pub rows: ::std::vec::Vec<RouteScheduleRow>,
    #[prost(message, repeated, tag="2")]
    pub headers: ::std::vec::Vec<Header>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RouteSchedule {
    #[prost(message, required, tag="1")]
    pub table: Table,
    #[prost(message, required, tag="2")]
    pub pt_display_informations: PtDisplayInfo,
    #[prost(message, optional, tag="3")]
    pub geojson: ::std::option::Option<MultiLineString>,
    #[prost(enumeration="ResponseStatus", optional, tag="4")]
    pub response_status: ::std::option::Option<i32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Passage {
    #[prost(message, required, tag="1")]
    pub stop_date_time: StopDateTime,
    #[prost(message, required, tag="2")]
    pub stop_point: StopPoint,
    #[prost(message, optional, tag="3")]
    pub pt_display_informations: ::std::option::Option<PtDisplayInfo>,
    #[prost(message, optional, tag="4")]
    pub route: ::std::option::Option<Route>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RoutePoint {
    #[prost(message, optional, tag="1")]
    pub route: ::std::option::Option<Route>,
    #[prost(message, optional, tag="2")]
    pub stop_point: ::std::option::Option<StopPoint>,
    #[prost(message, optional, tag="3")]
    pub pt_display_informations: ::std::option::Option<PtDisplayInfo>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BoardItem {
    #[prost(string, required, tag="1")]
    pub hour: std::string::String,
    #[prost(string, repeated, tag="2")]
    pub minutes: ::std::vec::Vec<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DepartureBoard {
    #[prost(message, required, tag="1")]
    pub stop_point: StopPoint,
    #[prost(message, required, tag="2")]
    pub route: Route,
    #[prost(message, repeated, tag="3")]
    pub board_items: ::std::vec::Vec<BoardItem>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Load {
    #[prost(bool, required, tag="1")]
    pub ok: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Metadatas {
    #[prost(string, required, tag="1")]
    pub start_production_date: std::string::String,
    #[prost(string, required, tag="2")]
    pub end_production_date: std::string::String,
    #[prost(string, required, tag="3")]
    pub shape: std::string::String,
    #[prost(string, required, tag="4")]
    pub status: std::string::String,
    #[prost(string, repeated, tag="12")]
    pub contributors: ::std::vec::Vec<std::string::String>,
    #[prost(string, optional, tag="13")]
    pub timezone: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="14")]
    pub name: ::std::option::Option<std::string::String>,
    #[prost(uint64, optional, tag="15")]
    pub last_load_at: ::std::option::Option<u64>,
    #[prost(string, optional, tag="16")]
    pub dataset_created_at: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Pagination {
    #[prost(int32, required, tag="1")]
    pub total_result: i32,
    #[prost(int32, required, tag="2")]
    pub start_page: i32,
    #[prost(int32, required, tag="3")]
    pub items_per_page: i32,
    #[prost(int32, required, tag="4")]
    pub items_on_page: i32,
    #[prost(string, optional, tag="5")]
    pub next_page: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="6")]
    pub previous_page: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StopSchedule {
    #[prost(message, required, tag="1")]
    pub route: Route,
    #[prost(message, required, tag="2")]
    pub pt_display_informations: PtDisplayInfo,
    #[prost(message, required, tag="3")]
    pub stop_point: StopPoint,
    #[prost(message, repeated, tag="4")]
    pub date_times: ::std::vec::Vec<ScheduleStopTime>,
    #[prost(enumeration="ResponseStatus", optional, tag="5")]
    pub response_status: ::std::option::Option<i32>,
    #[prost(message, optional, tag="6")]
    pub first_datetime: ::std::option::Option<ScheduleStopTime>,
    #[prost(message, optional, tag="7")]
    pub last_datetime: ::std::option::Option<ScheduleStopTime>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Error {
    #[prost(enumeration="error::ErrorId", optional, tag="1")]
    pub id: ::std::option::Option<i32>,
    #[prost(string, optional, tag="2")]
    pub message: ::std::option::Option<std::string::String>,
}
pub mod error {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum ErrorId {
        BadFilter = 1,
        UnknownApi = 2,
        DateOutOfBounds = 3,
        UnableToParse = 4,
        BadFormat = 5,
        NoOrigin = 6,
        NoDestination = 7,
        NoOriginNorDestination = 8,
        NoSolution = 9,
        UnknownObject = 10,
        ServiceUnavailable = 11,
        InvalidProtobufRequest = 12,
        InternalError = 13,
        DeadlineExpired = 14,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TrafficReports {
    #[prost(message, optional, tag="1")]
    pub network: ::std::option::Option<Network>,
    #[prost(message, repeated, tag="2")]
    pub lines: ::std::vec::Vec<Line>,
    #[prost(message, repeated, tag="3")]
    pub stop_areas: ::std::vec::Vec<StopArea>,
    #[prost(message, repeated, tag="4")]
    pub vehicle_journeys: ::std::vec::Vec<VehicleJourney>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LineReport {
    #[prost(message, optional, tag="1")]
    pub line: ::std::option::Option<Line>,
    #[prost(message, repeated, tag="2")]
    pub pt_objects: ::std::vec::Vec<PtObject>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LinkArgs {
    /// it's uggly but since some link are computed with the protobuf, we need some dict-like
    /// structure to compute the real link url later (it depends on the API version)
    #[prost(string, optional, tag="1")]
    pub key: ::std::option::Option<std::string::String>,
    #[prost(string, repeated, tag="2")]
    pub values: ::std::vec::Vec<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Link {
    #[prost(string, optional, tag="1")]
    pub rel: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="2")]
    pub r#type: ::std::option::Option<std::string::String>,
    #[prost(bool, optional, tag="3")]
    pub is_templated: ::std::option::Option<bool>,
    #[prost(string, optional, tag="4")]
    pub description: ::std::option::Option<std::string::String>,
    #[prost(message, repeated, tag="5")]
    pub kwargs: ::std::vec::Vec<LinkArgs>,
    /// name of the linked ressource. Used to create the full url link.
    #[prost(string, optional, tag="6")]
    pub ressource_name: ::std::option::Option<std::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GraphicalIsochrone {
    #[prost(string, optional, tag="1")]
    pub geojson: ::std::option::Option<std::string::String>,
    #[prost(int32, optional, tag="2")]
    pub max_duration: ::std::option::Option<i32>,
    #[prost(int32, optional, tag="3")]
    pub min_duration: ::std::option::Option<i32>,
    #[prost(message, optional, tag="4")]
    pub origin: ::std::option::Option<PtObject>,
    #[prost(message, optional, tag="5")]
    pub destination: ::std::option::Option<PtObject>,
    #[prost(uint64, optional, tag="6")]
    pub requested_date_time: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="7")]
    pub min_date_time: ::std::option::Option<u64>,
    #[prost(uint64, optional, tag="8")]
    pub max_date_time: ::std::option::Option<u64>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HeatMap {
    #[prost(string, optional, tag="1")]
    pub heat_matrix: ::std::option::Option<std::string::String>,
    #[prost(message, optional, tag="2")]
    pub origin: ::std::option::Option<PtObject>,
    #[prost(message, optional, tag="3")]
    pub destination: ::std::option::Option<PtObject>,
    #[prost(uint64, optional, tag="4")]
    pub requested_date_time: ::std::option::Option<u64>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RoutingElement {
    #[prost(int32, required, tag="1")]
    pub duration: i32,
    #[prost(enumeration="RoutingStatus", required, tag="2")]
    pub routing_status: i32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StreetNetworkRoutingRow {
    #[prost(message, repeated, tag="2")]
    pub routing_response: ::std::vec::Vec<RoutingElement>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StreetNetworkRoutingMatrix {
    #[prost(message, repeated, tag="1")]
    pub rows: ::std::vec::Vec<StreetNetworkRoutingRow>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Response {
    #[prost(int32, optional, tag="1")]
    pub status_code: ::std::option::Option<i32>,
    #[prost(message, optional, tag="2")]
    pub error: ::std::option::Option<Error>,
    #[prost(string, optional, tag="3")]
    pub info: ::std::option::Option<std::string::String>,
    #[prost(message, optional, tag="4")]
    pub status: ::std::option::Option<Status>,
    #[prost(int32, optional, tag="5")]
    pub publication_date: ::std::option::Option<i32>,
    ///PtObjects
    #[prost(string, repeated, tag="11")]
    pub ignored_words: ::std::vec::Vec<std::string::String>,
    #[prost(string, repeated, tag="12")]
    pub bad_words: ::std::vec::Vec<std::string::String>,
    #[prost(message, repeated, tag="13")]
    pub places: ::std::vec::Vec<PtObject>,
    #[prost(message, repeated, tag="14")]
    pub places_nearby: ::std::vec::Vec<PtObject>,
    ///Ptref
    #[prost(message, repeated, tag="15")]
    pub validity_patterns: ::std::vec::Vec<ValidityPattern>,
    #[prost(message, repeated, tag="16")]
    pub lines: ::std::vec::Vec<Line>,
    #[prost(message, repeated, tag="17")]
    pub journey_patterns: ::std::vec::Vec<JourneyPattern>,
    #[prost(message, repeated, tag="18")]
    pub vehicle_journeys: ::std::vec::Vec<VehicleJourney>,
    #[prost(message, repeated, tag="19")]
    pub stop_points: ::std::vec::Vec<StopPoint>,
    #[prost(message, repeated, tag="20")]
    pub stop_areas: ::std::vec::Vec<StopArea>,
    #[prost(message, repeated, tag="21")]
    pub networks: ::std::vec::Vec<Network>,
    #[prost(message, repeated, tag="22")]
    pub physical_modes: ::std::vec::Vec<PhysicalMode>,
    #[prost(message, repeated, tag="23")]
    pub commercial_modes: ::std::vec::Vec<CommercialMode>,
    #[prost(message, repeated, tag="24")]
    pub connections: ::std::vec::Vec<Connection>,
    #[prost(message, repeated, tag="25")]
    pub journey_pattern_points: ::std::vec::Vec<JourneyPatternPoint>,
    #[prost(message, repeated, tag="26")]
    pub companies: ::std::vec::Vec<Company>,
    #[prost(message, repeated, tag="27")]
    pub routes: ::std::vec::Vec<Route>,
    #[prost(message, repeated, tag="28")]
    pub pois: ::std::vec::Vec<Poi>,
    #[prost(message, repeated, tag="29")]
    pub poi_types: ::std::vec::Vec<PoiType>,
    #[prost(message, repeated, tag="55")]
    pub calendars: ::std::vec::Vec<Calendar>,
    #[prost(message, repeated, tag="56")]
    pub line_groups: ::std::vec::Vec<LineGroup>,
    #[prost(message, repeated, tag="62")]
    pub trips: ::std::vec::Vec<Trip>,
    #[prost(message, repeated, tag="64")]
    pub contributors: ::std::vec::Vec<Contributor>,
    #[prost(message, repeated, tag="65")]
    pub datasets: ::std::vec::Vec<Dataset>,
    #[prost(message, repeated, tag="66")]
    pub route_points: ::std::vec::Vec<RoutePoint>,
    /// For api /disruptions
    #[prost(message, repeated, tag="57")]
    pub impacts: ::std::vec::Vec<Impact>,
    ///Journeys
    #[prost(message, repeated, tag="30")]
    pub journeys: ::std::vec::Vec<Journey>,
    #[prost(enumeration="ResponseType", optional, tag="31")]
    pub response_type: ::std::option::Option<i32>,
    #[prost(string, optional, tag="32")]
    pub prev: ::std::option::Option<std::string::String>,
    #[prost(string, optional, tag="33")]
    pub next: ::std::option::Option<std::string::String>,
    /// Date time for creating the next kraken call
    #[prost(uint32, optional, tag="34")]
    pub next_request_date_time: ::std::option::Option<u32>,
    ///TimeTables
    #[prost(message, repeated, tag="35")]
    pub route_schedules: ::std::vec::Vec<RouteSchedule>,
    #[prost(message, repeated, tag="36")]
    pub departure_boards: ::std::vec::Vec<DepartureBoard>,
    #[prost(message, repeated, tag="37")]
    pub next_departures: ::std::vec::Vec<Passage>,
    #[prost(message, repeated, tag="38")]
    pub next_arrivals: ::std::vec::Vec<Passage>,
    #[prost(message, repeated, tag="39")]
    pub stop_schedules: ::std::vec::Vec<StopSchedule>,
    #[prost(message, optional, tag="46")]
    pub load: ::std::option::Option<Load>,
    #[prost(message, optional, tag="48")]
    pub metadatas: ::std::option::Option<Metadatas>,
    #[prost(message, optional, tag="49")]
    pub pagination: ::std::option::Option<Pagination>,
    ///TrafficReports
    #[prost(message, repeated, tag="61")]
    pub traffic_reports: ::std::vec::Vec<TrafficReports>,
    #[prost(message, repeated, tag="73")]
    pub line_reports: ::std::vec::Vec<LineReport>,
    ///Fare
    #[prost(message, repeated, tag="51")]
    pub tickets: ::std::vec::Vec<Ticket>,
    ///Ptobject
    #[prost(message, repeated, tag="52")]
    pub pt_objects: ::std::vec::Vec<PtObject>,
    #[prost(message, repeated, tag="53")]
    pub feed_publishers: ::std::vec::Vec<FeedPublisher>,
    ///experimental
    #[prost(message, repeated, tag="63")]
    pub nearest_stop_points: ::std::vec::Vec<NearestStopPoint>,
    /// links
    /// some links computation is done in jormungandr but before json creation, so it is in the protobuf
    /// even if it is not used in kraken
    #[prost(message, repeated, tag="67")]
    pub links: ::std::vec::Vec<Link>,
    ///Isochrone
    #[prost(message, repeated, tag="68")]
    pub graphical_isochrones: ::std::vec::Vec<GraphicalIsochrone>,
    ///Heat map
    #[prost(message, repeated, tag="71")]
    pub heat_maps: ::std::vec::Vec<HeatMap>,
    #[prost(message, optional, tag="69")]
    pub geo_status: ::std::option::Option<GeoStatus>,
    /// Car CO2 emission
    #[prost(message, optional, tag="70")]
    pub car_co2_emission: ::std::option::Option<Co2Emission>,
    #[prost(message, optional, tag="72")]
    pub sn_routing_matrix: ::std::option::Option<StreetNetworkRoutingMatrix>,
    /// EquipmentReports
    #[prost(message, repeated, tag="74")]
    pub equipment_reports: ::std::vec::Vec<EquipmentReport>,
    #[prost(message, repeated, tag="75")]
    pub terminus_schedules: ::std::vec::Vec<StopSchedule>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NearestStopPoint {
    #[prost(message, optional, tag="1")]
    pub stop_point: ::std::option::Option<StopPoint>,
    #[prost(int32, optional, tag="2")]
    pub access_duration: ::std::option::Option<i32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EquipmentReport {
    #[prost(message, optional, tag="1")]
    pub line: ::std::option::Option<Line>,
    #[prost(message, repeated, tag="2")]
    pub stop_area_equipments: ::std::vec::Vec<StopAreaEquipment>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum CyclePathType {
    NoCycleLane = 0,
    /// Shared use lane (could be shared with pedestrians)
    SharedCycleWay = 1,
    /// Dedicated cycle lane
    DedicatedCycleWay = 2,
    /// A separate cycle lane (physical separation from the main carriageway
    SeparatedCycleWay = 3,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum StreetNetworkMode {
    Walking = 0,
    Bike = 1,
    Car = 3,
    Bss = 4,
    Ridesharing = 5,
    CarNoPark = 6,
    Taxi = 7,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum SectionType {
    PublicTransport = 1,
    StreetNetwork = 2,
    Waiting = 3,
    Transfer = 4,
    Boarding = 6,
    Landing = 7,
    BssRent = 8,
    BssPutBack = 9,
    CrowFly = 10,
    Park = 11,
    LeaveParking = 12,
    Alighting = 13,
    Ridesharing = 14,
    OnDemandTransport = 15,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum TransferType {
    Walking = 1,
    StayIn = 3,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum SectionAdditionalInformationType {
    OdtWithZone = 1,
    OdtWithStopPoint = 2,
    OdtWithStopTime = 3,
    HasDatetimeEstimated = 4,
    Regular = 5,
    StayIn = 6,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum GenderType {
    Female = 1,
    Male = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ResponseType {
    ItineraryFound = 1,
    DateOutOfBounds = 2,
    NoOriginPoint = 3,
    NoDestinationPoint = 4,
    NoOriginNorDestinationPoint = 5,
    NoViaPoint = 6,
    NoSolution = 7,
    ConnectionLimitation = 8,
    DurationLimitation = 9,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum RoutingStatus {
    Reached = 0,
    Unreached = 1,
    Unknown = 2,
}
