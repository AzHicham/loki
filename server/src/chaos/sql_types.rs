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

use diesel::sql_types::NotNull;

#[derive(Debug, PartialEq, Eq, DbEnum, SqlType, AsExpression)]
#[DieselType = "impact_status"]
#[DbValueStyle = "snake_case"]
pub enum ImpactStatus {
    Published,
    Archived,
}

#[derive(Debug, PartialEq, Eq, DbEnum, SqlType, AsExpression)]
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

#[derive(Debug, PartialEq, Eq, DbEnum, SqlType, AsExpression)]
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

#[derive(Debug, PartialEq, Eq, DbEnum, SqlType, AsExpression)]
#[DieselType = "disruption_status"]
#[DbValueStyle = "snake_case"]
pub enum DisruptionStatus {
    Published,
    Archived,
    Draft,
}

#[derive(Debug, PartialEq, Eq, DbEnum, SqlType, AsExpression)]
#[DieselType = "status"]
#[DbValueStyle = "snake_case"]
pub enum Status {
    Waiting,
    Handling,
    Error,
    Done,
}

#[derive(Clone, Debug, PartialEq, Eq, DbEnum, SqlType, AsExpression)]
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

#[derive(Debug, PartialEq, Eq, DbEnum, SqlType, AsExpression)]
#[DieselType = "disruption_type_enum"]
#[DbValueStyle = "snake_case"]
pub enum DisruptionType {
    Unexpected,
}
