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

use crate::{
    engine::{
        engine_interface::{Request, RequestTypes},
        journeys_tree::{Arrive, Board, Debark, Wait},
    },
    transit_data::data_interface::TransitTypes,
};

use std::{fmt::Debug, slice::Iter as SliceIter};

pub struct ParetoFront<ItemData, T: RequestTypes> {
    elements: Vec<(ItemData, T::Criteria)>,
}

impl<ItemData, T: RequestTypes> Debug for ParetoFront<ItemData, T>
where
    ItemData: Debug,
    T::Criteria: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParetoFront")
            .field("elements", &self.elements)
            .finish()
    }
}

pub type BoardFront<T> = ParetoFront<(Board, <T as TransitTypes>::Trip), T>;
pub type DebarkFront<T> = ParetoFront<Debark, T>;
pub type WaitFront<T> = ParetoFront<Wait, T>;
pub type ArriveFront<T> = ParetoFront<Arrive, T>;

impl<ItemData: Clone, T: RequestTypes> Clone for ParetoFront<ItemData, T> {
    fn clone(&self) -> Self {
        ParetoFront {
            elements: self.elements.clone(),
        }
    }
}

impl<ItemData: Clone, T: RequestTypes> ParetoFront<ItemData, T> {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    pub fn replace_with(&mut self, other: &mut Self) {
        std::mem::swap(&mut self.elements, &mut other.elements);
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn clear(&mut self) {
        self.elements.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn dominates<R>(&self, criteria: &T::Criteria, request: &R) -> bool
    where
        R: Request<Criteria = T::Criteria>,
    {
        for (_, ref old_criteria) in &self.elements {
            if request.is_lower(old_criteria, criteria) {
                return true;
            }
        }
        false
    }

    pub fn add_unchecked(&mut self, item_data: ItemData, criteria: T::Criteria) {
        self.elements.push((item_data, criteria));
    }

    pub fn remove_elements_dominated_by<R>(&mut self, criteria: &T::Criteria, request: &R)
    where
        R: Request<Criteria = T::Criteria>,
    {
        self.elements
            .retain(|(_, old_criteria)| !request.is_lower(criteria, old_criteria));
    }

    pub fn remove_elements_that_can_be_discarded_by<R>(
        &mut self,
        criteria: &T::Criteria,
        request: &R,
    ) where
        R: Request<Criteria = T::Criteria>,
    {
        self.elements
            .retain(|(_, old_criteria)| !request.can_be_discarded(old_criteria, criteria));
    }

    pub fn add_and_remove_elements_dominated<R>(
        &mut self,
        item_data: ItemData,
        criteria: T::Criteria,
        request: &R,
    ) where
        R: Request<Criteria = T::Criteria>,
    {
        self.remove_elements_dominated_by(&criteria, request);
        self.add_unchecked(item_data, criteria);
    }

    pub fn add<R>(&mut self, item_data: ItemData, criteria: T::Criteria, request: &R)
    where
        R: Request<Criteria = T::Criteria>,
    {
        if self.dominates(&criteria, request) {
            return;
        }

        self.remove_elements_dominated_by(&criteria, request);
        self.add_unchecked(item_data, criteria);
    }

    pub fn iter(&self) -> SliceIter<'_, (ItemData, T::Criteria)> {
        self.elements.iter()
    }
}
