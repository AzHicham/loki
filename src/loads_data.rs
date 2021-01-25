use chrono::NaiveDate;
use log::{debug, trace};
use std::collections::{BTreeMap};
use std::error::Error;
use std::path::Path;
use transit_model::objects::VehicleJourney;
use transit_model::Model;
use typed_index_collection::Idx;

type StopSequence = u32;
type Occupancy = u8;
type VehicleJourneyIdx = Idx<VehicleJourney>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Load {
    Low,
    Medium, 
    High
}

use std::cmp::Ordering;

fn load_to_int(load : & Load) -> u8 {
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

fn occupancy_to_load(occupancy : Occupancy) -> Load {
    debug_assert!(occupancy <= 100);
    if occupancy <= 30 {
        Load::Low
    }
    else if occupancy <= 70 {
        Load::Medium
    }
    else {
        Load::High
    }
}


pub struct LoadsData {
    per_vehicle_journey : BTreeMap<VehicleJourneyIdx, VehicleJourneyLoads>
}

struct VehicleJourneyLoads {
    stop_sequence_to_idx : BTreeMap<StopSequence, usize>,
    per_date : BTreeMap<NaiveDate, TripLoads>
}

struct TripLoads {
    per_stop : Vec<Load>
}

impl VehicleJourneyLoads {
    fn new<StopSequenceIter>(stop_sequence_iter : StopSequenceIter) -> Self 
    where StopSequenceIter : Iterator<Item = StopSequence>
    {
        let mut stop_sequence_to_idx = BTreeMap::new();
        for (idx, stop_sequence) in stop_sequence_iter.enumerate() {
            stop_sequence_to_idx.insert(stop_sequence, idx);
        }
        Self {
            stop_sequence_to_idx,
            per_date : BTreeMap::new()
        }
    }

}

impl TripLoads {
    fn new(nb_of_stop : usize) -> Self {
        Self {
            per_stop : vec![Load::Medium; nb_of_stop]
        }
    }
}

impl LoadsData {

    pub fn loads(&self, vehicle_journey_idx : & VehicleJourneyIdx,  date : & NaiveDate) -> Option<&[Load]> {
        let vehicle_journey_load = self.per_vehicle_journey.get(vehicle_journey_idx)?;
        let trip_load = vehicle_journey_load.per_date.get(date)?;
        Some(trip_load.per_stop.as_slice())
    }

    // pub fn default_loads(&self) -> std::iter::Repeat<Load> {
    //     std::iter::repeat(Load::Medium)
    // }

    pub fn new<P: AsRef<Path>>(
        csv_occupancys_filepath: P,
        model: &Model,
    ) -> Result<Self, Box<dyn Error>> {

        let mut loads_data = LoadsData{
            per_vehicle_journey : BTreeMap::new()
        };
        let filepath = csv_occupancys_filepath.as_ref();
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b';')
            .from_path(filepath)?;

        let mut record = csv::StringRecord::new();

        while reader.read_record(&mut record)? {
            let is_valid_record = parse_record(&record, model);
            let (vehicle_journey_idx, stop_sequence, occupancy, date) = match is_valid_record {
                Ok((vehicle_journey_idx, stop_sequence, occupancy, date)) => {
                    (vehicle_journey_idx, stop_sequence, occupancy, date)
                }
                Err(parse_error) => {
                    debug!(
                        "Error reading {:?} at line {} : {} \n. I'll skip this line. ",
                        filepath,
                        reader.position().line(),
                        parse_error
                    );
                    continue;
                }
            };
            let load = occupancy_to_load(occupancy);

            let vehicle_journey = &model.vehicle_journeys[vehicle_journey_idx];
            let stop_sequence_iter =  vehicle_journey
                .stop_times.iter().map(|stop_time| {
                    stop_time.sequence
                });
            let nb_of_stop = vehicle_journey.stop_times.len();


            let vehicle_journey_loads = loads_data
                .per_vehicle_journey
                .entry(vehicle_journey_idx)
                .or_insert_with(||VehicleJourneyLoads::new(stop_sequence_iter));
            let idx = {
                let has_idx = vehicle_journey_loads.stop_sequence_to_idx.get(&stop_sequence);
                if has_idx.is_none() {
                    trace!("Error reading {:?} at line {}. \n 
                        The provided stop_sequence {} is not valid for the vehicle_journey {}.
                        I'll skip this line.",
                        filepath,
                        reader.position().line(),
                        stop_sequence,
                        &record[0]
                    );
                    continue;
                }
                has_idx.unwrap()
            };

            let trip_load = vehicle_journey_loads.per_date.entry(date).or_insert_with(||TripLoads::new(nb_of_stop));
            if trip_load.per_stop[*idx] != load {
                trace!("Error reading {:?}. There is more than one occupancy values for trip {} at date {}. I'll keep the first value.",
                    filepath,
                    &record[0],
                    date
                );
                continue;
            }
            trip_load.per_stop[*idx] = load;
        }

        loads_data.check(model);

        Ok(loads_data)
    }

    fn check(&self, model : & Model)  {
        // for each vehicle_journey, check that :
        //  - for each valid date, we have occupancy data for every stop_time
        for (vehicle_journey_idx, vehicle_journey) in model.vehicle_journeys.iter() {
            let has_calendar = model.calendars.get(&vehicle_journey.service_id);
            if has_calendar.is_none() {
                continue;
            }
            let calendar = has_calendar.unwrap();

            let has_vehicle_journey_load = self.per_vehicle_journey.get(&vehicle_journey_idx);
            if has_vehicle_journey_load.is_none() {
                debug!(
                    "No occupancy data provided for vehicle_journey {}",
                    vehicle_journey.id
                );
                continue;
            }
            let vehicle_journey_load = has_vehicle_journey_load.unwrap();
            for date in calendar.dates.iter() {
                let has_trip_load = vehicle_journey_load.per_date.get(date);
                if has_trip_load.is_none() {
                    trace!(
                        "No occupancy data provided for vehicle_journey {} on date {}",
                        vehicle_journey.id,
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
    model: &Model,
) -> Result<(VehicleJourneyIdx, StopSequence, Occupancy, NaiveDate), Box<dyn Error>> {
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



#[cfg(test)]
mod tests {
    use super::LoadsData;

    #[test]
    fn exploration() {
        let input_dir = "/home/pascal/data/charge/ntfs/";
        let model = transit_model::ntfs::read(input_dir).unwrap();
        let occupancy_data_filepath = "/home/pascal/data/charge/stoptimes_load.csv";
        let _ = LoadsData::new(occupancy_data_filepath, &model);
    }
}
