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
//! use license_fetcher::build::package_list;
//! use license_fetcher::PackageList;
//!
//! fn main() {
//!     // Config with environment variables set by cargo, to fetch licenses at build time.
//!     let config: Config = ConfigBuilder::from_build_env()
//!         .build()
//!         .expect("Failed to build configuration.");
//!
//!     // `packages` does not hold any licenses!
//!     let packages: PackageList = package_list(&config)
//!         .expect("Failed to fetch metadata.");
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
//! To counteract these issues, you might want to use environment variables to force the
//! fetching of licenses in CI and soft fail it when installing from source.
//!
//! `build.rs`
//!
//! ```
//! use std::{env::VarError, error::Error, path::PathBuf};
//!
//! use license_fetcher::{
//!     OUT_FILE_NAME, PackageList,
//!     build::{
//!         config::{Config, ConfigBuilder},
//!         package_list_with_licenses,
//!     },
//! };
//!
//! fn fetch_and_embed_licenses() -> Result<(), Box<dyn Error>> {
//!     let config: Config = ConfigBuilder::from_build_env().build()?;
//!     let packages: PackageList = package_list_with_licenses(config)?;
//!     packages.write_package_list_to_out_dir()?;
//!     Ok(())
//! }
//!
//! fn create_dummy_file() {
//!     let out_dir = std::env::var_os("OUT_DIR").expect("OUT_DIR not set");
//!     let path = PathBuf::from(out_dir).join(OUT_FILE_NAME);
//!     std::fs::File::create(path).expect("Failed to create dummy file");
//! }
//!
//! fn main() {
//!     match std::env::var("LICENSE_FETCHER") {
//!         Ok(mode) => match mode.as_str() {
//!             "FORCE" => fetch_and_embed_licenses().unwrap(),
//!             "SKIP" => {
//!                 eprintln!("Skipping license fetching.");
//!                 create_dummy_file();
//!             }
//!             wrong_arg => {
//!                 eprintln!(
//!                     "Env var `LICENSE_FETCHER` should be set `FORCE` or `SKIP`, not {wrong_arg}."
//!                 );
//!                 create_dummy_file();
//!             }
//!         },
//!         Err(VarError::NotPresent) => {
//!             eprintln!("`LICENSE_FETCHER` not set. Defaulting to fetching licenses.");
//!             if let Err(err) = fetch_and_embed_licenses() {
//!                 eprintln!("An error occurred during license fetch:\n{err:?}");
//!                 create_dummy_file();
//!             }
//!         }
//!         Err(VarError::NotUnicode(_)) => {
//!             eprintln!("Env var `LICENSE_FETCHER` must be valid unicode.");
//!             eprintln!("Skipping license fetching.");
//!             create_dummy_file();
//!         }
//!     }
//!
//!     println!("cargo::rerun-if-changed=build.rs");
//!     println!("cargo::rerun-if-changed=Cargo.lock");
//!     println!("cargo::rerun-if-changed=Cargo.toml");
//! }
//! ```
//!
//! This results in 3 modes:
//! * **Force** (`LICENSE_FETCHER=FORCE`): The build will fail, if license fetcher did not succeed. This will hinder you publishing a binary without attribution of your dependencies.
//! * **Skip** (`LICENSE_FETCHER=SKIP`): The license fetching step will be skipped.
//! * **Soft Fail**: If someone installs your software from source via `cargo install`, the build will never fail because of license fetcher.
//!     On the other hand the embedded license data being empty needs to be handled.
//!
//! _Handling the dummy file:_
//! ```
//! use std::process::exit;
//!
//! use license_fetcher::{error::UnpackError, read_package_list_from_out_dir};
//!
//! fn main() {
//!     match read_package_list_from_out_dir!() {
//!         Ok(package_list) => println!("{package_list}"),
//!         Err(UnpackError::Empty) => {
//!             eprintln!(
//!                 "Failed to embed license data during build. Please see ... for license data information."
//!             );
//!             exit(0); // or exitcode 1 for signaling failure
//!         }
//!         Err(err) => {
//!             eprintln!(
//!                 "An error during decompression or deserialization of license data has occurred:\n{err}"
//!             );
//!             exit(1);
//!         }
//!     }
//! }
//! ```
//!
//! This way if a dummy is written the program fails gracefully:
//! ```code
//! $ LICENSE_FETCHER=SKIP cargo run
//! Failed to embed license data during build. Please see ... for license data information.
//! ```
//!

mod cache;

/// Errors that might appear during build.
pub mod error;

mod fetch;

mod wrapper;

mod metadata;

use exn::OptionExt;
use exn::Result;
use exn::ResultExt;
use fetch::license_texts_from_folder;
use metadata::package_list_impl;

use crate::build::config::Config;
use crate::build::config::MetadataConfig;
use crate::build::fetcher::cache::populate_with_cache_from_package_list;
use crate::build::fetcher::cache::read_package_list_with_tests;
use crate::build::fetcher::error::LicenseFetcherError;
use crate::build::fetcher::error::IE;
use crate::build::fetcher::wrapper::PackageWrapper;
use crate::Package;
use crate::PackageList;
use fetch::populate_package_list_licenses;

fn wrap_package_iter(
    package_iter: impl Iterator<Item = Package>,
) -> impl Iterator<Item = PackageWrapper> {
    package_iter.map(|package| PackageWrapper {
        package,
        restored_from_cache: false,
    })
}

fn sort_package_list(root_package_name: &str, package_vec: &mut [Package]) -> Result<(), IE> {
    let root_pos = package_vec
        .iter()
        .position(|e| e.name == root_package_name)
        .ok_or_raise(|| IE::new("root crate should be part of license metadata"))?;

    package_vec.swap(0, root_pos);
    package_vec[1..].sort();

    Ok(())
}

fn attach_root_package_license(config: &Config, root_package: &mut Package) -> Result<(), IE> {
    root_package.license_texts = license_texts_from_folder(&config.metadata_config.manifest_dir)
        .or_raise(|| {
            IE::new("license of root package should fetch from it's manifest directory")
        })?;

    Ok(())
}

fn package_list_internal(config: impl AsRef<MetadataConfig>) -> Result<PackageList, IE> {
    let (package_root_name, package_iter) =
        package_list_impl(config.as_ref()).or_raise(|| IE::new("license metadata should fetch"))?;

    let mut package_vec: Vec<Package> = package_iter.collect();

    sort_package_list(package_root_name.as_str(), &mut package_vec)
        .or_raise(|| IE::new("crate list should sort"))?;

    Ok(package_vec.into())
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
pub fn package_list(
    config: impl AsRef<MetadataConfig>,
) -> Result<PackageList, LicenseFetcherError> {
    package_list_internal(config).map_err(LicenseFetcherError::from_internal)
}

fn package_list_with_licenses_internal(config: impl AsRef<Config>) -> Result<PackageList, IE> {
    let (root_package_name, package_iter) = package_list_impl(&config.as_ref().metadata_config)
        .or_raise(|| IE::new("license metadata should fetch"))?;

    let mut wrapped_package_vec: Vec<PackageWrapper> =
        if let Some(cache_path) = &config.as_ref().cache_path {
            let cached_packages = read_package_list_with_tests(cache_path).or_raise(|| {
                IE::new("reading cache from cache path should succeed").with_path(cache_path)
            })?;
            populate_with_cache_from_package_list(package_iter, cached_packages).collect()
        } else {
            wrap_package_iter(package_iter).collect()
        };

    populate_package_list_licenses(&mut wrapped_package_vec, &config.as_ref().cargo_home_dir)
        .or_raise(|| IE::new("populating packages with licenses should succeed"))?;

    let mut package_vec = Vec::with_capacity(wrapped_package_vec.capacity());
    package_vec.extend(wrapped_package_vec.into_iter().map(|p| p.package));

    sort_package_list(&root_package_name, &mut package_vec)
        .or_raise(|| IE::new("crate list should sort"))?;

    attach_root_package_license(config.as_ref(), &mut package_vec[0])
        .or_raise(|| IE::new("license should fetch for root package"))?;

    Ok(package_vec.into())
}

/// Generates a package list with package name, authors and license text.
///
/// Fetches the the metadata of a cargo project via `cargo metadata` and walks the `.cargo/registry/src` path, searching for license files of dependencies.
pub fn package_list_with_licenses(
    config: impl AsRef<Config>,
) -> Result<PackageList, LicenseFetcherError> {
    package_list_with_licenses_internal(config).map_err(LicenseFetcherError::from_internal)
}
