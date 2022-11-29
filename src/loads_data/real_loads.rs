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

use chrono::NaiveDate;
use std::{collections::BTreeMap, error::Error, fmt::Display, io};
use tracing::{debug, info, trace};

type Occupancy = u8;

use crate::models::{
    base_model::{self, BaseModel, BaseVehicleJourneyIdx},
    StopSequence, StopTimeIdx, VehicleJourneyIdx,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Load {
    Low,
    Medium,
    High,
}

impl Display for Load {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Load::Low => write!(f, "Low"),
            Load::Medium => write!(f, "Medium"),
            Load::High => write!(f, "High"),
        }
    }
}

impl Default for Load {
    fn default() -> Self {
        Load::Medium
    }
}

use std::cmp::Ordering;

fn load_to_int(load: &Load) -> u8 {
    match load {
        Load::Low => 0,
        Load::Medium => 1,
        Load::High => 2,
    }
}

impl Ord for Load {
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&load_to_int(self), &load_to_int(other))
    }
}

impl PartialOrd for Load {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LoadsCount {
    pub high: u16,
    pub medium: u16,
    pub low: u16,
}

impl LoadsCount {
    pub fn zero() -> Self {
        Self {
            high: 0,
            medium: 0,
            low: 0,
        }
    }

    pub fn add(&self, load: Load) -> Self {
        let mut high = self.high;
        let mut medium = self.medium;
        let mut low = self.low;
        match load {
            Load::High => {
                high += 1;
            }
            Load::Medium => {
                medium += 1;
            }
            Load::Low => {
                low += 1;
            }
        }
        Self { high, medium, low }
    }

    pub fn total(&self) -> u16 {
        self.high + self.medium + self.low
    }

    pub fn max(&self) -> Load {
        if self.high > 0 {
            return Load::High;
        }
        if self.medium > 0 {
            return Load::Medium;
        }
        Load::Low
    }

    pub fn is_lower(&self, other: &Self) -> bool {
        use Ordering::{Equal, Greater, Less};
        match self.high.cmp(&other.high) {
            Less => true,
            Greater => false,
            Equal => match self.medium.cmp(&other.medium) {
                Less => true,
                Greater => false,
                Equal => match self.low.cmp(&other.low) {
                    Less | Equal => true,
                    Greater => false,
                },
            },
        }
    }
}

impl Default for LoadsCount {
    fn default() -> Self {
        Self::zero()
    }
}

impl Display for LoadsCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "High {}; Medium {}; Low {}; total {}",
            self.high,
            self.medium,
            self.low,
            self.total()
        )
    }
}

fn occupancy_to_load(occupancy: Occupancy) -> Load {
    debug_assert!(occupancy <= 100);
    if occupancy <= 30 {
        Load::Low
    } else if occupancy <= 70 {
        Load::Medium
    } else {
        Load::High
    }
}

pub struct LoadsData {
    per_vehicle_journey: BTreeMap<BaseVehicleJourneyIdx, VehicleJourneyLoads>,
}

struct VehicleJourneyLoads {
    stop_sequence_to_idx: BTreeMap<StopSequence, usize>,
    per_date: BTreeMap<NaiveDate, TripLoads>,
}

struct TripLoads {
    per_stop: Vec<Load>,
}

impl VehicleJourneyLoads {
    fn new<StopSequenceIter>(stop_sequence_iter: StopSequenceIter) -> Self
    where
        StopSequenceIter: Iterator<Item = StopSequence>,
    {
        let mut stop_sequence_to_idx = BTreeMap::new();
        for (idx, stop_sequence) in stop_sequence_iter.enumerate() {
            stop_sequence_to_idx.insert(stop_sequence, idx);
        }
        Self {
            stop_sequence_to_idx,
            per_date: BTreeMap::new(),
        }
    }
}

impl TripLoads {
    fn new(nb_of_stop: usize) -> Self {
        Self {
            per_stop: vec![Load::Medium; nb_of_stop],
        }
    }
}

impl LoadsData {
    pub fn loads(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        date: &NaiveDate,
    ) -> Option<&[Load]> {
        match vehicle_journey_idx {
            VehicleJourneyIdx::Base(idx) => {
                let vehicle_journey_load = self.per_vehicle_journey.get(idx)?;
                let trip_load = vehicle_journey_load.per_date.get(date)?;
                let nb_of_stops = trip_load.per_stop.len();
                Some(&trip_load.per_stop[..(nb_of_stops - 1)])
            }
            VehicleJourneyIdx::New(_) => None,
        }
    }

    pub fn load(
        &self,
        vehicle_journey_idx: &VehicleJourneyIdx,
        stop_time_idx: StopTimeIdx,
        date: &NaiveDate,
    ) -> Option<Load> {
        self.loads(vehicle_journey_idx, date)
            .map(|loads| loads[stop_time_idx.idx])
    }

    pub fn empty() -> Self {
        LoadsData {
            per_vehicle_journey: BTreeMap::new(),
        }
    }

    #[cfg(feature = "demo_occupancy")]
    pub fn fake_occupancy_metro1_rera(model: &base_model::Model) -> Result<Self, Box<dyn Error>> {
        use transit_model::objects::{Line, Network};
        tracing::info!("loading fake vehicle occupancy for Metro 1 (RATP) and RER A (RER)");
        let mut loads_data = LoadsData {
            per_vehicle_journey: BTreeMap::new(),
        };
        let get_line_id = |network_name: &'static str, line_code: &'static str| {
            let network_idx = model
                .networks
                .iter()
                .find(|(_, network)| network.name == network_name)
                .map(|(idx, _)| idx)?;
            model
                .get_corresponding_from_idx::<Network, Line>(network_idx)
                .into_iter()
                .find(|line_idx| model.lines[*line_idx].code == Some(line_code.to_string()))
        };
        let mut line_loads = Vec::new();
        let rera_idx = get_line_id("RER", "A");
        if let Some(rera_idx) = rera_idx {
            trace!("loading vehicle occupancy data for RER A");
            line_loads.push((model.lines[rera_idx].id.clone(), Load::High));
        }
        let metro1_idx = get_line_id("RATP", "1");
        if let Some(metro1_idx) = metro1_idx {
            trace!("loading vehicle occupancy data for Metro 1");
            line_loads.push((model.lines[metro1_idx].id.clone(), Load::Medium));
        }
        for (line_id, load) in line_loads {
            let line_idx = if let Some(line_idx) = model.lines.get_idx(&line_id) {
                line_idx
            } else {
                continue;
            };
            for vehicle_journey_idx in model.get_corresponding_from_idx(line_idx) {
                let vehicle_journey = &model.vehicle_journeys[vehicle_journey_idx];
                let stop_sequence_iter = vehicle_journey
                    .stop_times
                    .iter()
                    .map(|stop_time| stop_time.sequence);
                let nb_of_stop = vehicle_journey.stop_times.len();
                let vehicle_journey_loads = loads_data
                    .per_vehicle_journey
                    .entry(vehicle_journey_idx)
                    .or_insert_with(|| VehicleJourneyLoads::new(stop_sequence_iter.clone()));
                let service_id = &vehicle_journey.service_id;
                let calendar = if let Some(calendar) = model.calendars.get(service_id) {
                    calendar
                } else {
                    continue;
                };
                for date in &calendar.dates {
                    for stop_sequence in stop_sequence_iter.clone() {
                        let idx = vehicle_journey_loads
                            .stop_sequence_to_idx
                            .get(&stop_sequence)
                            // unwrap is safe since we created the `vehicle_journey_loads` with the same stop_sequence_iter
                            .unwrap();
                        let trip_load = vehicle_journey_loads
                            .per_date
                            .entry(*date)
                            .or_insert_with(|| TripLoads::new(nb_of_stop));
                        trip_load.per_stop[*idx] = load;
                    }
                }
            }
        }
        Ok(loads_data)
    }

    pub fn new<R: io::Read>(
        csv_occupancys_reader: R,
        model: &base_model::Model,
    ) -> Result<Self, Box<dyn Error>> {
        info!("loading vehicle loads data");
        let mut loads_data = LoadsData {
            per_vehicle_journey: BTreeMap::new(),
        };
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b',')
            .from_reader(csv_occupancys_reader);

        let mut record = csv::StringRecord::new();

        while reader.read_record(&mut record)? {
            let is_valid_record = parse_record(&record, model);
            let (vehicle_journey_idx, stop_sequence, occupancy, date) = match is_valid_record {
                Ok((vehicle_journey_idx, stop_sequence, occupancy, date)) => {
                    (vehicle_journey_idx, stop_sequence, occupancy, date)
                }
                Err(parse_error) => {
                    trace!(
                        "Error reading at line {}: {} \n. I'll skip this line. ",
                        reader.position().line(),
                        parse_error
                    );
                    continue;
                }
            };
            let load = occupancy_to_load(occupancy);

            let vehicle_journey = &model.vehicle_journeys[vehicle_journey_idx];
            let stop_sequence_iter = vehicle_journey
                .stop_times
                .iter()
                .map(|stop_time| stop_time.sequence);
            let nb_of_stop = vehicle_journey.stop_times.len();

            let vehicle_journey_loads = loads_data
                .per_vehicle_journey
                .entry(vehicle_journey_idx)
                .or_insert_with(|| VehicleJourneyLoads::new(stop_sequence_iter));
            let idx = {
                let has_idx = vehicle_journey_loads
                    .stop_sequence_to_idx
                    .get(&stop_sequence);
                if has_idx.is_none() {
                    trace!(
                        "Error while reading at line {}. \n
                        The provided stop_sequence {} is not valid for the vehicle_journey {}.
                        I'll skip this line.",
                        reader.position().line(),
                        stop_sequence,
                        &record[0]
                    );
                    continue;
                }
                has_idx.unwrap()
            };

            let trip_load = vehicle_journey_loads
                .per_date
                .entry(date)
                .or_insert_with(|| TripLoads::new(nb_of_stop));
            trip_load.per_stop[*idx] = load;
            trace!(
                "load inserted for vehicle journey '{}' on stop sequence '{}': load={}",
                vehicle_journey.id,
                stop_sequence,
                load
            );
        }

        // loads_data._check(model);

        info!("vehicle loads data loaded");
        Ok(loads_data)
    }

    fn _check(&self, model: &BaseModel) {
        // for each vehicle_journey, check that :
        //  - for each valid date, we have occupancy data for every stop_time
        for vehicle_journey_idx in model.vehicle_journeys() {
            let has_vehicle_journey_load = self.per_vehicle_journey.get(&vehicle_journey_idx);
            if has_vehicle_journey_load.is_none() {
                debug!(
                    "No occupancy data provided for vehicle_journey {}",
                    model.vehicle_journey_name(vehicle_journey_idx)
                );
                continue;
            }
            let has_dates = model.vehicle_journey_dates(vehicle_journey_idx);
            let dates = match has_dates {
                Some(dates) => dates,
                None => {
                    continue;
                }
            };
            let vehicle_journey_load = has_vehicle_journey_load.unwrap();
            for date in dates {
                let has_trip_load = vehicle_journey_load.per_date.get(&date);
                if has_trip_load.is_none() {
                    trace!(
                        "No occupancy data provided for vehicle_journey {} on date {}",
                        model.vehicle_journey_name(vehicle_journey_idx),
                        date
                    );
                    continue;
                }
            }
        }
    }
}

fn parse_record(
    record: &csv::StringRecord,
    model: &base_model::Model,
) -> Result<(BaseVehicleJourneyIdx, StopSequence, Occupancy, NaiveDate), Box<dyn Error>> {
    if record.len() != 4 {
        let msg = format!("Expected 4 fields, but got {}", record.len());
        return Err(From::from(msg));
    }

    let vehicle_journey_idx = {
        let trip_id = &record[0];
        model
            .vehicle_journeys
            .get_idx(trip_id)
            .ok_or_else(|| format!("Cannot find a trip named {} in the ntfs data.", trip_id,))?
    };

    let stop_sequence = {
        let string = &record[1];
        string.parse::<StopSequence>().map_err(|parse_error| {
            format!(
                "Cannot parse the second field (stop_sequence) {} as usize.
                    Parse error {:?}.",
                string, parse_error
            )
        })?
    };

    let occupancy = {
        let occupancy_string = &record[2];
        let occupancy_float = occupancy_string.parse::<f64>().map_err(|parse_error| {
            format!(
                "Cannot parse the third field (occupancy) {} as a float.
                        Parse error {:?}.",
                occupancy_string, parse_error
            )
        })?;
        if occupancy_float.is_infinite()
            || occupancy_float.is_nan()
            || occupancy_float < 0.0f64
            || occupancy_float > 1.0f64
        {
            let msg = format!(
                "The third field {} is not a valid value for occupancy.
                                It should be a float between 0.0 and 1.0.",
                occupancy_string,
            );
            return Err(From::from(msg));
        }
        // the cast is safe because we check above that occupancy_float is between 0.0 and 1.0
        // thus (occupancy_float * 100).trunc is between 0 and 100
        // and thus will fit into an u8
        (occupancy_float * 100.0).trunc() as u8
    };

    let date = {
        let date_string = &record[3];
        NaiveDate::parse_from_str(date_string, "%Y-%m-%d").map_err(|_| {
            format!(
                "The fourth field {} is not a valid date.
                    It should be formatted like 2020-04-17.",
                date_string
            )
        })?
    };

    Ok((vehicle_journey_idx, stop_sequence, occupancy, date))
}
