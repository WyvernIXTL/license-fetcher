// Copyright Adam McKellar 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::thread::scope;

use command::exec_cargo;
use error_stack::{report, Result, ResultExt};
use fnv::{FnvHashMap, FnvHashSet};
use metadata::{Metadata, MetadataResolveNode};
use serde_json::from_slice;
use thiserror::Error;

use crate::{Package, PackageList};

use super::config::MetadataConfig;

mod command;
mod metadata;

#[derive(Debug, Clone, Copy, Error)]
pub enum PkgListFromCargoMetadataError {
    #[error("Failed to execute `cargo metadata`.")]
    ExecCargo,
    #[error("Failed to parse output of `cargo metadata`.")]
    ParseJson,
    #[error("Failed to parse `cargo` output to utf-8 string.")]
    ParseString,
    #[error("Error occurred with thread.")]
    Thread,
}

fn walk_dependencies<'a>(
    used_dependencies: &mut FnvHashSet<&'a String>,
    dependencies: &'a FnvHashMap<&String, &MetadataResolveNode>,
    root: &String,
) {
    let package = match dependencies.get(root) {
        Some(pack) => pack,
        None => return,
    };
    used_dependencies.insert(&package.id);
    for dep in package.deps.iter() {
        if dep.dep_kinds.iter().map(|d| &d.kind).any(|o| o.is_none()) {
            walk_dependencies(used_dependencies, dependencies, &dep.pkg);
        }
    }
}

fn package_list_from_cargo_metadata(
    config: &MetadataConfig,
) -> Result<Vec<Package>, PkgListFromCargoMetadataError> {
    const ARGUMENTS: &'static [&'static str] =
        &["metadata", "--format-version", "1", "--color", "never"];

    let output =
        exec_cargo(config, ARGUMENTS).change_context(PkgListFromCargoMetadataError::ExecCargo)?;

    let metadata_parsed: Metadata =
        from_slice(&output.stdout).change_context(PkgListFromCargoMetadataError::ParseJson)?;

    let packages = metadata_parsed.packages;
    let package_id = metadata_parsed
        .resolve
        .root
        .ok_or(PkgListFromCargoMetadataError::ParseJson)
        .attach_printable("Failed to resolve package id from output.")?;
    let dependencies = metadata_parsed.resolve.nodes;

    let mut used_packages = FnvHashSet::default(); // TODO: Check if there is a speed bump with Vec.
    let dependencies_hash_map = FnvHashMap::from_iter(dependencies.iter().map(|d| (&d.id, d))); // TODO: Check if there is a speed bump with Vec.

    walk_dependencies(&mut used_packages, &dependencies_hash_map, &package_id);

    Ok(packages
        .into_iter()
        .filter(|e| used_packages.contains(&e.id))
        .map(|package| {
            let is_root = package.name.as_ref() == package_id;
            let name_version = format!("{}-{}", package.name, package.version);
            Package {
                license_text: None,
                authors: package.authors,
                license_identifier: package.license,
                name: package.name,
                version: package.version,
                description: package.description,
                homepage: package.homepage,
                repository: package.repository,
                restored_from_cache: false,
                is_root_pkg: is_root,
                name_version,
            }
        })
        .collect())
}

fn used_pkg_names_from_cargo_tree(
    config: &MetadataConfig,
) -> Result<FnvHashSet<String>, PkgListFromCargoMetadataError> {
    const ARGUMENTS: &'static [&'static str] = &[
        "tree",
        "-e",
        "normal",
        "-f",
        "{p}",
        "--prefix",
        "none",
        "--color",
        "never",
        "--no-dedupe",
    ];

    let output =
        exec_cargo(config, ARGUMENTS).change_context(PkgListFromCargoMetadataError::ExecCargo)?;

    Ok(String::from_utf8(output.stdout)
        .change_context(PkgListFromCargoMetadataError::ParseString)?
        .lines()
        .map(|l| l.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .collect::<FnvHashSet<String>>())
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
pub fn package_list(config: MetadataConfig) -> Result<PackageList, PkgListFromCargoMetadataError> {
    scope(|scope| {
        let packages_handle = scope.spawn(|| package_list_from_cargo_metadata(&config));

        // TODO: Check if not spawning this thread makes a difference.
        let used_package_names_handle = scope.spawn(|| used_pkg_names_from_cargo_tree(&config));

        let packages = packages_handle.join().map_err(|e| {
            report!(PkgListFromCargoMetadataError::Thread).attach_printable(format!("{:?}", e))
        })?;
        let used_packages = used_package_names_handle.join().map_err(|e| {
            report!(PkgListFromCargoMetadataError::Thread).attach_printable(format!("{:?}", e))
        })?;
        if packages.is_err() && used_packages.is_err() {
            let mut pkgs_err = packages.unwrap_err();
            let used_pkgs_err = used_packages.unwrap_err();
            pkgs_err.extend_one(used_pkgs_err);
            return Err(pkgs_err);
        }

        let packages = packages?;
        let used_package_names = used_packages?;

        let mut filtered_packages = Vec::with_capacity(packages.capacity());
        let filtered_packages_iter = packages
            .into_iter()
            .filter(|e| used_package_names.contains(&e.name));
        filtered_packages.extend(filtered_packages_iter);

        debug_assert!(filtered_packages.iter().any(|e| e.is_root_pkg));

        Ok(filtered_packages.into())
    })
}
