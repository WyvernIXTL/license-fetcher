// Copyright Adam McKellar 2024, 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! This module holds functions to fetch metadata and licenses.
//!
//! ## Configuration
//!
//! There is some configuration. See the [`config` module](license-fetcher::build::config).
//!
//! ## Examples
//!
//! ### Fetch Metadata Only
//!
//! If you are not interested in fetching licenses, license-fetcher is able to
//! only fetch metadata of packages:
//!
//! `build.rs`
//!
//! ```
//! use license_fetcher::build::config::{ConfigBuilder, Config};
//! use license_fetcher::build::metadata::package_list;
//! use license_fetcher::PackageList;
//!
//! fn main() {
//!     // Config with environment variables set by cargo, to fetch licenses at build time.
//!     let config: Config = ConfigBuilder::from_build_env()
//!         .build()
//!         .expect("Failed to build configuration.");
//!
//!     // `packages` does not hold any licenses!
//!     let packages: PackageList = package_list(&config.metadata_config).expect("Failed to fetch metadata.");
//!
//!     // Write packages to out dir to be embedded.
//!     packages.write_package_list_to_out_dir().expect("Failed to write package list.");
//!
//!     // Rerun only if one of the following files changed:
//!     println!("cargo::rerun-if-changed=build.rs");
//!     println!("cargo::rerun-if-changed=Cargo.lock");
//!     println!("cargo::rerun-if-changed=Cargo.toml");
//! }
//! ```
//!
//!
//! ### Fetch Metadata and Licenses
//!
//! `build.rs`
//!
//! ```
//! use license_fetcher::build::config::{ConfigBuilder, Config};
//! use license_fetcher::build::package_list_with_licenses;
//! use license_fetcher::PackageList;
//!
//! fn main() {
//!     // Config with environment variables set by cargo, to fetch licenses at build time.
//!     let config: Config = ConfigBuilder::from_build_env()
//!         .build()
//!         .expect("Failed to build configuration.");
//!
//!     let packages: PackageList = package_list_with_licenses(config).expect("Failed to fetch metadata or licenses.");
//!
//!     // Write packages to out dir to be embedded.
//!     packages.write_package_list_to_out_dir().expect("Failed to write package list.");
//!
//!     // Rerun only if one of the following files changed:
//!     println!("cargo::rerun-if-changed=build.rs");
//!     println!("cargo::rerun-if-changed=Cargo.lock");
//!     println!("cargo::rerun-if-changed=Cargo.toml");
//! }
//! ```
//!
//! ### Advanced
//!
//! Most often there is no need to fetch licenses during development.
//! Also there is the potential issue of the build failing, just because license fetcher did.
//! To counteract these issues, you might want to use environment variables to set the behavior during build time:
//!
//!```
//! use license_fetcher::build::config::{ConfigBuilder, Config};
//! use license_fetcher::build::package_list_with_licenses;
//! use license_fetcher::PackageList;
//!
//! // license-fetcher uses `error_stack` for structured errors.
//! use error_stack::{Result, ResultExt};
//!
//! #[derive(Debug)]
//! enum BuildScriptError {
//!     ConfigBuild,
//!     LicenseFetch,
//!     WriteLicenses
//! }
//!
//! impl std::fmt::Display for BuildScriptError {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         match self {
//!             Self::ConfigBuild => writeln!(f, "Failed to build config."),
//!             Self::LicenseFetch => writeln!(f, "Failed to fetch licenses."),
//!             Self::WriteLicenses => writeln!(f, "Failed to write licenses into out folder."),
//!         }
//!     }
//! }
//!
//! impl std::error::Error for BuildScriptError {}
//!
//! fn fetch_and_embed_licenses() -> Result<(), BuildScriptError> {
//!     // Config with environment variables set by cargo, to fetch licenses at build time.
//!     let config: Config = ConfigBuilder::from_build_env()
//!         .build()
//!         .change_context(BuildScriptError::ConfigBuild)?;
//!
//!     let packages: PackageList = package_list_with_licenses(config).change_context(BuildScriptError::LicenseFetch)?;
//!
//!     // Write packages to out dir to be embedded.
//!     packages.write_package_list_to_out_dir().change_context(BuildScriptError::WriteLicenses)?;
//!
//!     Ok(())
//! }
//!
//! // Create empty dummy file so that the embedding does not fail.
//! fn dummy_file() {
//!     let mut path = std::env::var_os("OUT_DIR").unwrap();
//!     path.push("/LICENSE-3RD-PARTY.bincode.deflate");
//!     let _ = std::fs::File::create(path);
//! }
//!
//! fn main() {
//!     if let Some(mode) = std::env::var_os("LICENSE_FETCHER") {
//!         match mode.to_ascii_lowercase().to_string_lossy().as_ref() {
//!             "production" => fetch_and_embed_licenses().unwrap(),
//!             "development" => {
//!                 eprintln!("Skipping license fetching.");
//!                 dummy_file();
//!             },
//!             &_ => {
//!                 eprintln!("Wrong environment variable!");
//!                 dummy_file();
//!             }
//!         }
//!     } else {
//!         if let Err(err) = fetch_and_embed_licenses() {
//!             eprintln!("An error occurred during license fetch:\n\n");
//!             eprintln!("{}", err);
//!
//!             dummy_file();
//!         }
//!     }
//!
//!     // Rerun only if one of the following files changed:
//!     println!("cargo::rerun-if-changed=build.rs");
//!     println!("cargo::rerun-if-changed=Cargo.lock");
//!     println!("cargo::rerun-if-changed=Cargo.toml");
//! }
//! ```
//!
//! This results in 3 modes:
//! * **Production**: The build will fail, if license fetcher did not succeed. This will hinder you publishing a binary without attribution of your dependencies.
//! * **Development**: license fetching step will be skipped.
//! * **Soft Fail**: If someone installs your software from source via `cargo install`, the build will never fail because of license fetcher.
//!     On the other hand the execution might fail, when trying to print the licenses.
//!
//! I know that this is not pretty and I'll think about how to solve that in a future release.
//! If you have a nicer `build.rs` don't shy away from sharing it :)
//!
//! ## Error Handling
//!
//! See [error-stack](https://docs.rs/error-stack/latest/error_stack/struct.Report.html).
//!

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
#[doc(hidden)]
pub mod fetch;

/// Logic for reading metadata of a package.
pub mod metadata;

use bincode::error::EncodeError;
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

#[derive(Debug, Clone, Copy, Error)]
pub enum WriteError {
    #[error("Failed to encode package list.")]
    Encode,
    #[error("Failed to write encoded package list.")]
    Write,
    #[error("Executed not inside a build script.")]
    NotBuildScript,
}

impl PackageList {
    /// Encodes and compresses a [PackageList].
    pub fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let data = bincode::encode_to_vec(self, bincode::config::standard())?;

        info!("License data size: {} Bytes", data.len());
        let instant_before_compression = Instant::now();

        let compressed_data = compress_to_vec(&data, 10);

        info!(
            "Compressed data size: {} Bytes in {}ms",
            compressed_data.len(),
            instant_before_compression.elapsed().as_millis()
        );

        Ok(compressed_data)
    }

    /// Writes the [PackageList] into [`$OUT_DIR/LICENSE-3RD-PARTY.bincode.deflate`](`env!("OUT_DIR")`)
    ///
    /// `$OUT_DIR` is set by cargo during build. This function is meant to be only used inside a build script
    /// and only in conjunction with [read_package_list_from_out_dir].
    ///
    /// [`env!("OUT_DIR")`]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
    pub fn write_package_list_to_out_dir(&self) -> Result<(), WriteError> {
        let compressed_data = self.encode().change_context(WriteError::Encode)?;

        let mut path = var_os("OUT_DIR").ok_or(WriteError::NotBuildScript)?;
        path.push("/LICENSE-3RD-PARTY.bincode.deflate");

        info!("Writing to file: {:?}", &path);
        write(path, compressed_data).change_context(WriteError::Write)?;

        Ok(())
    }
}
