use chrono::NaiveDate;
use log::{debug, trace};
use std::collections::{BTreeMap};
use std::error::Error;
use std::path::Path;
use transit_model::objects::VehicleJourney;
use transit_model::Model;
use typed_index_collection::Idx;

type StopSequence = u32;
type Load = u8;

type VehicleJourneyIdx = Idx<VehicleJourney>;



pub struct CrowdData {
    per_vehicle_journey : BTreeMap<VehicleJourneyIdx, VehicleJourneyCrowd>
}

struct VehicleJourneyCrowd {
    stop_sequence_to_idx : BTreeMap<StopSequence, usize>,
    per_date : BTreeMap<NaiveDate, TripCrowd>
}

struct TripCrowd {
    per_stop : Vec<Option<Load>>
}

impl VehicleJourneyCrowd {
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


    // Returns `Ok()` if the load has been set.
    // Returns `Err()` if `stop_sequence` is not valid for this vehicle_journey
    fn set_load(& mut self, date : & NaiveDate, stop_sequence : StopSequence, load : Load) -> Result<(), ()>
    {
        let idx = self.stop_sequence_to_idx.get(&stop_sequence).ok_or(())?;
        let nb_of_stop = self.stop_sequence_to_idx.len();
        let trip_crowd = self.per_date.entry(*date).or_insert_with(||TripCrowd::new(nb_of_stop));
        trip_crowd.per_stop[*idx] = Some(load);
        Ok(())
    }

    // Returns `Err()` if `stop_sequence` is not valid for this vehicle_journey
    fn get_load(&self, date : & NaiveDate, stop_sequence : StopSequence) -> Result<Option<Load>, ()> {
        let idx = self.stop_sequence_to_idx.get(&stop_sequence).ok_or(())?;
        if let Some(trip_crowd) = self.per_date.get(date) {
            Ok(trip_crowd.per_stop[*idx].clone())
        }
        else {
            Ok(None)
        }
        
    }
}

impl TripCrowd {
    fn new(nb_of_stop : usize) -> Self {
        Self {
            per_stop : vec![None; nb_of_stop]
        }
    }
}

impl CrowdData {

    pub fn new<P: AsRef<Path>>(
        csv_loads_filepath: P,
        model: &Model,
    ) -> Result<Self, Box<dyn Error>> {
        let mut crowd_data = CrowdData{
            per_vehicle_journey : BTreeMap::new()
        };
        let filepath = csv_loads_filepath.as_ref();
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b';')
            .from_path(filepath)?;

        let mut record = csv::StringRecord::new();

        while reader.read_record(&mut record)? {
            let is_valid_record = parse_record(&record, model);
            let (vehicle_journey_idx, stop_sequence, load, date) = match is_valid_record {
                Ok((vehicle_journey_idx, stop_sequence, load, date)) => {
                    (vehicle_journey_idx, stop_sequence, load, date)
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

            let stop_sequence_iter =  model.vehicle_journeys[vehicle_journey_idx]
                .stop_times.iter().map(|stop_time| {
                    stop_time.sequence
                });


            let vehicle_journey_crowd = crowd_data
                .per_vehicle_journey
                .entry(vehicle_journey_idx)
                .or_insert_with(||VehicleJourneyCrowd::new(stop_sequence_iter));
            let load_result = vehicle_journey_crowd.get_load(&date, stop_sequence);
            match load_result {
                Err(_) => {
                    trace!("Error reading {:?} at line {}. \n 
                        The provided stop_sequence {} is not valid for the vehicle_journey {}.
                        I'll skip this line.",
                        filepath,
                        reader.position().line(),
                        stop_sequence,
                        &record[0]
                    );
                    trace!("Valid stop_sequence : {:#?}", vehicle_journey_crowd.stop_sequence_to_idx);
                    continue;
                }
                Ok(Some(_)) => {
                    trace!("Error reading {:?}. There is two load values for trip {} at date {}. I'll ignore the second value.",
                        filepath,
                        &record[0],
                        date
                    );
                    continue;
                },
                Ok(None) => {}

            }
            let _ = vehicle_journey_crowd.set_load(&date, stop_sequence, load);
        }

        crowd_data.check(model);

        Ok(crowd_data)
    }

    fn check(&self, model : & Model)  {
        // for each vehicle_journey, check that :
        //  - for each valid date, we have load data for every stop_time
        for (vehicle_journey_idx, vehicle_journey) in model.vehicle_journeys.iter() {
            let has_calendar = model.calendars.get(&vehicle_journey.service_id);
            if has_calendar.is_none() {
                continue;
            }
            let calendar = has_calendar.unwrap();

            let has_vehicle_journey_crowd = self.per_vehicle_journey.get(&vehicle_journey_idx);
            if has_vehicle_journey_crowd.is_none() {
                debug!(
                    "No crowding data provided for vehicle_journey {}",
                    vehicle_journey.id
                );
                continue;
            }
            let vehicle_journey_crowd = has_vehicle_journey_crowd.unwrap();
            for date in calendar.dates.iter() {
                for stop_time in vehicle_journey.stop_times.iter() {
                    let stop_sequence = stop_time.sequence;
                    // unwrap is safe here because we initialize `vehicle_journey_crowd`
                    // with vehicle_journey.stop_times.stop_sequence (used also here to iterate stop_sequence)
                    let load_result = vehicle_journey_crowd.get_load(date, stop_sequence).unwrap(); 
                    if load_result.is_none() {
                        debug!(
                            "No crowding data provided for vehicle_journey {} on date {} at stop sequence {}",
                            vehicle_journey.id, date, stop_sequence
                        );
                    }
                }
            }            
        }
    }

}

fn parse_record(
    record: &csv::StringRecord,
    model: &Model,
) -> Result<(VehicleJourneyIdx, StopSequence, Load, NaiveDate), Box<dyn Error>> {
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

    let load = {
        let load_string = &record[2];
        let load_float = load_string.parse::<f64>().map_err(|parse_error| {
            format!(
                "Cannot parse the third field (load) {} as a float.
                        Parse error {:?}.",
                load_string, parse_error
            )
        })?;
        if load_float.is_infinite()
            || load_float.is_nan()
            || load_float < 0.0f64
            || load_float > 1.0f64
        {
            let msg = format!(
                "The third field {} is not a valid value for load.
                                It should be a float between 0.0 and 1.0.",
                load_string,
            );
            return Err(From::from(msg));
        }
        // the cast is safe because we check above that load_float is between 0.0 and 1.0
        // thus (load_float * 100).trunc is between 0 and 100
        // and thus will fit into an u8
        (load_float * 100.0).trunc() as u8
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

    Ok((vehicle_journey_idx, stop_sequence, load, date))
}

#[cfg(test)]
mod tests {
    use super::CrowdData;

    #[test]
    fn exploration() {
        let input_dir = "/home/pascal/data/charge/ntfs/";
        let model = transit_model::ntfs::read(input_dir).unwrap();
        let crowd_data_filepath = "/home/pascal/data/charge/stoptimes_load.csv";
        let _crowd_data = CrowdData::new(crowd_data_filepath, &model);
    }
}
