// Copyright Adam McKellar 2024, 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! This module holds functions to fetch metadata and licenses.
//!
//!
//! ## Examples
//!
//! The examples here are directed for fetching licenses during build time.
//! They can also applied for use with applications if configured correctly.
//!
//! See the [`config` module](crate::build::config).
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
//!     let (_root_package_name, package_iter) = package_list(&config.metadata_config)
//!         .expect("Failed to fetch metadata.");
//!
//!     let packages: PackageList = package_iter.collect::<Vec<_>>().into();
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
//!     let packages: PackageList = package_list_with_licenses(&config)
//!                                     .expect("Failed to fetch metadata or licenses.");
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
//! `build.rs`
//!
//!```
//! use std::error::Error;
//!
//! use license_fetcher::build::config::{ConfigBuilder, Config};
//! use license_fetcher::build::package_list_with_licenses;
//! use license_fetcher::PackageList;
//!
//! fn fetch_and_embed_licenses() -> Result<(), Box<dyn Error>> {
//!     // Config with environment variables set by cargo, to fetch licenses at build time.
//!     let config: Config = ConfigBuilder::from_build_env().build()?;
//!
//!     let packages: PackageList = package_list_with_licenses(&config)?;
//!
//!     // Write packages to out dir to be embedded.
//!     packages.write_package_list_to_out_dir()?;
//!
//!     Ok(())
//! }
//!
//! // Create empty dummy file so that the embedding does not fail.
//! fn dummy_file() {
//!     let mut path = std::env::var_os("OUT_DIR")
//!         .expect("Creation of dummy file failed: Environment variable 'OUT_DIR' not set.");
//!     path.push("/LICENSE-3RD-PARTY.bincode.deflate");
//!     let _ = std::fs::File::create(path)
//!         .expect("Creation of dummy file failed: Write failed.");
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
//!                 eprintln!("Wrong environment variable `LICENSE_FETCHER`!");
//!                 eprintln!("Expected either ``, `production` or `development`.");
//!
//!                 dummy_file();
//!             }
//!         }
//!     } else {
//!         if let Err(err) = fetch_and_embed_licenses() {
//!             eprintln!("An error occurred during license fetch:\n\n");
//!             eprintln!("{:?}", err);
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
//!
//! ## Error Handling
//!
//! See [error-stack](https://docs.rs/error-stack/latest/error_stack/struct.Report.html).
//!

use std::env::var_os;
use std::error::Error;
use std::fs::write;
use std::path::PathBuf;
use std::time::Instant;

mod cache;
/// Configuration structs and builders.
pub mod config;
#[cfg(test)]
mod debug;
/// Errors that might appear during build.
pub mod error;

mod fetch;

mod wrapper;

mod metadata;

use cache::CacheError;
use config::Config;
use error_stack::Result;
use error_stack::ResultExt;
use fetch::license_text_from_folder;
use log::{error, info};
use lz4_flex::compress_prepend_size;
use metadata::package_list_impl;
use nanoserde::SerBin;

use crate::build::cache::populate_with_cache_from_package_list;
use crate::build::cache::read_package_list_with_tests;
use crate::build::config::MetadataConfig;
use crate::build::wrapper::PackageWrapper;
use crate::Package;
use crate::PackageList;
use crate::OUT_FILE_NAME;
use fetch::populate_package_list_licenses;

/// Error that might occur during fetching of license data.
#[derive(Debug, Clone, Copy, displaydoc::Display)]
pub enum BuildError {
    /// failed to fetch package metadata with `cargo metadata` and `cargo tree`
    FailedMetadataFetching,
    /// failed to read cache with an io error
    CacheReadError,
    /// failed to read licenses from cargo sources folder
    FailedLicenseFetch,
    /// root package is not in output license data
    RootPackageNotInOutput,
}

impl Error for BuildError {}

fn wrap_package_iter(
    package_iter: impl Iterator<Item = Package>,
) -> impl Iterator<Item = PackageWrapper> {
    package_iter.map(|package| PackageWrapper {
        package,
        restored_from_cache: false,
    })
}

fn sort_package_list(
    root_package_name: &str,
    package_vec: &mut [Package],
) -> Result<(), BuildError> {
    let root_pos = package_vec
        .iter()
        .position(|e| e.name == root_package_name)
        .ok_or(BuildError::RootPackageNotInOutput)
        .attach_printable_lazy(|| "Root package is not in package list.")?;

    package_vec.swap(0, root_pos);
    package_vec[1..].sort();

    Ok(())
}

fn attach_root_package_license(
    config: &Config,
    root_package: &mut Package,
) -> Result<(), BuildError> {
    root_package.license_text = license_text_from_folder(&config.metadata_config.manifest_dir)
        .change_context(BuildError::FailedLicenseFetch)?;

    Ok(())
}

/// Get a list of dependencies.
///
/// [`cargo metadata`] and [`cargo tree`] are use in combination to get all used dependencies and their metadata.
///
/// (The reason for using `cargo tree` as well is, that I had some issues at some time, with `cargo metadata`
/// including unused dependencies. I am not sure why this was the case, as I am failing to reproduce this problem currently.)
///
/// [`cargo tree`]: https://doc.rust-lang.org/cargo/commands/cargo-tree.html
/// [`cargo metadata`]: https://doc.rust-lang.org/cargo/commands/cargo-metadata.html
///
pub fn package_list(config: impl AsRef<MetadataConfig>) -> Result<PackageList, BuildError> {
    let (package_root_name, package_iter) =
        package_list_impl(config.as_ref()).change_context(BuildError::FailedMetadataFetching)?;

    let mut package_vec: Vec<Package> = package_iter.collect();

    sort_package_list(package_root_name.as_str(), &mut package_vec)?;

    Ok(package_vec.into())
}

/// Generates a package list with package name, authors and license text.
///
/// Fetches the the metadata of a cargo project via `cargo metadata` and walks the `.cargo/registry/src` path, searching for license files of dependencies.
pub fn package_list_with_licenses(config: impl AsRef<Config>) -> Result<PackageList, BuildError> {
    let (root_package_name, package_iter) = package_list_impl(&config.as_ref().metadata_config)
        .change_context(BuildError::FailedMetadataFetching)?;

    let mut wrapped_package_vec: Vec<PackageWrapper> = if let Some(cache_path) =
        &config.as_ref().cache_path
    {
        match read_package_list_with_tests(cache_path) {
            Ok(cache) => populate_with_cache_from_package_list(package_iter, cache).collect(),
            Err(err) => match err.current_context() {
                CacheError::Invalid => {
                    error!("Cache is invalid. Skipping cache. Error: \n{err}");
                    wrap_package_iter(package_iter).collect()
                }
                CacheError::ReadError => return Err(err.change_context(BuildError::CacheReadError)),
            },
        }
    } else {
        wrap_package_iter(package_iter).collect()
    };

    populate_package_list_licenses(&mut wrapped_package_vec, &config.as_ref().cargo_home_dir)
        .change_context(BuildError::FailedLicenseFetch)?;

    let mut package_vec = Vec::with_capacity(wrapped_package_vec.capacity());
    package_vec.extend(wrapped_package_vec.into_iter().map(|p| p.package));

    sort_package_list(&root_package_name, &mut package_vec)?;

    attach_root_package_license(config.as_ref(), &mut package_vec[0])?;

    Ok(package_vec.into())
}

/// Errors that might occur during the writing process of the license data to the output directory.
#[derive(Debug, Clone, Copy, displaydoc::Display)]
pub enum WriteError {
    /// failed writing license data to output directory
    Write,
    /// function was called not in build script which is disallowed
    NotBuildScript,
}

impl Error for WriteError {}

impl PackageList {
    /// Encodes and compresses a [`PackageList`].
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let data = self.serialize_bin();

        info!("License data size: {} Bytes", data.len());
        let instant_before_compression = Instant::now();

        let compressed_data = compress_prepend_size(&data);

        info!(
            "Compressed data size: {} Bytes in {}ms",
            compressed_data.len(),
            instant_before_compression.elapsed().as_millis()
        );

        compressed_data
    }

    /// Writes the [`PackageList`] into [`$OUT_DIR/LICENSE-3RD-PARTY.bincode.deflate`](`env!("OUT_DIR")`)
    ///
    /// `$OUT_DIR` is set by cargo during build. This function is meant to be only used inside a build script
    /// and only in conjunction with [`read_package_list_from_out_dir`].
    ///
    /// [`env!("OUT_DIR")`]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
    ///
    /// ## Errors
    ///
    /// Returns [`WriteError`] if the license file was failed to be written to the `OUT_DIR` or if more importabntly this function was not called from a build script!
    /// The reason for the latter variant, [`WriteError::NotBuildScript`], is that this function depends on environment variables set during
    /// compilation.
    ///
    pub fn write_package_list_to_out_dir(&self) -> Result<(), WriteError> {
        let compressed_data = self.encode();

        let path =
            PathBuf::from(var_os("OUT_DIR").ok_or(WriteError::NotBuildScript)?).join(OUT_FILE_NAME);

        info!("Writing to file: {}", &path.display());
        write(path, compressed_data).change_context(WriteError::Write)?;

        Ok(())
    }
}
