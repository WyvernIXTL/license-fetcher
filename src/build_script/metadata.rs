//               Copyright Adam McKellar 2024
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)


use std::cmp;

use serde::Deserialize;

// Compatible json decode of `cargo metadata --format-version 1`
// https://doc.rust-lang.org/cargo/commands/cargo-metadata.html

#[derive(Deserialize, Debug)]
pub(super) struct MetadataPackage {
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

#[derive(Deserialize, Debug, cmp::PartialEq, cmp::Eq, cmp::PartialOrd, cmp::Ord)]
pub(super) struct MetadataResolveNodeDepsKind {
    pub kind: Option<String>,
}

#[derive(Deserialize, Debug, cmp::PartialEq, cmp::PartialOrd, cmp::Eq, cmp::Ord)]
pub(super) struct MetadataResolveNodeDeps {
    pub pkg: String,
    pub dep_kinds: Vec<MetadataResolveNodeDepsKind>,
}

#[derive(Deserialize, Debug, cmp::PartialEq, cmp::PartialOrd, cmp::Eq, cmp::Ord)]
pub(super) struct MetadataResolveNode {
    pub id: String,
    pub deps: Vec<MetadataResolveNodeDeps>,
}

#[derive(Deserialize, Debug)]
pub(super) struct MetadataResolve {
    pub nodes: Vec<MetadataResolveNode>,
    pub root: Option<String>
}

#[derive(Deserialize, Debug)]
pub(super) struct Metadata {
    pub packages: Vec<MetadataPackage>,
    pub resolve: MetadataResolve
}


#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::from_slice;
    use std::fs::read;
    use std::ffi::OsString;
    use std::env;

    fn get_path() -> OsString {
        env::var_os("CARGO_MANIFEST_DIR").unwrap()
    }

    #[test]
    fn test_parse_metadata_json() {
        let mut root = get_path();
        root.push("/tests/metadata_test.json");
        let json_bytes = read(root).unwrap();
        let _parsed_metadata: Metadata = from_slice(&json_bytes).unwrap();
    }
}
