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


use chrono::NaiveDate;

use crate::model::real_time::TripData;

use super::{ModelRefs, StopPointIdx, TransitModelVehicleJourneyIdx, VehicleJourneyIdx, real_time};

#[derive(Debug, Clone)]
pub struct Coord {
    pub lat : f64,
    pub lon : f64,
}

#[derive(Debug, Clone)]
pub enum StopTimes<'model> {
    Base(& 'model [transit_model::objects::StopTime], NaiveDate, chrono_tz::Tz),
    New(&'model [real_time::StopTime], NaiveDate) 
}



impl<'model> ModelRefs<'model> {

    pub fn stop_point_uri(&self,  stop_point_idx : & StopPointIdx) -> String {
        format!("stop_point:{}", self.stop_point_name(stop_point_idx))
    }

    pub fn house_numer(&self, stop_point_idx : & StopPointIdx) -> Option<&'model str> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => {
                let stop_point = &self.base.stop_points[*idx];
                let has_address_id = &stop_point.address_id;
                if let Some(address_id) = has_address_id {
                    let address = &self.base.addresses.get(&address_id)?;
                    Some(address.street_name.as_str())
                }
                else {
                    None
                }

            },
            StopPointIdx::New(_) => None,
        }
    }

    pub fn street_name(&self, stop_point_idx : & StopPointIdx) -> Option<&'model str> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => {
                let stop_point = &self.base.stop_points[*idx];
                let has_address_id = &stop_point.address_id;
                if let Some(address_id) = has_address_id {
                    let address = &self.base.addresses.get(&address_id)?;
                    Some(address.street_name.as_str())
                }
                else {
                    None
                }
            },
            StopPointIdx::New(_) => None,
        }
    }


    pub fn coord(&self, stop_point_idx : & StopPointIdx) -> Option<Coord> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => {
                let stop_point = &self.base.stop_points[*idx];
                let coord = Coord {
                    lat : stop_point.coord.lat,
                    lon : stop_point.coord.lon,
                };
                Some(coord)


            },
            StopPointIdx::New(_) => None,
        }
    }

    pub fn platform_code(&self, stop_point_idx : & StopPointIdx) -> Option<&str> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => {
                let stop_point = &self.base.stop_points[*idx];
                let has_platform_code = &stop_point.platform_code;
                has_platform_code.as_ref()
                    .map(|s| s.as_str())


            },
            StopPointIdx::New(_) => None,
        }
    }

    pub fn fare_zone_id(&self, stop_point_idx : & StopPointIdx) -> Option<&str> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => {
                let stop_point = &self.base.stop_points[*idx];
                stop_point.fare_zone_id.as_ref()
                    .map(|s| s.as_str())
            },
            StopPointIdx::New(_) => None,
        }
    } 

    pub fn stop_area(&self, stop_area_id : &str) -> Option<&transit_model::objects::StopArea> {
        self.base.stop_areas.get(stop_area_id)
    }

    pub fn codes(&self, stop_point_idx : & StopPointIdx) -> Option<impl Iterator<Item = & (String, String)> + '_> {
        match stop_point_idx {
            StopPointIdx::Base(idx) => {
                let stop_point = &self.base.stop_points[*idx];
                Some(stop_point.codes.iter())
            },
            StopPointIdx::New(_) => {
                None
            },
        }
    }

    pub fn timezone(&self, vehicle_journey_idx : & VehicleJourneyIdx, date : &NaiveDate) -> chrono_tz::Tz {
        if let VehicleJourneyIdx::Base(idx) = vehicle_journey_idx {
            if self.real_time.base_vehicle_journey_last_version(idx, date).is_none() {
                return self.base_vehicle_journey_timezone(idx)
                    .unwrap_or(chrono_tz::UTC);
            }
        }
        return chrono_tz::UTC;
    }

    fn base_vehicle_journey_timezone(&self, idx : & TransitModelVehicleJourneyIdx) -> Option<chrono_tz::Tz> {
        let route_id = &self.base.vehicle_journeys[*idx].route_id;
        let route = self.base.routes.get(route_id)?;
        let line = self.base.lines.get(&route.line_id)?;
        let network = self.base.networks.get(&line.network_id)?;
        network.timezone
    }

    pub fn stop_times(&self, 
        vehicle_journey_idx : & VehicleJourneyIdx,
        date : & NaiveDate, 
        from_stoptime_idx : usize,
        to_stoptime_idx : usize
    ) -> Option<StopTimes> {
        if from_stoptime_idx > to_stoptime_idx {
            return None;
        }
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let has_history = self.real_time.base_vehicle_journey_last_version(idx, date);
                match has_history {
                    Some(TripData::Present(stop_times)) => {
                        if from_stoptime_idx < stop_times.len() && to_stoptime_idx < stop_times.len() {
                            Some(StopTimes::New(&stop_times[from_stoptime_idx..=to_stoptime_idx], date.clone()))
                        }
                        else {
                            None
                        }
                    },
                    Some(TripData::Deleted()) => {
                        None
                    }
                    None => {
                        let vj = &self.base.vehicle_journeys[*idx];
                        let stop_times = &vj.stop_times;
                        let timezone = self.timezone(vehicle_journey_idx, date);
                        if from_stoptime_idx < stop_times.len() && to_stoptime_idx < stop_times.len() {
                            Some(StopTimes::Base(&stop_times[from_stoptime_idx..=to_stoptime_idx], date.clone(), timezone))
                        }
                        else {
                            None
                        }
                    }
                }
            },
            VehicleJourneyIdx::New(idx) =>  {
                let trip_data = self.real_time.new_vehicle_journey_last_version(idx, date)?;
                if let TripData::Present(stop_times) = trip_data {
                    Some(StopTimes::New(stop_times.as_slice(), date.clone()))
                }
                else {
                    None
                }
                   
            },
        }
    }

    pub fn line_code(&self, vehicle_journey_idx : & VehicleJourneyIdx) -> Option<&str> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                self.base_vehicle_journey_line(*idx)
                    .map(|line| line.code.as_ref())
                    .flatten()
                    .map(|s| s.as_str())
            },
            VehicleJourneyIdx::New(_) => None,
        }
        
    }

    pub fn headsign(&self, vehicle_journey_idx : & VehicleJourneyIdx ,date : & NaiveDate) -> Option<&str> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let has_history = self.real_time.base_vehicle_journey_last_version(idx, date);
                if has_history.is_none() {
                    self.base.vehicle_journeys[*idx].headsign
                        .as_ref()
                        .map(|s| s.as_str())
                }
                else {
                    None
                }
               
            },
            VehicleJourneyIdx::New(_idx) => None,
        }
        
    }

    pub fn direction(&self, vehicle_journey_idx : & VehicleJourneyIdx ,date : & NaiveDate) -> Option<&str> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let has_history = self.real_time.base_vehicle_journey_last_version(idx, date);
                if has_history.is_none() {
                    let route = self.base_vehicle_journey_route(idx)?;
                    route.destination_id
                    .as_ref()
                    .and_then(|destination_id| self.base.stop_areas.get(&destination_id))
                    .map(|stop_area| stop_area.name.as_str())
                }
                else {
                    None
                }
               
            },
            VehicleJourneyIdx::New(_idx) => None,
        }
    }

    pub fn line_color(&self, vehicle_journey_idx : & VehicleJourneyIdx ,date : & NaiveDate) -> Option<&transit_model::objects::Rgb> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let has_history = self.real_time.base_vehicle_journey_last_version(idx, date);
                if has_history.is_none() {
                   let line = self.base_vehicle_journey_line(*idx)?;
                    line.color.as_ref()
                }
                else {
                    None
                }
               
            },
            VehicleJourneyIdx::New(_idx) => None,
        }
    }

    pub fn text_color(&self, vehicle_journey_idx : & VehicleJourneyIdx ,date : & NaiveDate) -> Option<&transit_model::objects::Rgb> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let has_history = self.real_time.base_vehicle_journey_last_version(idx, date);
                if has_history.is_none() {
                   let line = self.base_vehicle_journey_line(*idx)?;
                    line.text_color.as_ref()
                }
                else {
                    None
                }
               
            },
            VehicleJourneyIdx::New(_idx) => None,
        }
    }


    pub fn trip_short_name(&self, vehicle_journey_idx : & VehicleJourneyIdx ,date : & NaiveDate) -> Option<&str> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let has_history = self.real_time.base_vehicle_journey_last_version(idx, date);
                if has_history.is_none() {
                   let vj = &self.base.vehicle_journeys[*idx];
                   vj
                   .short_name
                   .as_deref()
                   .or_else(|| vj.headsign.as_deref())
                }
                else {
                    None
                }
               
            },
            VehicleJourneyIdx::New(_idx) => None,
        }
    }

}