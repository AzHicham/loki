use super::{Stop, Transfer, TransitData};

use crate::timetables::Timetables as TimetablesTrait;

impl<Timetables: TimetablesTrait> TransitData<Timetables> {
    pub fn missions_of<'a>(&'a self, stop: &Stop) -> MissionsOfStop<'a, Timetables> {
        let stop_data = self.stop_data(stop);
        MissionsOfStop {
            inner: stop_data.position_in_timetables.iter(),
        }
    }

    pub fn transfers_of(&self, stop: &Stop) -> TransfersOfStop {
        let stop_data = self.stop_data(stop);
        let nb_of_transfers = stop_data.transfers.len();
        TransfersOfStop {
            stop: *stop,
            tranfer_idx_iter: 0..nb_of_transfers,
        }
    }
}

pub struct MissionsOfStop<'a, Timetables: TimetablesTrait> {
    inner: std::slice::Iter<'a, (Timetables::Mission, Timetables::Position)>,
}

impl<'a, Timetables: TimetablesTrait> Iterator for MissionsOfStop<'a, Timetables> {
    type Item = (Timetables::Mission, Timetables::Position);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().cloned()
    }
}

impl<'a, Timetables: TimetablesTrait> ExactSizeIterator for MissionsOfStop<'a, Timetables> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

use std::ops::Range;
pub struct TransfersOfStop {
    stop: Stop,
    tranfer_idx_iter: Range<usize>,
}

impl Iterator for TransfersOfStop {
    type Item = Transfer;

    fn next(&mut self) -> Option<Self::Item> {
        self.tranfer_idx_iter
            .next()
            .map(|idx_in_stop_transfers| Transfer {
                stop: self.stop,
                idx_in_stop_transfers,
            })
    }
}

impl ExactSizeIterator for TransfersOfStop {
    fn len(&self) -> usize {
        self.tranfer_idx_iter.len()
    }
}
