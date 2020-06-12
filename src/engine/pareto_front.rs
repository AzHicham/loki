use crate::engine::public_transit::PublicTransit;
use crate::engine::journeys_tree::{Onboard, Debarked, Waiting, Arrived};

use std::slice::Iter as SliceIter;
use std::vec::Drain as DrainIter;

pub struct ParetoFront<ItemData, PT : PublicTransit> {
    elements : Vec<(ItemData, PT::Criteria)>
}

pub type OnboardFront<PT> = ParetoFront<(Onboard, <PT as PublicTransit>::Trip), PT>;
pub type DebarkedFront<PT> = ParetoFront<Debarked, PT>;
pub type WaitingFront<PT> = ParetoFront<Waiting, PT>;
pub type ArrivedFront<PT> = ParetoFront<Arrived, PT>;

impl<ItemData : Clone, PT : PublicTransit> Clone for ParetoFront<ItemData, PT> {

    fn clone(& self) -> Self {
        ParetoFront{
            elements : self.elements.clone()
        }
    }
}

impl<ItemData : Clone, PT : PublicTransit> ParetoFront<ItemData, PT> {
    pub fn new() -> Self {
        Self {
            elements : Vec::new()
        }
    }

    pub fn replace_with(& mut self, other : & mut Self) {
        std::mem::swap(& mut self.elements, & mut other.elements);
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

    pub fn dominates(&self, criteria : & PT::Criteria, pt : & PT) -> bool {
        for (_, ref old_criteria) in & self.elements {
            if PublicTransit::is_lower(pt, old_criteria, criteria) {
                return true;
            }
        }
        return false;
    }


    pub fn add_unchecked(& mut self, item_data :  ItemData, criteria :  PT::Criteria)
    {
        self.elements.push((item_data, criteria));
    }

    pub fn remove_elements_dominated_by(& mut self, criteria :  & PT::Criteria, pt : & PT)  {

        self.elements.retain(|(_, old_criteria)| {
            ! PublicTransit::is_lower(pt, &criteria, old_criteria)
        });

    }

    pub fn add_and_remove_elements_dominated(& mut self, item_data :  ItemData, criteria :  PT::Criteria, pt : & PT) {
        self.remove_elements_dominated_by(&criteria, pt);
        self.add_unchecked(item_data, criteria);
    }

    pub fn add(& mut self, item_data :  ItemData, criteria :  PT::Criteria, pt : & PT)
    {
        if self.dominates(&criteria, pt) {
            return;
        }

        self.remove_elements_dominated_by(&criteria, pt);
        self.add_unchecked(item_data, criteria);
 
    }

    pub fn merge_with(& mut self, other : & Self, pt : & PT) {
        for element in & other.elements {
            let id = &element.0;
            let criteria = &element.1;
            self.add(id.clone(), criteria.clone(), pt);
        }
    }

    pub fn iter(&self) -> SliceIter<'_, (ItemData, PT::Criteria)> {
        self.elements.iter()
    }

    pub fn drain(& mut self) -> DrainIter<'_, (ItemData, PT::Criteria)> {
        self.elements.drain(..)
    }

}