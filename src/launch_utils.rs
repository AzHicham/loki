use std::{collections::BTreeMap, path::Path, time::SystemTime};
use crate::traits;
use crate::LoadsData;
use crate::PositiveDuration;
use crate::config;
use log::{info, warn};
use transit_model::Model;

pub fn read<Data, InputPath: AsRef<Path>, LoadsPath: AsRef<Path>>(
    ntfs_path: InputPath,
    input_type : & config::InputType,
    loads_data_path: Option<LoadsPath>,
    default_transfer_duration: &PositiveDuration,
) -> Result<(Data, Model), transit_model::Error>
where
    Data: traits::Data,
{
    let model = match input_type {
        config::InputType::Ntfs => transit_model::ntfs::read(ntfs_path)?,
        config::InputType::Gtfs => {
            let configuration = transit_model::gtfs::Configuration {
                contributor : transit_model::objects::Contributor::default(),
                dataset : transit_model::objects::Dataset::default(),
                feed_infos : BTreeMap::new(),
                prefix_conf: None,
                on_demand_transport: false,
                on_demand_transport_comment: None,
            };
        
            transit_model::gtfs::read_from_path(ntfs_path, configuration)?
        }
    };
    
    
    info!("Transit model loaded");
    info!(
        "Number of vehicle journeys : {}",
        model.vehicle_journeys.len()
    );
    info!("Number of routes : {}", model.routes.len());

    let loads_data = loads_data_path.map(|path| {
        LoadsData::new(&path, &model).unwrap_or_else(|err| {
            warn!(
                "Error while reading the passenger loads file at {:?} : {:?}",
                &path.as_ref(),
                err.source()
            );
            warn!("I'll use default loads.");
            LoadsData::empty()
        })
    })
    .unwrap_or_else( || LoadsData::empty());

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
