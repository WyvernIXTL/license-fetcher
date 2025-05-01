// Copyright Adam McKellar 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::Path;

use command::exec_cargo;
use error_stack::{Result, ResultExt};
use fnv::{FnvHashMap, FnvHashSet};
use metadata::{Metadata, MetadataResolveNode};
use serde_json::from_slice;
use thiserror::Error;

use crate::{Package, PackageList};

use super::config::CargoDirectiveList;

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

fn package_list_from_cargo_metadata<P>(
    cargo: P,
    cargo_directives: &CargoDirectiveList,
    manifest_dir: P,
) -> Result<PackageList, PkgListFromCargoMetadataError>
where
    P: AsRef<Path>,
{
    const ARGUMENTS: &'static [&'static str] =
        &["metadata", "--format-version", "1", "--color", "never"];

    let output = exec_cargo(cargo, cargo_directives, manifest_dir, ARGUMENTS)
        .change_context(PkgListFromCargoMetadataError::ExecCargo)?;

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
        .map(|package| Package {
            license_text: None,
            authors: package.authors,
            license_identifier: package.license,
            name: package.name,
            version: package.version,
            description: package.description,
            homepage: package.homepage,
            repository: package.repository,
        })
        .collect::<Vec<Package>>()
        .into())
}

fn used_pkg_names_from_cargo_tree<P>(
    cargo: P,
    cargo_directives: &CargoDirectiveList,
    manifest_dir: P,
) -> Result<FnvHashSet<String>, PkgListFromCargoMetadataError>
where
    P: AsRef<Path>,
{
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

    let output = exec_cargo(cargo, cargo_directives, manifest_dir, ARGUMENTS)
        .change_context(PkgListFromCargoMetadataError::ExecCargo)?;

    Ok(String::from_utf8(output.stdout)
        .change_context(PkgListFromCargoMetadataError::ParseString)?
        .lines()
        .map(|l| l.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .collect::<FnvHashSet<String>>())
}
