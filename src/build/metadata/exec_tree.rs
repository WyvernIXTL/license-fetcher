// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{collections::HashSet, process::Output};

use error_stack::{Result, ResultExt};

use crate::build::{
    config::MetadataConfig,
    metadata::{exec::exec_cargo, PkgListFromCargoMetadataError},
};

fn exec_cargo_tree(config: &MetadataConfig) -> Result<Output, PkgListFromCargoMetadataError> {
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

    exec_cargo(config, &ARGUMENTS).change_context(PkgListFromCargoMetadataError::ExecCargo)
}

fn parse_cargo_tree_output(
    output: Output,
) -> Result<HashSet<String>, PkgListFromCargoMetadataError> {
    Ok(String::from_utf8(output.stdout)
        .change_context(PkgListFromCargoMetadataError::ParseString)?
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|e| e.split(' ').next().unwrap_or(e))
        .map(std::borrow::ToOwned::to_owned)
        .collect::<HashSet<String>>())
}

pub fn exec_cargo_tree_and_parse_output(
    config: &MetadataConfig,
) -> Result<HashSet<String>, PkgListFromCargoMetadataError> {
    let output = exec_cargo_tree(config)?;
    parse_cargo_tree_output(output)
}

/* -------------------------------------------------------------------------- */
/*                                 Unit Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod test {
    // TODO: add tests for parsing here
}
