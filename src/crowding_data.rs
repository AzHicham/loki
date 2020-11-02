use chrono::NaiveDate;
use transit_model::objects::{VehicleJourney};
use transit_model::Model;
use typed_index_collection::{Idx};
use std::collections::BTreeMap;
use std::path::Path;
use std::error::Error;
use log::{warn};

type StopSequence = usize;
type Load = u8;

type VehicleJourneyIdx = Idx<VehicleJourney>;

type TripDay = (VehicleJourneyIdx, NaiveDate);

type VehicleCrowding = BTreeMap<StopSequence, Load>;

type CrowdingData = BTreeMap<TripDay, VehicleCrowding>;



pub fn read<P: AsRef<Path>>(csv_loads_filepath : P, model : & Model) -> Result<CrowdingData, Box<dyn Error>> {
    let mut result = CrowdingData::new();
    let filepath = csv_loads_filepath.as_ref();
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_path(filepath)?;

    let mut record = csv::StringRecord::new();

    while reader.read_record(& mut record)? {

        let is_valid_record = parse_record(&record, model);
        let (vehicle_journey, stop_sequence, load, date) = match is_valid_record {
            Ok((vehicle_journey, stop_sequence, load, date)) => (vehicle_journey, stop_sequence, load, date),
            Err(parse_error) => {
                warn!("Error reading {:?} at line {} : {} \n. I'll skip this line. ",
                        filepath,
                        reader.position().line(),
                        parse_error           
                    );
                continue; 
            }
        };

        let vehicle_crowding = result.entry((vehicle_journey, date)).or_insert(VehicleCrowding::new());
        if vehicle_crowding.contains_key(&stop_sequence) {
            warn!("Error reading {:?}. There is two load values for trip {} at date {}. I'll ignore the second value.",
                filepath,
                &record[0],
                date
            );
            continue;
        }
        vehicle_crowding.insert(stop_sequence, load);

    }

    Ok(result)

}


fn parse_record(record : &csv::StringRecord, model : & Model) -> Result<(VehicleJourneyIdx, StopSequence, Load, NaiveDate), Box<dyn Error>> {
    if record.len() != 4 {
        let msg = format!("Expected 4 fields, but got {}",
                    record.len()
                );
        return Err(From::from(msg));
        
    }

    let vehicle_journey = {
        let trip_id =  &record[0];
        model.vehicle_journeys.get_idx(trip_id)
        .ok_or_else(||
            format!("Cannot find a trip named {} in the ntfs data.", 
                        trip_id,
                    )
            )?
    };

    let stop_sequence = {
        let string = &record[1];
        string.parse::<usize>()
        .map_err(|parse_error|
            format!("Cannot parse the second field (stop_sequence) {} as usize.
                    Parse error {:?}.",
                    string,
                    parse_error
            )
        )?
    };

    let load = {
        let load_string = &record[2];
        let load_float = 
            load_string.parse::<f64>()
            .map_err(|parse_error|
                format!("Cannot parse the third field (load) {} as a float.
                        Parse error {:?}.",
                        load_string,
                        parse_error
                )
            )?;
        if load_float.is_infinite() || load_float.is_nan() || load_float < 0.0f64 || load_float > 1.0f64 {
            let msg = format!("The third field {} is not a valid value for load.
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
        NaiveDate::parse_from_str(date_string, "%Y-%m-%d")
        .map_err(|_|
            format!("The fourth field {} is not a valid date.
                    It should be formatted like 2020-04-17.",
                    date_string
                )
        )?
    };

    Ok((vehicle_journey, stop_sequence, load, date))

}
