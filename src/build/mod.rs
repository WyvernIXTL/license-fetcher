// Copyright Adam McKellar 2024, 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use config::Config;
use miniz_oxide::deflate::compress_to_vec;
use std::collections::{BTreeSet, HashSet};
use std::env::{var, var_os};
use std::ffi::OsString;
use std::fs::write;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use thiserror::Error;

use log::info;
use serde_json::from_slice;

mod cache;
/// Configuration structs and builders.
pub mod config;
#[cfg(test)]
mod debug;
/// Errors that might appear during build.
pub mod error;
mod fetch;

/// Logic for reading metadata of a package.
pub mod metadata;

use crate::*;
use fetch::{license_text_from_folder, licenses_text_from_cargo_src_folder};

#[derive(Debug, Clone, Copy, Error)]
pub enum BuildError {}

pub fn package_list_with_licenses(config: Config) -> Result<PackageList, BuildError> {
    todo!()
}

/// Generates a package list with package name, authors and license text.
///
/// Fetches the the metadata of a cargo project via `cargo metadata` and walks the `.cargo/registry/src` path, searching for license files of dependencies.
/// This function is not dependant on environment variables and thus also useful outside of build scripts.
///
/// ### Arguments
///
/// * **cargo_path** - Absolute path to cargo executable. If omitted, tries to fetch the path from `PATH`.
/// * **manifest_dir_path** - Relative or absolute path to manifest dir.
/// * **this_package_name** - Name of the package. `cargo metadata` does not disclose the name, but it is needed for parsing the used licenses.
///
/// ### Inner Workings
///
/// This function:
/// 1. Calls `cargo metadata --frozen`.
/// 2. Traverses all dependencies not marked as `build` or as `dev` dependencies and writes the metadata like name and version to a [PackageList].
/// 3. Calls `cargo tree -e normal --frozen`.
/// 4. Filters the [PackageList] from step 2. to only contain crates / packages, that `cargo tree` outputted. (This is to omit crates, that are not used.)
/// 5. Licenses are fetched from the subfolders of the `~/.cargo/registry/src` folder, by checking the name and the license version of the crates contained in the [PackageList] against the folders names in said folders.
/// 6. The root crate / package is put at index 0 and all other crates / packages are sorted by their name and version.
///
/// If the executions of the `cargo` commands error and if the `frozen` flag is not set, `cargo` is executed without the `--frozen` argument and is free to write to `Cargo.lock`.
pub fn generate_package_list_with_licenses_without_env_calls(
    cargo_path: Option<OsString>,
    manifest_dir_path: OsString,
    this_package_name: String,
) -> PackageList {
    let cargo_path = cargo_path.unwrap_or_else(|| OsString::from("cargo"));

    let cargo_tree_handle = {
        let cargo_path = cargo_path.clone();
        let manifest_dir_path = manifest_dir_path.clone();
        std::thread::spawn(move || cargo_tree(cargo_path, manifest_dir_path))
    };

    let mut package_list = generate_package_list(cargo_path.clone(), manifest_dir_path.clone());

    if let Some(output) = cargo_tree_handle
        .join()
        .expect("Failed executing cargo tree.")
    {
        package_list = filter_package_list_with_cargo_tree(package_list, output);
    }

    licenses_text_from_cargo_src_folder(&mut package_list).unwrap();

    // Put root crate at front.
    let this_package_index = package_list
        .iter()
        .position(|e| e.name == this_package_name)
        .unwrap();
    package_list.swap(this_package_index, 0);
    package_list[0].license_text =
        license_text_from_folder(&PathBuf::from(manifest_dir_path)).unwrap();

    package_list[1..].sort();

    package_list
}

/// Generates a package list with package name, authors and license text. Uses environment variables supplied by cargo during build.
///
/// This functions usage is in your cargo build script (`build.rs`), that is being run during compilation of your program.
///
/// Uses the [generate_package_list_with_licenses_without_env_calls] function under the hood.
/// The remaining arguments are fetched via the environment variables that cargo sets during compilation:
/// * `CARGO` - The path to the `cargo` executable that compiled this code.
/// * `CARGO_MANIFEST_DIR` - The path to the directory that contains the `Cargo.toml` that this code is compiled for.
/// * `CARGO_PKG_NAME` - The package name of the package this code is compiled for.
///
/// # Example
/// In `build.rs`:
/// ```no_run
/// use license_fetcher::build::generate_package_list_with_licenses;
///
/// fn main() {
///     generate_package_list_with_licenses().write();
///     println!("cargo::rerun-if-changed=build.rs");
///     println!("cargo::rerun-if-changed=Cargo.lock");
///     println!("cargo::rerun-if-changed=Cargo.toml");
/// }
/// ```
pub fn generate_package_list_with_licenses() -> PackageList {
    let cargo_path = var_os("CARGO").unwrap();
    let manifest_dir_path = var_os("CARGO_MANIFEST_DIR").unwrap();
    let this_package_name = var("CARGO_PKG_NAME").unwrap();

    generate_package_list_with_licenses_without_env_calls(
        Some(cargo_path),
        manifest_dir_path,
        this_package_name,
    )
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
