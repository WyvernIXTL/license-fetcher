// Copyright Adam McKellar 2024, 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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

use crate::Package;
use crate::PackageList;
use crate::build::config::Config;
use crate::build::config::MetadataConfig;
use crate::build::fetcher::cache::populate_with_cache_from_package_list;
use crate::build::fetcher::cache::read_package_list_with_tests;
use crate::build::fetcher::error::IE;
use crate::build::fetcher::error::LicenseFetcherError;
use crate::build::fetcher::wrapper::PackageWrapper;
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
) -> std::result::Result<PackageList, LicenseFetcherError> {
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
) -> std::result::Result<PackageList, LicenseFetcherError> {
    package_list_with_licenses_internal(config).map_err(LicenseFetcherError::from_internal)
}
