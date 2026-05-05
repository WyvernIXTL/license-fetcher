// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    collections::{HashMap, HashSet},
    error::Error,
    sync::LazyLock,
    thread::scope,
};

use command::exec_cargo;
use error_stack::{ensure, report, Result, ResultExt};
use json_parsing::{Metadata, MetadataResolveNode};
use nanoserde::DeJson;
use regex_lite::Regex;

use crate::{Package, PackageList};

use super::config::MetadataConfig;

mod command;
mod json_parsing;

/// Error handling the execution and parsing of package metadata.
#[derive(Debug, Clone, Copy, displaydoc::Display)]
pub enum PkgListFromCargoMetadataError {
    /// failed to execute `cargo metadata` or `cargo tree`
    ExecCargo,
    /// failed to parse the output of `cargo metadata`
    ParseJson,
    /// failed to parse the output of `cargo tree` as it is not valid UTF-8
    ParseString,
    /// a thread executing `cargo metadata` or `cargo tree` panicked
    Thread,
    /// failed to parse a package name from a package id
    PackageNameParseError,
    /// the root package is not part of the filtered package metadata
    RootPackageMissing,
}

impl Error for PkgListFromCargoMetadataError {}

fn walk_dependencies<'a>(
    used_dependencies: &mut HashSet<&'a String>,
    dependencies: &'a HashMap<&String, &MetadataResolveNode>,
    root: &String,
) {
    let Some(package) = dependencies.get(root) else {
        return;
    };

    used_dependencies.insert(&package.id);
    for dep in &package.deps {
        if dep
            .dep_kinds
            .iter()
            .map(|d| &d.kind)
            .any(std::option::Option::is_none)
        {
            walk_dependencies(used_dependencies, dependencies, &dep.pkg);
        }
    }
}

fn extract_package_name_from_id(
    package_id: &String,
) -> Result<String, PkgListFromCargoMetadataError> {
    static PARSE_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r".*?[#|\/](?<name>[a-z\-\_\d]+)[@|#][\d\.]+").unwrap());

    if let Some(caps) = PARSE_REGEX.captures(package_id) {
        Ok(caps["name"].to_owned())
    } else {
        Err(
            report!(PkgListFromCargoMetadataError::PackageNameParseError)
                .attach_printable(format!("package id: '{package_id}'")),
        )
    }
}

fn package_list_from_cargo_metadata(
    config: &MetadataConfig,
) -> Result<Vec<Package>, PkgListFromCargoMetadataError> {
    const ARGUMENTS: &[&str] = &["metadata", "--format-version", "1", "--color", "never"];

    let output =
        exec_cargo(config, &ARGUMENTS).change_context(PkgListFromCargoMetadataError::ExecCargo)?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    let metadata_parsed = Metadata::deserialize_json(&output_str)
        .change_context(PkgListFromCargoMetadataError::ParseJson)?;

    let packages = metadata_parsed.packages;
    let package_id = metadata_parsed
        .resolve
        .root
        .ok_or(PkgListFromCargoMetadataError::ParseJson)
        .attach_printable("Failed to resolve package id from output.")?;
    let dependencies = metadata_parsed.resolve.nodes;

    let mut used_packages = HashSet::default();
    let dependencies_hash_map = dependencies
        .iter()
        .map(|d| (&d.id, d))
        .collect::<HashMap<_, _>>();

    walk_dependencies(&mut used_packages, &dependencies_hash_map, &package_id);

    let root_package_name = extract_package_name_from_id(&package_id)?;

    Ok(packages
        .into_iter()
        .filter(|e| used_packages.contains(&e.id))
        .map(|package| {
            let is_root = package.name.as_ref() == root_package_name;
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
) -> Result<HashSet<String>, PkgListFromCargoMetadataError> {
    const ARGUMENTS: &[&str] = &[
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
        exec_cargo(config, &ARGUMENTS).change_context(PkgListFromCargoMetadataError::ExecCargo)?;

    Ok(String::from_utf8(output.stdout)
        .change_context(PkgListFromCargoMetadataError::ParseString)?
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|e| e.split(' ').next().unwrap_or(e))
        .map(std::borrow::ToOwned::to_owned)
        .collect::<HashSet<String>>())
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
pub fn package_list(config: &MetadataConfig) -> Result<PackageList, PkgListFromCargoMetadataError> {
    scope(|scope| {
        let packages_handle = scope.spawn(|| package_list_from_cargo_metadata(config));

        let used_package_names_handle = scope.spawn(|| used_pkg_names_from_cargo_tree(config));

        let packages = packages_handle.join().map_err(|e| {
            report!(PkgListFromCargoMetadataError::Thread).attach_printable(format!("{e:?}"))
        })?;
        let used_packages = used_package_names_handle.join().map_err(|e| {
            report!(PkgListFromCargoMetadataError::Thread).attach_printable(format!("{e:?}"))
        })?;

        match (packages, used_packages) {
            (Err(mut pkgs_err), Err(used_pkgs_err)) => {
                pkgs_err.extend_one(used_pkgs_err);
                Err(pkgs_err)
            }
            (Err(pkgs_err), _) => Err(pkgs_err),
            (_, Err(used_pkgs_err)) => Err(used_pkgs_err),
            (Ok(packages), Ok(used_package_names)) => {
                let mut filtered_packages = Vec::with_capacity(packages.capacity());
                let filtered_packages_iter = packages
                    .into_iter()
                    .filter(|e| used_package_names.contains(&e.name));
                filtered_packages.extend(filtered_packages_iter);

                ensure!(
                    filtered_packages.iter().any(|e| e.is_root_pkg),
                    PkgListFromCargoMetadataError::RootPackageMissing
                );

                Ok(filtered_packages.into())
            }
        }
    })
}

/* -------------------------------------------------------------------------- */
/*                                 Unit Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod test {
    use super::*;

    #[test]
    fn test_extract_package_name_from_id() {
        let pkg_id = "path+file:///M:/DEV/Projects/Rust/projects/license-fetcher#0.7.3".to_owned();
        let pkg_name = extract_package_name_from_id(&pkg_id).unwrap();
        assert_eq!(pkg_name, "license-fetcher");
    }
}
