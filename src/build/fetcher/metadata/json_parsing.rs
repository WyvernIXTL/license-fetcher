// Copyright Adam McKellar 2024, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// The `DeJson` derive gives a clippy warning when the pedantic group is enabled.
#![allow(clippy::question_mark)]

use std::cmp;

use nanoserde::DeJson;

// Compatible json decode of `cargo metadata --format-version 1`
// https://doc.rust-lang.org/cargo/commands/cargo-metadata.html

#[derive(DeJson, Debug)]
pub struct MetadataPackage {
    pub name: String,
    pub version: String,
    pub id: String,
    pub license: Option<String>,
    // pub license_file: Option<String>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
}

#[derive(DeJson, Debug, cmp::PartialEq, cmp::Eq, cmp::PartialOrd, cmp::Ord)]
pub struct MetadataResolveNodeDepsKind {
    pub kind: Option<String>,
}

#[derive(DeJson, Debug, cmp::PartialEq, cmp::PartialOrd, cmp::Eq, cmp::Ord)]
pub struct MetadataResolveNodeDeps {
    pub pkg: String,
    pub dep_kinds: Vec<MetadataResolveNodeDepsKind>,
}

#[derive(DeJson, Debug, cmp::PartialEq, cmp::PartialOrd, cmp::Eq, cmp::Ord)]
pub struct MetadataResolveNode {
    pub id: String,
    pub deps: Vec<MetadataResolveNodeDeps>,
}

#[derive(DeJson, Debug)]
pub struct MetadataResolve {
    pub nodes: Vec<MetadataResolveNode>,
    pub root: Option<String>,
}

#[derive(DeJson, Debug)]
pub struct Metadata {
    pub packages: Vec<MetadataPackage>,
    pub resolve: MetadataResolve,
}

/* -------------------------------------------------------------------------- */
/*                                 Unit Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    use std::env;
    use std::ffi::OsString;
    use std::fs::read_to_string;

    fn get_path() -> OsString {
        env::var_os("CARGO_MANIFEST_DIR").unwrap()
    }

    #[test]
    fn test_parse_metadata_json() {
        let mut root = get_path();
        root.push("/tests/metadata_test.json");
        let json_string = read_to_string(root).unwrap();
        let _parsed_metadata: Metadata = Metadata::deserialize_json(&json_string).unwrap();
    }
}
