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

use anyhow::{Context, Error};

use loki_launch::{
    config::{launch_params::LocalFileParams, parse_env_var, read_env_var},
    loki::PositiveDuration,
};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, str::FromStr};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DataSourceParams {
    Local(LocalFileParams),
    S3(BucketParams),
}

impl DataSourceParams {
    pub fn new_from_env_vars() -> Result<Self, Error> {
        let data_source_type = std::env::var("LOKI_DATA_SOURCE_TYPE")
            .context("Could not read mandatory env var LOKI_DATA_SOURCE_TYPE")?;

        match data_source_type.trim() {
            "s3" => {
                let bucket_params = BucketParams::new_from_env_vars()
                    .context("LOKI_DATA_SOURCE_TYPE is set to 's3' but I could not read bucket params from env vars")?;
                Ok(DataSourceParams::S3(bucket_params))
            }
            "local" => {
                let local_file_params = LocalFileParams::new_from_env_vars()
                    .context("LOKI_DATA_SOURCE_TYPE is set to 'local' but I could not read local file params from env vars")?;
                Ok(DataSourceParams::Local(local_file_params))
            }
            _ => {
                anyhow::bail!(
                    "Bad LOKI_DATA_SOURCE_TYPE : '{}'. Allowed values are 's3' or 'local'",
                    data_source_type
                );
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct BucketParams {
    /// for example 's3-eu-west-1.amazonaws.com'
    pub bucket_url: String,
    pub bucket_region: String,
    /// name of the bucket which should exists at bucket_url
    pub bucket_name: String,

    /// Change the host used to make requests to the bucket.
    /// When `true`, use : http://bucket_url/bucket_name/
    ///
    /// When `false`, use Virtual-hosted–style : http://bucket.name.bucket_url/
    ///
    /// Defaults to `false`
    ///
    /// see https://docs.aws.amazon.com/AmazonS3/latest/userguide/RESTAPI.html
    ///
    /// Path-style are being deprecated on aws https://aws.amazon.com/blogs/aws/amazon-s3-path-deprecation-plan-the-rest-of-the-story/
    /// But are still useful for other storage providers like Minio
    #[serde(default = "default_path_style")]
    pub path_style: bool,

    /// where to get the file in the bucket
    pub data_path_key: String,

    #[serde(default = "default_bucket_credentials")]
    pub bucket_credentials: BucketCredentials,

    #[serde(default = "default_bucket_timeout")]
    pub bucket_timeout: PositiveDuration,
}

pub fn default_bucket_credentials() -> BucketCredentials {
    BucketCredentials::AwsHttpCredentials
}

pub fn default_bucket_timeout() -> PositiveDuration {
    PositiveDuration::from_hms(0, 0, 30)
}

pub fn default_path_style() -> bool {
    false
}

impl BucketParams {
    pub fn new_from_env_vars() -> Result<Self, Error> {
        let bucket_name = std::env::var("LOKI_BUCKET_NAME")
            .context("Could not read mandatory env var LOKI_BUCKET_NAME")?;

        let bucket_url = std::env::var("LOKI_BUCKET_URL")
            .context("Could not read mandatory env var LOKI_BUCKET_URL")?;

        let bucket_region = std::env::var("LOKI_BUCKET_REGION")
            .context("Could not read mandatory env var LOKI_BUCKET_REGION")?;

        let bucket_credentials = BucketCredentials::new_from_env_vars()
            .context("Could not read bucket credentials from env vars")?;

        let path_style = parse_env_var(
            "LOKI_BUCKET_PATH_STYLE",
            default_path_style(),
            bool::from_str,
        );

        let data_path_key = std::env::var("LOKI_BUCKET_DATA_PATH")
            .context("Could not read mandatory env var LOKI_BUCKET_DATA_PATH")?;

        let bucket_timeout = parse_env_var(
            "LOKI_BUCKET_TIMEOUT",
            default_bucket_timeout(),
            PositiveDuration::from_str,
        );

        Ok(Self {
            bucket_name,
            bucket_url,
            bucket_region,
            path_style,
            bucket_credentials,
            data_path_key,
            bucket_timeout,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "credentials_type", rename_all = "snake_case")]
pub enum BucketCredentials {
    Explicit(ExplicitCredentials),
    AwsHttpCredentials,
}

impl BucketCredentials {
    pub fn new_from_env_vars() -> Result<Self, Error> {
        let credentials_type = read_env_var(
            "LOKI_BUCKET_CREDENTIALS_TYPE",
            "aws_http_credentials".to_string(),
            |s| s.to_string(),
        );

        match credentials_type.trim() {
            "explicit" => {
                let explicit_credentials = ExplicitCredentials::new_from_env_vars()
                .context("LOKI_BUCKET_CREDENTIALS_TYPE is set to 'explicit' but I could not read explicit bucket credentials from env vars")?;
                Ok(BucketCredentials::Explicit(explicit_credentials))
            }
            "aws_http_credentials" => Ok(BucketCredentials::AwsHttpCredentials),
            _ => {
                anyhow::bail!(
                    "Bad LOKI_BUCKET_CREDENTIALS_TYPE : '{}'. Allowed values are 'explicit' or 'aws_http_credentials'",
                    credentials_type
                );
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExplicitCredentials {
    pub access_key: String,
    pub secret_key: String,
}

impl ExplicitCredentials {
    pub fn new_from_env_vars() -> Result<Self, Error> {
        let access_key = std::env::var("LOKI_BUCKET_ACCESS_KEY")
            .context("Could not read mandatory env var LOKI_BUCKET_ACCESS_KEY")?;
        let secret_key = std::env::var("LOKI_BUCKET_SECRET_KEY")
            .context("Could not read mandatory env var LOKI_BUCKET_SECRET_KEY")?;

        Ok(Self {
            access_key,
            secret_key,
        })
    }
}

#[cfg(test)]
mod tests {

    use crate::server_config::data_source_params::{BucketCredentials, BucketParams};

    #[test]
    fn test_toml_default_credentials() {
        let str = "
        bucket_url = 's3.eu-west-1.amazonaws.com'
        bucket_region = 'eu-west-1'
        bucket_name = 'loki'
        path_style = false
        data_path_key = 'my_coverage/ntfs.zip'
        bucket_timeout = '00:01:00'
        ";
        let result: Result<BucketParams, _> = toml::from_str(&str);
        assert!(
            result.is_ok(),
            "Error while reading config file {:?} : {:#?}",
            &str,
            result
        );
        let params = result.unwrap();
        assert_eq!(
            params.bucket_credentials,
            BucketCredentials::AwsHttpCredentials
        );
    }

    #[test]
    fn test_vars_empty() {
        let params = BucketParams::new_from_env_vars();
        assert!(params.is_err());
    }

    #[test]
    fn test_env_vars_bucket_name_only() {
        temp_env::with_var("LOKI_BUCKET_NAME", Some("my_bucket_name"), || {
            let params = BucketParams::new_from_env_vars();

            assert!(
                params.is_err(),
                "Error reading config from env vars {:?}",
                params
            );
        })
    }

    #[test]
    fn test_env_vars_no_credentials_type() {
        temp_env::with_vars(
            vec![
                ("LOKI_BUCKET_NAME", Some("my_bucket_name")),
                ("LOKI_BUCKET_URL", Some("my.bucket.url")),
                ("LOKI_BUCKET_REGION", Some("my_region")),
            ],
            || {
                let params = BucketParams::new_from_env_vars();

                assert!(
                    params.is_err(),
                    "Error reading config from env vars {:?}",
                    params
                );
            },
        )
    }

    #[test]
    fn test_env_vars_minimal() {
        temp_env::with_vars(
            vec![
                ("LOKI_BUCKET_NAME", Some("my_bucket_name")),
                ("LOKI_BUCKET_URL", Some("my.bucket.url")),
                ("LOKI_BUCKET_REGION", Some("my_region")),
                ("LOKI_BUCKET_DATA_PATH", Some("path/to/my/file")),
                ("LOKI_BUCKET_CREDENTIALS_TYPE", Some("aws_http_credentials")),
            ],
            || {
                let params = BucketParams::new_from_env_vars();

                assert!(
                    params.is_ok(),
                    "Error reading config from env vars {:?}",
                    params
                );
                let params = params.unwrap();
                assert_eq!(&params.bucket_name, "my_bucket_name");
                assert_eq!(&params.bucket_url, "my.bucket.url");
                assert_eq!(&params.bucket_region, "my_region");
                assert_eq!(&params.data_path_key, "path/to/my/file");
                assert_eq!(
                    params.bucket_credentials,
                    BucketCredentials::AwsHttpCredentials
                );
            },
        )
    }
}
