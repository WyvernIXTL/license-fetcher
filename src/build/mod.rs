// Copyright Adam McKellar 2024, 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::env::var_os;
use std::fs::write;
use std::time::Instant;

mod cache;
/// Configuration structs and builders.
pub mod config;
#[cfg(test)]
mod debug;
/// Errors that might appear during build.
pub mod error;
pub mod fetch;

/// Logic for reading metadata of a package.
pub mod metadata;

use cache::{populate_with_cache, CacheError};
use config::Config;
use error_stack::Result;
use error_stack::ResultExt;
use fetch::license_text_from_folder;
use log::{error, info, warn};
use metadata::package_list;
use miniz_oxide::deflate::compress_to_vec;
use thiserror::Error;

use crate::*;
use fetch::populate_package_list_licenses;

#[derive(Debug, Clone, Copy, Error)]
pub enum BuildError {
    #[error("Failed to fetch metadata and generate a package list.")]
    FailedMetadataFetching,
    #[error("Failed loading cache with a read error.")]
    CacheReadError,
    #[error("Failed fetching licenses from cargo src folders.")]
    FailedLicenseFetch,
    #[error("Unexpected error. (ꞋꞋŏ_ŏ)")]
    Unexpected,
}

/// Generates a package list with package name, authors and license text.
///
/// Fetches the the metadata of a cargo project via `cargo metadata` and walks the `.cargo/registry/src` path, searching for license files of dependencies.
pub fn package_list_with_licenses(config: Config) -> Result<PackageList, BuildError> {
    let mut package_list =
        package_list(&config.metadata_config).change_context(BuildError::FailedMetadataFetching)?;

    if config.cache {
        if let Err(err) = populate_with_cache(&mut package_list) {
            match err.current_context() {
                CacheError::Invalid => {
                    error!(err:%; "Cache is invalid. Skipping cache.");
                }
                CacheError::NotBuildScript => {
                    warn!(err:%; "Loading licenses from cache is not available for non build script environments.")
                }
                CacheError::ReadError => return Err(err.change_context(BuildError::CacheReadError)),
            }
        }
    }

    populate_package_list_licenses(&mut package_list, config.cargo_home_dir)
        .change_context(BuildError::FailedLicenseFetch)?;

    let root_pos = package_list
        .iter()
        .position(|e| e.is_root_pkg)
        .ok_or(BuildError::Unexpected)
        .attach_printable_lazy(|| "Root package is not in package list.")?;

    package_list.swap(0, root_pos);
    package_list[1..].sort();

    package_list[0].license_text = license_text_from_folder(&config.metadata_config.manifest_dir)
        .change_context(BuildError::FailedLicenseFetch)?;

    Ok(package_list)
}

impl PackageList {
    /// Writes the [PackageList] into [`env!("OUT_DIR")/LICENSE-3RD-PARTY.bincode`](`env!("OUT_DIR")`)
    ///
    /// If the `compress` feature is set, the output is is compressed as well.
    ///
    /// [`env!("OUT_DIR")`]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
    pub fn write(self) {
        let mut path = var_os("OUT_DIR").unwrap();
        path.push("/LICENSE-3RD-PARTY.bincode");

        let data = bincode::encode_to_vec(self, bincode::config::standard()).unwrap();

        info!("License data size: {} Bytes", data.len());
        let instant_before_compression = Instant::now();

        let compressed_data = compress_to_vec(&data, 10);

        info!(
            "Compressed data size: {} Bytes in {}ms",
            compressed_data.len(),
            instant_before_compression.elapsed().as_millis()
        );

        info!("Writing to file: {:?}", &path);
        write(path, compressed_data).unwrap();
    }
}
