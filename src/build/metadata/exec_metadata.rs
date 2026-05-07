// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use error_stack::{report, Result, ResultExt};
use nanoserde::DeJson;
use regex_lite::Regex;

use crate::{
    build::{
        config::MetadataConfig,
        metadata::{
            exec::exec_cargo,
            json_parsing::{Metadata, MetadataResolveNode},
            PkgListFromCargoMetadataError,
        },
    },
    Package,
};

fn exec_cargo_metadata(config: &MetadataConfig) -> Result<Metadata, PkgListFromCargoMetadataError> {
    const ARGUMENTS: &[&str] = &["metadata", "--format-version", "1", "--color", "never"];

    let output =
        exec_cargo(config, &ARGUMENTS).change_context(PkgListFromCargoMetadataError::ExecCargo)?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    Metadata::deserialize_json(&output_str).change_context(PkgListFromCargoMetadataError::ParseJson)
}

fn walk_metadata_resolve_nodes<'a>(
    used_dependencies: &mut HashSet<String>,
    dependencies: &'a HashMap<&String, &MetadataResolveNode>,
    root: &String,
) {
    let Some(package) = dependencies.get(root) else {
        return;
    };

    used_dependencies.insert(package.id.clone());
    for dep in &package.deps {
        if dep
            .dep_kinds
            .iter()
            .map(|d| &d.kind)
            .any(std::option::Option::is_none)
        {
            walk_metadata_resolve_nodes(used_dependencies, dependencies, &dep.pkg);
        }
    }
}

fn used_package_names_from_metadata_resolve_nodes(
    deps: Vec<MetadataResolveNode>,
    root_package_id: String,
) -> HashSet<String> {
    let mut used_packages = HashSet::default();
    let dependencies_hash_map = deps.iter().map(|d| (&d.id, d)).collect::<HashMap<_, _>>();

    walk_metadata_resolve_nodes(&mut used_packages, &dependencies_hash_map, &root_package_id);

    used_packages
}

fn parse_package_name_from_package_id(
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

fn parse_metadata(
    metadata_parsed: Metadata,
) -> Result<(String, impl Iterator<Item = Package>), PkgListFromCargoMetadataError> {
    let packages = metadata_parsed.packages;
    let root_package_id = metadata_parsed
        .resolve
        .root
        .ok_or(PkgListFromCargoMetadataError::ParseJson)
        .attach_printable("Failed to resolve root package id from output.")?;
    let root_package_name = parse_package_name_from_package_id(&root_package_id)?;
    let dependencies = metadata_parsed.resolve.nodes;

    let used_packages =
        used_package_names_from_metadata_resolve_nodes(dependencies, root_package_id);

    let res_iter = packages
        .into_iter()
        .filter(move |metadata_package| used_packages.contains(&metadata_package.id))
        .map(|metadata_package| Package {
            license_text: None,
            authors: metadata_package.authors,
            license_identifier: metadata_package.license,
            name: metadata_package.name,
            version: metadata_package.version,
            description: metadata_package.description,
            homepage: metadata_package.homepage,
            repository: metadata_package.repository,
        });

    Ok((root_package_name, res_iter))
}

pub fn exec_cargo_metadata_and_parse_result(
    config: &MetadataConfig,
) -> Result<(String, impl Iterator<Item = Package> + '_), PkgListFromCargoMetadataError> {
    let metadata = exec_cargo_metadata(config)?;
    parse_metadata(metadata)
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
        let pkg_name = parse_package_name_from_package_id(&pkg_id).unwrap();
        assert_eq!(pkg_name, "license-fetcher");
    }

    // TODO: add tests for parsing here
}
