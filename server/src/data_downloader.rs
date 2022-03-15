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

use anyhow::{bail, Context, Error};
use s3::{creds::Credentials, Bucket, Region};
use std::io::Cursor;

use crate::server_config::BucketParams;
pub struct DataDownloader {
    bucket: Bucket,

    // S3 key of ntfs/gtfs file
    fusio_data_key: String,

    // latest version_id of ntfs/gtfs file
    fusio_data_version_id: String,
}

pub enum DownloadStatus {
    Ok(Cursor<Vec<u8>>), // In-Memory File Handler
    AlreadyPresent,
}

impl DataDownloader {
    pub fn new(config: &BucketParams) -> Result<DataDownloader, Error> {
        let credentials = Credentials::new(
            Some(&config.bucket_access_key),
            Some(&config.bucket_secret_key),
            None,
            None,
            None,
        )?;

        let bucket = match config.bucket_region.parse() {
            // Custom Region / Minio
            Ok(Region::Custom { .. }) => {
                let region = Region::Custom {
                    region: "".to_string(),
                    endpoint: config.bucket_region.clone(),
                };
                Bucket::new_with_path_style(&config.bucket_name, region, credentials)?
            }
            // AWS Region
            Ok(region) => Bucket::new(&config.bucket_name, region, credentials)?,
            Err(err) => {
                bail!("{err}")
            }
        };

        Ok(Self {
            bucket,
            fusio_data_key: config.data_path_key.clone(),
            fusio_data_version_id: "".to_string(),
        })
    }

    async fn get_file_version_id(&self, file_key: &str) -> Result<String, Error> {
        let (meta, status_code) = self.bucket.head_object(&file_key).await?;
        let version_id = if status_code == 200 {
            if let Some(version_id) = meta.last_modified {
                version_id
            } else {
                bail!(
                    "Error while fetching file metadata, version_id contains no value,\
                    file_key: {}, bucket: {}",
                    file_key,
                    self.bucket.name
                )
            }
        } else {
            bail!(
                "Error while fetching file metadata, status code : {}, \
            file_key: {}, bucket: {}",
                status_code,
                file_key,
                self.bucket.name
            )
        };
        Ok(version_id)
    }

    async fn download_file(&self, file_key: &str) -> Result<Vec<u8>, Error> {
        // let mut data_file_handler = tokio::fs::File::create(&destination_path)
        //     .await
        //     .context(format!("Cannot create file {:?}", destination_path))?;
        let (data, status_code) = self.bucket.get_object(file_key).await.context(format!(
            "Cannot download file {} from bucket {}",
            file_key, self.bucket.name
        ))?;
        if status_code == 200 {
            Ok(data)
        } else {
            bail!(
                "Error while downloading file {} from bucket {}, status code : {}",
                file_key,
                self.bucket.name,
                status_code
            )
        }
    }

    pub async fn download_fusio_data(&mut self) -> Result<DownloadStatus, Error> {
        // get meta info about file we are going to download
        let version_id = self.get_file_version_id(&self.fusio_data_key).await?;
        if self.fusio_data_version_id != version_id {
            let data = self.download_file(&self.fusio_data_key).await?;
            let cursor = std::io::Cursor::new(data);
            self.fusio_data_version_id = version_id;
            Ok(DownloadStatus::Ok(cursor))
        } else {
            Ok(DownloadStatus::AlreadyPresent)
        }
    }
}
