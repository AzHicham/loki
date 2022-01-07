// In sql_types.rs
use diesel::sql_types::NotNull;

#[derive(Debug, PartialEq, DbEnum, SqlType)]
#[DieselType = "impact_status"]
pub enum Impact_status {
    Admin,
    Employee,
}

#[derive(Debug, PartialEq, DbEnum, SqlType)]
#[DieselType = "severity_effect"]
pub enum Severity_effect {
    Admin,
    Employee,
}

#[derive(Debug, PartialEq, DbEnum, SqlType)]
#[DieselType = "channel_type_enum"]
pub enum Channel_type_enum {
    Admin,
    Employee,
}

#[derive(Debug, PartialEq, DbEnum, SqlType)]
#[DieselType = "disruption_status"]
pub enum Disruption_status {
    Admin,
    Employee,
}

#[derive(Debug, PartialEq, DbEnum, SqlType)]
#[DieselType = "status"]
pub enum Status {
    Admin,
    Employee,
}

#[derive(Debug, PartialEq, DbEnum, SqlType)]
#[DieselType = "pt_object_type"]
pub enum Pt_object_type {
    Admin,
    Employee,
}
#[derive(Debug, PartialEq, DbEnum, SqlType)]
#[DieselType = "disruption_type_enum"]
pub enum Disruption_type_enum {
    Admin,
    Employee,
}
