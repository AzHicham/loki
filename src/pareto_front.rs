use crate::public_transit::PublicTransit;
use crate::journeys_tree::{Onboard, Debarked, Waiting};

use std::slice::Iter as SliceIter;

pub struct ParetoFront<Id, PT : PublicTransit> {
    elements : Vec<(Id, PT::Criteria)>
}

pub type OnboardFront<PT : PublicTransit> = ParetoFront<(Onboard, PT::Trip), PT>;
pub type DebarkedFront<PT> = ParetoFront<Debarked, PT>;
pub type WaitingFront<PT> = ParetoFront<Waiting, PT>;

impl<Id : Clone, PT : PublicTransit> Clone for ParetoFront<Id, PT> {

    fn clone(& self) -> Self {
        ParetoFront{
            elements : self.elements.clone()
        }
    }
}

impl<Id : Clone, PT : PublicTransit> ParetoFront<Id, PT> {
    pub fn new() -> Self {
        Self {
            elements : Vec::new()
        }
    }

    pub fn dominates(&self, criteria : & PT::Criteria, pt : & PT) -> bool {
        for (_, ref old_criteria) in & self.elements {
            if PublicTransit::is_lower(pt, old_criteria, criteria) {
                return true;
            }
        }
        return false;
    }

    // returns true if some element was removed because it was dominated
    //  by the added element
    pub fn add_unchecked(& mut self, id :  Id, criteria :  PT::Criteria, pt : & PT) -> bool
    {
        let old_len = self.elements.len();
        self.elements.retain(|(_, old_criteria)| {
            ! PublicTransit::is_lower(pt, &criteria, old_criteria)
        });
        let new_len = self.elements.len();
        self.elements.push((id.clone(), criteria.clone()));
        return new_len < old_len;
    }

    // returns true if some element was removed because it was dominated
    //  by the added element
    pub fn add(& mut self, id :  Id, criteria :  PT::Criteria, pt : & PT) -> bool
    {
        if self.dominates(&criteria, pt) {
            return false;
        }

        let some_element_removed = self.add_unchecked(id, criteria, pt);

        return some_element_removed;
    }

    pub fn merge_with(& mut self, other : & Self, pt : & PT) {
        for element in & other.elements {
            let id = &element.0;
            let criteria = &element.1;
            self.add(id, criteria, pt);
        }
    }

    pub fn iter(&self) -> SliceIter<'_, (Id, PT::Criteria)> {
        self.elements.iter()
    }

}