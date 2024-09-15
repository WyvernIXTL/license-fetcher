//               Copyright Adam McKellar 2024
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use serde::Deserialize;

// Compatible json decode of `cargo metadata --format-version 1`
// https://doc.rust-lang.org/cargo/commands/cargo-metadata.html

#[derive(Deserialize, Debug)]
pub(super) struct MetadataDependencies {
    name: String,
    kind: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(super) struct MetadataPackage {
    name: String,
    version: String,
    id: String,
    license: Option<String>,
    license_file: Option<String>,
    description: Option<String>,
    dependencies: Vec<MetadataDependencies>,
    authors: Vec<String>,
    repository: Option<String>,
    homepage: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(super) struct MetadataResolveNodeDepsKind {
    kind: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(super) struct MetadataResolveNodeDeps {
    pkg: String,
    dep_kinds: Vec<MetadataResolveNodeDepsKind>,
}

#[derive(Deserialize, Debug)]
pub(super) struct MetadataResolveNode {
    id: String,
    deps: Vec<MetadataResolveNodeDeps>,
}

#[derive(Deserialize, Debug)]
pub(super) struct MetadataResolve {
    nodes: Vec<MetadataResolveNode>,
    root: Option<String>
}

#[derive(Deserialize, Debug)]
pub(super) struct Metadata {
    packages: Vec<MetadataPackage>,
    resolve: MetadataResolve
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
