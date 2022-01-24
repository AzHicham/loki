use diesel::sql_types::NotNull;

#[derive(Debug, PartialEq, DbEnum, SqlType, AsExpression)]
#[DieselType = "impact_status"]
#[DbValueStyle = "snake_case"]
pub enum ImpactStatus {
    Published,
    Archived,
}

#[derive(Debug, PartialEq, DbEnum, SqlType, AsExpression)]
#[DieselType = "severity_effect"]
#[DbValueStyle = "snake_case"]
pub enum SeverityEffect {
    NoService,
    ReducedService,
    SignificantDelays,
    Detour,
    AdditionalService,
    ModifiedService,
    OtherEffect,
    UnknownEffect,
    StopMoved,
}

#[derive(Debug, PartialEq, DbEnum, SqlType, AsExpression)]
#[DieselType = "channel_type_enum"]
#[DbValueStyle = "snake_case"]
pub enum ChannelType {
    Web,
    Sms,
    Email,
    Mobile,
    Notification,
    Twitter,
    Facebook,
    Title,
    Beacon,
}

#[derive(Debug, PartialEq, DbEnum, SqlType, AsExpression)]
#[DieselType = "disruption_status"]
#[DbValueStyle = "snake_case"]
pub enum DisruptionStatus {
    Published,
    Archived,
    Draft,
}

#[derive(Debug, PartialEq, DbEnum, SqlType, AsExpression)]
#[DieselType = "status"]
#[DbValueStyle = "snake_case"]
pub enum Status {
    Waiting,
    Handling,
    Error,
    Done,
}

#[derive(Clone, Debug, PartialEq, DbEnum, SqlType, AsExpression)]
#[DieselType = "pt_object_type"]
#[DbValueStyle = "snake_case"]
pub enum PtObjectType {
    Network,
    StopArea,
    Line,
    LineSection,
    Route,
    StopPoint,
    RailSection,
}

#[derive(Debug, PartialEq, DbEnum, SqlType, AsExpression)]
#[DieselType = "disruption_type_enum"]
#[DbValueStyle = "snake_case"]
pub enum DisruptionType {
    Unexpected,
}
