use super::transit_data::{Mission, Stop, Transfer, TransitData, Trip};

use super::calendar::DaysIter;

use super::timetables::{
    timetables_data::{Position, Vehicle},
    iters::{TimetablesIter, VehiclesIter}
};
use std::slice::Iter as SliceIter;


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
    transit_data: &'a TransitData,
    pattern_iter: PatternsOfStop<'a>,
    curr_pattern: Option<(StopPattern, Position, TimetablesIter)>, // None when iterator has ended
}

impl<'a> MissionsOfStop<'a> {
    pub(super) fn new(transit_data: &'a TransitData, stop: &Stop) -> Self {
        let stop_data = transit_data.stop_data(stop);
        let mut pattern_iter = stop_data.position_in_patterns.iter();
        let has_first_pattern_idx = pattern_iter.next();
        let curr_pattern = has_first_pattern_idx.map(|(pattern, position)| {
            (
                *pattern,
                *position,
                transit_data.pattern(&pattern).timetables(),
            )
        });
        Self {
            transit_data,
            pattern_iter,
            curr_pattern,
        }
    }
}

impl<'a> Iterator for MissionsOfStop<'a> {
    type Item = (Mission, Position);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((pattern, position, timetable_iter)) = &mut self.curr_pattern {
                // if there is still a timetable in this pattern, we return it
                if let Some(timetable) = timetable_iter.next() {
                    let mission = Mission {
                        stop_pattern: *pattern,
                        timetable,
                    };
                    return Some((mission, *position));
                } else {
                    // otherwise, all timetables in the current pattern have been yielded
                    match self.pattern_iter.next() {
                        None => {
                            self.curr_pattern = None;
                        }
                        Some((new_pattern, new_position)) => {
                            let new_timetable_iter =
                                self.transit_data.pattern(&new_pattern).timetables();
                            self.curr_pattern =
                                Some((*new_pattern, *new_position, new_timetable_iter));
                        }
                    }
                }
            } else {
                return None;
            }
        }
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
    mission: Mission,
    has_current_vehicle: Option<Vehicle>, // None when the iterator is exhausted
    vehicles_iter: VehiclesIter,
    days_iter: DaysIter,
}

impl TripsOfMission {
    fn new(transit_data: &TransitData, mission: &Mission) -> Self {
        let pattern = mission.stop_pattern;
        let pattern_data = &transit_data.pattern(&pattern);

        let mut vehicles_iter = pattern_data.vehicles(&mission.timetable);
        let has_current_vehicle = vehicles_iter.next();
        let days_iter = transit_data.calendar.days();

        Self {
            mission: mission.clone(),
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
                            mission: self.mission.clone(),
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
