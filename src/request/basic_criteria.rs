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

use std::marker::PhantomData;

use crate::{time::SecondsSinceDatasetUTCStart, PositiveDuration};

use crate::engine::engine_interface::RequestTypes;
use crate::transit_data::data_interface::Data as DataTrait;
use crate::transit_data::data_interface::TransitTypes;

pub mod depart_after;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Criteria {
    arrival_time: SecondsSinceDatasetUTCStart,
    nb_of_legs: u8,
    fallback_duration: PositiveDuration,
    transfers_duration: PositiveDuration,
}

pub struct Types<Data> {
    _phantom: PhantomData<Data>,
}

impl<'data, Data: DataTrait> TransitTypes for Types<Data> {
    type Stop = Data::Stop;

    type Mission = Data::Mission;

    type Position = Data::Position;

    type Trip = Data::Trip;

    type Transfer = Data::Transfer;
}

impl<'data, Data: DataTrait> RequestTypes for Types<Data> {
    type Departure = super::generic_request::Departure;

    type Arrival = super::generic_request::Arrival;

    type Criteria = Criteria;
}