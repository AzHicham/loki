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

use anyhow::{bail, Context, Error};
use core::time::Duration;
use s3::{creds::Credentials, Bucket, Region};
use std::io::Cursor;

use crate::server_config::BucketParams;
pub struct DataDownloader {
    bucket: Bucket,

    // S3 key of ntfs/gtfs file
    data_key: String,
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

        let mut bucket = match config.bucket_region.parse() {
            // Custom Region / Minio
            Ok(Region::Custom { .. }) => {
                let region = Region::Custom {
                    region: "".to_string(),
                    endpoint: config.bucket_region.clone(),
                };
                Bucket::new(&config.bucket_name, region, credentials)?.with_path_style()
            }
            // AWS Region
            Ok(region) => Bucket::new(&config.bucket_name, region, credentials)?,
            Err(err) => {
                bail!(
                    "Error while creating Bucket {} with region {} and name {}",
                    err,
                    config.bucket_region,
                    config.bucket_region
                )
            }
        };

        let timeout = Duration::from_millis(u64::from(config.bucket_timeout_in_ms));
        bucket.set_request_timeout(Some(timeout));

        Ok(Self {
            bucket,
            data_key: config.data_path_key.clone(),
        })
    }

    async fn _get_file_version_id(&self, file_key: &str) -> Result<String, Error> {
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

    pub async fn download_data(&self) -> Result<Vec<u8>, Error> {
        let response = self
            .bucket
            .get_object(&self.data_key)
            .await
            .context(format!(
                "Cannot download file {} from bucket {}",
                self.data_key, self.bucket.name
            ))?;
        let status_code = response.status_code();
        if status_code == 200 {
            Ok(response.bytes().to_owned())
        } else {
            bail!(
                "Error while downloading file {} from bucket {}, status code : {}",
                self.data_key,
                self.bucket.name,
                status_code
            )
        }
    }
}
