use crate::engine::journeys_tree::{Arrive, Board, Debark, Wait};
use crate::engine::public_transit::PublicTransit;

use std::slice::Iter as SliceIter;

pub struct ParetoFront<ItemData, PT: PublicTransit> {
    elements: Vec<(ItemData, PT::Criteria)>,
}

pub type BoardFront<PT> = ParetoFront<(Board, <PT as PublicTransit>::Trip), PT>;
pub type DebarkFront<PT> = ParetoFront<Debark, PT>;
pub type WaitFront<PT> = ParetoFront<Wait, PT>;
pub type ArriveFront<PT> = ParetoFront<Arrive, PT>;

impl<ItemData: Clone, PT: PublicTransit> Clone for ParetoFront<ItemData, PT> {
    fn clone(&self) -> Self {
        ParetoFront {
            elements: self.elements.clone(),
        }
    }
}

impl<ItemData: Clone, PT: PublicTransit> ParetoFront<ItemData, PT> {
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

    pub fn dominates(&self, criteria: &PT::Criteria, pt: &PT) -> bool {
        for (_, ref old_criteria) in &self.elements {
            if PublicTransit::is_lower(pt, old_criteria, criteria) {
                return true;
            }
        }
        return false;
    }

    pub fn add_unchecked(&mut self, item_data: ItemData, criteria: PT::Criteria) {
        self.elements.push((item_data, criteria));
    }

    pub fn remove_elements_dominated_by(&mut self, criteria: &PT::Criteria, pt: &PT) {
        self.elements
            .retain(|(_, old_criteria)| !PublicTransit::is_lower(pt, &criteria, old_criteria));
    }

    pub fn add_and_remove_elements_dominated(
        &mut self,
        item_data: ItemData,
        criteria: PT::Criteria,
        pt: &PT,
    ) {
        self.remove_elements_dominated_by(&criteria, pt);
        self.add_unchecked(item_data, criteria);
    }

    pub fn add(&mut self, item_data: ItemData, criteria: PT::Criteria, pt: &PT) {
        if self.dominates(&criteria, pt) {
            return;
        }

        self.remove_elements_dominated_by(&criteria, pt);
        self.add_unchecked(item_data, criteria);
    }

    pub fn iter(&self) -> SliceIter<'_, (ItemData, PT::Criteria)> {
        self.elements.iter()
    }
}
