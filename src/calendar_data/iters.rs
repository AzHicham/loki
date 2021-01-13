use super::transit_data::{Mission, Stop, Transfer, TransitData, Trip};

use crate::time::DaysIter;

use super::timetables::{
    timetables_data::{Position, Vehicle},
    iters::{VehiclesIter}
};



impl TransitData {
    pub fn missions_of<'a>(&'a self, stop: &Stop) -> MissionsOfStop<'a> {
        MissionsOfStop::new(&self, stop)
    }

    pub fn trips_of(&self, mission: &Mission) -> TripsOfMission {
        TripsOfMission::new(&self, mission)
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

pub struct MissionsOfStop<'a> {
    positions : std::slice::Iter<'a, Position>,

}

impl<'a> MissionsOfStop<'a> {
    pub(super) fn new(transit_data: &'a TransitData, stop: &Stop) -> Self {
        let stop_data = transit_data.stop_data(stop);
        let positions = stop_data.position_in_timetables.iter();
        Self {
            positions
        }
    }
}

impl<'a> Iterator for MissionsOfStop<'a> {
    type Item = (Mission, Position);

    fn next(&mut self) -> Option<Self::Item> {
       self.positions.next().map(|position| {
           let mission = Mission {
               timetable : position.timetable.clone()
           };
           (mission, position.clone())
       })
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

pub struct TripsOfMission {
    has_current_vehicle : Option<Vehicle>,
    vehicles_iter: VehiclesIter,
    days_iter: DaysIter,
}

impl TripsOfMission {
    fn new(transit_data: &TransitData, mission: &Mission) -> Self {
        let mut vehicles_iter = transit_data.timetables.vehicles(&mission.timetable);
        let has_current_vehicle = vehicles_iter.next();
        let days_iter = transit_data.calendar.days();

        Self {
            has_current_vehicle,
            vehicles_iter,
            days_iter,
        }
    }
}

impl Iterator for TripsOfMission {
    type Item = Trip;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(current_vehicle) = &mut self.has_current_vehicle {
                match self.days_iter.next() {
                    Some(day) => {
                        let trip = Trip {
                            vehicle: current_vehicle.clone(),
                            day,
                        };
                        return Some(trip);
                    }
                    None => {
                        self.has_current_vehicle = self.vehicles_iter.next();
                    }
                }
            } else {
                return None;
            }
        }
    }
}
