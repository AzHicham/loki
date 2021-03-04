use std::{path::Path, time::SystemTime};

use crate::traits;
use crate::LoadsData;
use crate::PositiveDuration;
use log::{info, warn};
use transit_model::Model;

pub fn read_ntfs<Data, NtfsPath: AsRef<Path>, LoadsPath: AsRef<Path>>(
    ntfs_path: NtfsPath,
    loads_data_path: LoadsPath,
    default_transfer_duration: &PositiveDuration,
) -> Result<(Data, Model), transit_model::Error>
where
    Data: traits::Data,
{
    let model = transit_model::ntfs::read(ntfs_path)?;
    info!("Transit model loaded");
    info!(
        "Number of vehicle journeys : {}",
        model.vehicle_journeys.len()
    );
    info!("Number of routes : {}", model.routes.len());

    let loads_data = LoadsData::new(&loads_data_path, &model).unwrap_or_else(|err| {
        warn!(
            "Error while reading the passenger loads file at {:?} : {:?}",
            &loads_data_path.as_ref(),
            err.source()
        );
        warn!("I'll use default loads.");
        LoadsData::empty()
    });

    let data_timer = SystemTime::now();
    let data = Data::new(&model, &loads_data, *default_transfer_duration);
    let data_build_duration = data_timer.elapsed().unwrap().as_millis();
    info!("Data constructed in {} ms", data_build_duration);
    info!("Number of missions {} ", data.nb_of_missions());
    info!("Number of trips {} ", data.nb_of_trips());
    info!(
        "Validity dates between {} and {}",
        data.calendar().first_date(),
        data.calendar().last_date()
    );

    Ok((data, model))
}
