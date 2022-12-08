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

use super::config;
use crate::{config::launch_params::LocalFileParams, loki::TransitData, timer};
use anyhow::{format_err, Error};
use loki::{
    models::base_model::{self, BaseModel},
    tracing::{info, warn},
    transit_model, DataTrait, LoadsData, PositiveDuration,
};
use std::{str::FromStr, time::SystemTime};

pub fn read(launch_params: &config::LaunchParams) -> Result<(TransitData, BaseModel), Error> {
    let base_model = read_model(
        &LocalFileParams {
            input_data_path: launch_params.input_data_path.clone(),
            loads_data_path: launch_params.loads_data_path.clone(),
        },
        launch_params.input_data_type.clone(),
        launch_params.default_transfer_duration,
    )?;

    let data = build_transit_data(&base_model);

    Ok((data, base_model))
}

pub fn read_model_from_zip_reader<R>(
    input_data_reader: R,
    loads_data_reader: Option<R>,
    source: &str,
    input_data_type: config::InputDataType,
    default_transfer_duration: PositiveDuration,
) -> Result<BaseModel, Error>
where
    R: std::io::Seek + std::io::Read,
{
    let read_model_start_time = SystemTime::now();
    let model = match input_data_type {
        config::InputDataType::Ntfs => {
            transit_model::ntfs::from_zip_reader(input_data_reader, source)?
        }
        config::InputDataType::Gtfs => {
            let configuration = transit_model::gtfs::Configuration::default();
            let max_distance = f64::from_str(transit_model::TRANSFER_MAX_DISTANCE)?;
            let walking_speed = f64::from_str(transit_model::TRANSFER_WALKING_SPEED)?;
            let waiting_time = u32::from_str(transit_model::TRANSFER_WAITING_TIME)?;

            let model = transit_model::gtfs::Reader::new(configuration)
                .parse_zip_reader(input_data_reader, source)?;

            transit_model::transfers::generates_transfers(
                model,
                max_distance,
                walking_speed,
                waiting_time,
                None,
            )?
        }
    };
    info!(
        "Transit model loaded in {} ms",
        timer::duration_since(read_model_start_time)
    );

    let loads_data = read_loads_data_from_reader(loads_data_reader, &model);

    BaseModel::new(model, loads_data, default_transfer_duration)
        .map_err(|err| format_err!("Could not create base model {:?}", err))
}

pub fn read_model(
    data_files: &LocalFileParams,
    input_data_type: config::InputDataType,
    default_transfer_duration: PositiveDuration,
) -> Result<BaseModel, Error> {
    let read_model_start_time = SystemTime::now();
    let model = match input_data_type {
        config::InputDataType::Ntfs => transit_model::ntfs::read(&data_files.input_data_path)?,
        config::InputDataType::Gtfs => {
            let configuration = transit_model::gtfs::Configuration::default();
            let max_distance = f64::from_str(transit_model::TRANSFER_MAX_DISTANCE)?;
            let walking_speed = f64::from_str(transit_model::TRANSFER_WALKING_SPEED)?;
            let waiting_time = u32::from_str(transit_model::TRANSFER_WAITING_TIME)?;

            let model = transit_model::gtfs::Reader::new(configuration)
                .parse(&data_files.input_data_path)?;

            transit_model::transfers::generates_transfers(
                model,
                max_distance,
                walking_speed,
                waiting_time,
                None,
            )?
        }
    };
    info!(
        "Transit model loaded in {} ms",
        timer::duration_since(read_model_start_time)
    );

    let loads_data_reader =
        data_files.loads_data_path.as_ref().and_then(
            |csv_occupancy_path| match std::fs::File::open(csv_occupancy_path) {
                Ok(reader) => Some(reader),
                Err(err) => {
                    warn!("Could not open load_data_path {csv_occupancy_path:?} : {err:?}");
                    None
                }
            },
        );

    let loads_data = read_loads_data_from_reader(loads_data_reader, &model);

    BaseModel::new(model, loads_data, default_transfer_duration)
        .map_err(|err| format_err!("Could not create base model {:?}", err))
}

fn read_loads_data_from_reader<R: std::io::Read>(
    _reader: Option<R>,
    _model: &base_model::Model,
) -> LoadsData {
    #[cfg(not(feature = "demo_occupancy"))]
    {
        _reader
            .map(|csv_occupancy_reader| {
                LoadsData::try_from_reader(csv_occupancy_reader, &_model).unwrap_or_else(|e| {
                    warn!("failed to load passenger occupancy data: {e}");
                    warn!("initialized with empty passenger occupancy.");
                    LoadsData::empty()
                })
            })
            .unwrap_or_else(|| {
                info!("No load data given. I'll use empty loads.");
                LoadsData::empty()
            })
    }
    #[cfg(feature = "demo_occupancy")]
    {
        LoadsData::fake_occupancy_metro1_rera(&model).unwrap_or_else(|err| {
            warn!("Failed to create fake occupancy data {err}. I'll use empty loads.");
            LoadsData::empty()
        })
    }
}

pub fn build_transit_data(base_model: &BaseModel) -> TransitData {
    info!(
        "Number of vehicle journeys : {}",
        base_model.nb_of_vehicle_journeys()
    );

    let start_build_time = SystemTime::now();
    let data = TransitData::new(base_model);
    let data_build_duration = timer::duration_since(start_build_time);
    info!("Data constructed in {} ms", data_build_duration);
    info!("Number of missions {} ", data.nb_of_missions());
    info!("Number of trips {} ", data.nb_of_trips());
    info!(
        "Validity dates between {} and {}",
        data.calendar().first_date(),
        data.calendar().last_date()
    );

    data
}
