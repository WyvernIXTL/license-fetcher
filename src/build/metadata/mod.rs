// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{error::Error, thread::scope};

use error_stack::{report, Result};

use crate::{
    build::metadata::{
        exec_metadata::exec_cargo_metadata_and_parse_result,
        exec_tree::exec_cargo_tree_and_parse_output,
    },
    Package,
};

use super::config::MetadataConfig;

mod exec;
mod exec_metadata;
mod exec_tree;
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
    config: &MetadataConfig,
) -> Result<(String, impl Iterator<Item = Package> + '_), PkgListFromCargoMetadataError> {
    scope(|scope| {
        let cargo_metadata_thread_handle =
            scope.spawn(|| exec_cargo_metadata_and_parse_result(config));

        let cargo_tree_thread_handle = scope.spawn(|| exec_cargo_tree_and_parse_output(config));

        let cargo_metadata_thread_result = cargo_metadata_thread_handle.join().map_err(|e| {
            report!(PkgListFromCargoMetadataError::Thread).attach_printable(format!("{e:?}"))
        })?;
        let cargo_tree_thread_result = cargo_tree_thread_handle.join().map_err(|e| {
            report!(PkgListFromCargoMetadataError::Thread).attach_printable(format!("{e:?}"))
        })?;

        match (cargo_metadata_thread_result, cargo_tree_thread_result) {
            (Err(mut cargo_metadata_err), Err(cargo_tree_err)) => {
                cargo_metadata_err.extend_one(cargo_tree_err);
                Err(cargo_metadata_err)
            }
            (Err(cargo_metadata_err), _) => Err(cargo_metadata_err),
            (_, Err(cargo_tree_err)) => Err(cargo_tree_err),
            (Ok((root_package_name, packages_iter)), Ok(used_package_names)) => Ok((
                root_package_name,
                packages_iter.filter(move |p| used_package_names.contains(&p.name)),
            )),
        }
    })
}
