// Copyright Adam McKellar 2024, 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Functions for fetching metadata and licenses and writing license data.
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
//! use license_fetcher::prelude::*;
//!
//! fn main() {
//!     // Config with environment variables set by cargo, to fetch licenses at build time.
//!     let config: Config = ConfigBuilder::from_build_env()
//!         .build()
//!         .expect("failed to build configuration");
//!
//!     // `packages` does not hold any licenses!
//!     let packages: PackageList = package_list(&config)
//!         .expect("failed to fetch metadata");
//!
//!     // Write packages to out dir to be embedded.
//!     packages.write_package_list_to_out_dir().expect("failed to write package list");
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
//! use license_fetcher::prelude::*;
//!
//! fn main() {
//!     // Config with environment variables set by cargo, to fetch licenses at build time.
//!     let config: Config = ConfigBuilder::from_build_env()
//!         .build()
//!         .expect("failed to build configuration");
//!
//!     let packages: PackageList = package_list_with_licenses(&config)
//!                                     .expect("failed to fetch metadata or licenses");
//!
//!     // Write packages to out dir to be embedded.
//!     packages.write_package_list_to_out_dir().expect("failed to write package list");
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
//! use license_fetcher::prelude::*;
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
//!     std::fs::File::create(path).expect("failed to create dummy file");
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
//!   On the other hand the embedded license data being empty needs to be handled.
//!
//! _Handling the dummy file:_
//! ```
//! use std::process::exit;
//!
//! use license_fetcher::prelude::*;
//!
//! fn main() {
//!     match read_package_list_from_out_dir!() {
//!         Ok(package_list) => println!("{package_list}"),
//!         Err(UnpackError::Empty) => {
//!             eprintln!(
//!                 "failed to embed license data during build. Please see ... for license data information"
//!             );
//!             exit(0); // or exitcode 1 for signaling failure
//!         }
//!         Err(err) => {
//!             eprintln!(
//!                 "an error during decompression or deserialization of license data has occurred:\n{err}"
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

/// Configuration structs and builders.
pub mod config;

/// Functions for fetching metadata and licenses.
pub mod fetcher;

/// Methods for serializing, compressing and writing of [`PackageList`](crate::PackageList).
pub mod write;
