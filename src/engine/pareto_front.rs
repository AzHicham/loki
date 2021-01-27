use crate::engine::journeys_tree::{Arrive, Board, Debark, Wait};
use crate::traits::{Request, RequestTypes, TransitTypes};

use std::slice::Iter as SliceIter;

pub struct ParetoFront<ItemData, T: RequestTypes> {
    elements: Vec<(ItemData, T::Criteria)>,
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
            .retain(|(_, old_criteria)| !request.is_lower(&criteria, old_criteria));
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
