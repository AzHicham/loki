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

use std::collections::HashMap;

use tracing::log::error;

use crate::{models::VehicleJourneyIdx, time::{DaysSinceDatasetStart, days_map::{DaysMap, InsertError}, days_patterns::{DaysPattern, DaysPatterns}}};

use super::{generic_timetables::Timetable};


pub struct VehicleJourneyToTimetable {
    data : HashMap<VehicleJourneyIdx, DayToTimetable>
}

struct DayToTimetable {
    data : DaysMap<VehicleTimetable>,
    
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum VehicleTimetable {
    BaseAndRealTime(Timetable),
    SplittedBaseRealTime(Timetable, Timetable),
    RealTimeOnly(Timetable),
    BaseOnly(Timetable)
}


impl DayToTimetable {
    fn new() -> Self {
        Self {
            data : DaysMap::new(),
            
        }
    }

    // pub fn insert_base_and_realtime_vehicle(
    //     &mut self,
    //     vehicle_journey_idx: &VehicleJourneyIdx,
    //     days_pattern_to_insert: &DaysPattern,
    //     timetable_to_insert: &Timetable,
    //     days_patterns: &mut DaysPatterns,
    // ) -> Result<(), InsertError> 
    // {
    //     let to_insert = VehicleTimetable::BaseAndRealTime(timetable_to_insert.clone());
    //     self.data.insert(days_pattern_to_insert, to_insert, days_patterns)
    // }

    // pub fn insert_real_time_only_vehicle(&mut self,
    //     vehicle_journey_idx: &VehicleJourneyIdx,
    //     days_pattern_to_insert: &DaysPattern,
    //     timetable_to_insert: &Timetable,
    //     days_patterns: &mut DaysPatterns,
    // ) -> Result<(), InsertError> {
    //     self.status.insert(days_pattern_to_insert, Status::BaseIsRealTime, days_patterns)?;
    //     let result = self.real_time.insert(days_pattern_to_insert, timetable_to_insert.clone(), days_patterns);
    //     if result.is_err() {
    //        error!("Data not in sync between status and real_time.");
    //        Err(InsertError::DayAlreadySet)
    //     }
    //     else {
    //         Ok(())
    //     }

    // }

    // pub fn remove_real_time_vehicle(
    //     &mut self,
    //     day: &DaysSinceDatasetStart,
    //     days_patterns: &mut DaysPatterns,
    // ) -> Result<Timetable, Unknown> {
    //     let status = self.status.get(day, days_patterns).ok_or(Unknown::DayForVehicleJourney)?;
    //     match status {
    //         Status::BaseOnly => {
    //             return Err(Unknown::DayForVehicleJourney);
    //         },
    //         Status::BaseIsRealTime => {
    //             self.status.remove(day, days_patterns);
    //             let days_pattern = days_patterns.get_from_days(std::iter::once(*day));
    //             let result = self.status.insert(&days_pattern, Status::BaseOnly, days_patterns);

    //         },
    //         Status::SplittedBaseRealTime 
    //         | Status::RealTimeOnly => {

    //         },
    //     };
    //     Ok(())
    // }




    // fn get(&self, day : &DaysSinceDatasetStart, days_patterns : & DaysPatterns) -> Option<VehicleTimetable> {
    //     let status = self.status.get(day, days_patterns)?;
    //     match status {
    //         Status::BaseOnly => {
    //             if let Some(timetable) = self.base.get(day, days_patterns) {
    //                 let result = VehicleTimetable::BaseOnly(timetable.clone());
    //                 Some(result)
    //             }
    //             else {
    //                 error!("Data not in sync between status and base.");
    //                 None
    //             }
    //         },
    //         Status::RealTimeOnly => {
    //             if let Some(timetable) = self.real_time.get(day, days_patterns) {
    //                 let result = VehicleTimetable::RealTimeOnly(timetable.clone());
    //                 Some(result)
    //             }
    //             else {
    //                 error!("Data not in sync between status  and real_time.");
    //                 None
    //             }
    //         },
    //         Status::BaseIsRealTime => {
    //             if let Some(timetable) = self.base.get(day, days_patterns) {
    //                 let result = VehicleTimetable::BaseAndRealTime(timetable.clone());
    //                 Some(result)
    //             }
    //             else {
    //                 error!("Data not in sync between status and base.");
    //                 None
    //             }
    //         },
    //         Status::SplittedBaseRealTime => {
    //             if let Some(base_timetable) = self.base.get(day, days_patterns) {
    //                 if let Some(real_time_timetable) = self.real_time.get(day, days_patterns) {
    //                     let result = VehicleTimetable::SplittedBaseRealTime(base_timetable.clone(), real_time_timetable.clone());
    //                     Some(result)
    //                 }
    //                 else {
    //                     error!("Data not in sync between status  and real_time.");
    //                     None
    //                 }
    //             }
    //             else {
    //                 error!("Data not in sync between status and base.");
    //                 None
    //             }
    //         }
    //     }
    // }
}

impl VehicleJourneyToTimetable {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn insert_base_and_realtime_vehicle(
        &mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        days_pattern_to_insert: &DaysPattern,
        timetable_to_insert: &Timetable,
        days_patterns: &mut DaysPatterns,
    ) -> Result<(), InsertError>
    {
        let day_to_timetable = self.data.entry(*vehicle_journey_idx).or_insert_with(|| DayToTimetable::new());

        let value_to_insert = VehicleTimetable::BaseAndRealTime(timetable_to_insert.clone());
        day_to_timetable.data.insert(days_pattern_to_insert, value_to_insert, days_patterns)
        
      
    }

    pub fn insert_real_time_only_vehicle(&mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        days_pattern_to_insert: &DaysPattern,
        timetable_to_insert: &Timetable,
        days_patterns: &mut DaysPatterns,
    ) -> Result<(), InsertError> {
        let day_to_timetable = self.data.entry(*vehicle_journey_idx).or_insert_with(|| DayToTimetable::new());

        let value_to_insert = VehicleTimetable::RealTimeOnly(timetable_to_insert.clone());
        day_to_timetable.data.insert(days_pattern_to_insert, value_to_insert, days_patterns)
        
    }

    pub fn remove_real_time_vehicle(
        &mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        day: &DaysSinceDatasetStart,
        days_patterns: &mut DaysPatterns,
    ) -> Result<(), Unknown> {
        let day_to_timetable = self.data.get(vehicle_journey_idx).ok_or(Unknown::VehicleJourneyIdx)?;
        let old_timetable = day_to_timetable.data.get(day, days_patterns).ok_or(Unknown::DayForVehicleJourney)?;
        match old_timetable {
            VehicleTimetable::BaseOnly(_) => {return Err(Unknown::DayForVehicleJourney);},
            VehicleTimetable::BaseAndRealTime(timetable) => {
                day_to_timetable.data.remove(day, days_patterns).map_err(|_|Unknown::DayForVehicleJourney);
                let days_pattern_to_insert = days_patterns.get_from_days(std::iter::once(*day));
                let value_to_insert = VehicleTimetable::BaseOnly(timetable.clone());
                day_to_timetable.data.insert(&days_pattern_to_insert, value_to_insert, days_patterns).map_err(|_|Unknown::DayForVehicleJourney)

            },
            VehicleTimetable::SplittedBaseRealTime(_, timetable)  => {
                day_to_timetable.data.remove(day, days_patterns)
                .map_err(|_|Unknown::DayForVehicleJourney)?;
                let days_pattern_to_insert = days_patterns.get_from_days(std::iter::once(*day));
                let value_to_insert = VehicleTimetable::BaseOnly(timetable.clone());
                day_to_timetable.data.insert(&days_pattern_to_insert, value_to_insert, days_patterns).map_err(|_|Unknown::DayForVehicleJourney)

            }
            VehicleTimetable::RealTimeOnly(timetable) => {
                day_to_timetable.data.remove(day, days_patterns)
                    .map(|_| ())
                    .map_err(|_|Unknown::DayForVehicleJourney)
                
            }
            
        }
        
    }

    pub fn update(
        &mut self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        day: &DaysSinceDatasetStart,
        timetable_to_insert : & Timetable,
        days_patterns: &mut DaysPatterns,
    ) -> Result<(), Unknown> {
        let day_to_timetable = self.data.get(vehicle_journey_idx).ok_or(Unknown::VehicleJourneyIdx)?;
        let old_timetable = day_to_timetable.data.get(day, days_patterns).ok_or(Unknown::DayForVehicleJourney)?;
        match old_timetable {
            VehicleTimetable::BaseOnly(timetable) => {return Err(Unknown::DayForVehicleJourney);},
            VehicleTimetable::BaseAndRealTime(base_timetable) => {
                day_to_timetable.data.remove(day, days_patterns).map_err(|_|Unknown::DayForVehicleJourney);
                let days_pattern_to_insert = days_patterns.get_from_days(std::iter::once(*day));
                let value_to_insert = VehicleTimetable::SplittedBaseRealTime(*base_timetable, *timetable_to_insert);
                day_to_timetable.data.insert(&days_pattern_to_insert, value_to_insert, days_patterns).map_err(|_|Unknown::DayForVehicleJourney)

            },
            VehicleTimetable::SplittedBaseRealTime(base_timetable, _) => {
                day_to_timetable.data.remove(day, days_patterns)
                    .map_err(|_|Unknown::DayForVehicleJourney)?;
                let days_pattern_to_insert = days_patterns.get_from_days(std::iter::once(*day));
                let value_to_insert = VehicleTimetable::SplittedBaseRealTime(*base_timetable, timetable_to_insert.clone());
                day_to_timetable.data.insert(&days_pattern_to_insert, value_to_insert, days_patterns).map_err(|_|Unknown::DayForVehicleJourney)
            }
            VehicleTimetable::RealTimeOnly(timetable) => {
                day_to_timetable.data.remove(day, days_patterns)
                    .map_err(|_|Unknown::DayForVehicleJourney)?;
                let days_pattern_to_insert = days_patterns.get_from_days(std::iter::once(*day));
                let value_to_insert = VehicleTimetable::RealTimeOnly(*timetable_to_insert);
                day_to_timetable.data.insert(&days_pattern_to_insert, value_to_insert, days_patterns).map_err(|_|Unknown::DayForVehicleJourney)
                
            }
            
        }

    }



    pub fn get_timetable(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        day: &DaysSinceDatasetStart,
        days_patterns: &DaysPatterns,
    ) -> Result<&VehicleTimetable, Unknown> {
        let day_to_timetable = self.data.get(vehicle_journey_idx)
                .ok_or_else(|| Unknown::VehicleJourneyIdx) ?;
        day_to_timetable.data.get(day, days_patterns).ok_or_else(|| Unknown::DayForVehicleJourney)
 
    }



}

pub enum RemovalError {

}

#[derive(Debug)]
pub enum Unknown {
    VehicleJourneyIdx,
    DayForVehicleJourney,
}

