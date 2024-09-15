//               Copyright Adam McKellar 2024
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use std::fs::write;
use std::process::Command;
use std::collections::BTreeSet;

#[cfg(feature = "compress")]
use lz4_flex::block::compress_prepend_size;

use serde_json::from_slice;


mod metadata;

use crate::*;
use crate::build_script::metadata::*;


fn write_package_list(package_list: PackageList) {
    let mut path = env::var_os("OUT_DIR").unwrap();
    path.push("/LICENSE-3RD-PARTY.bincode");

    let data = bincode::encode_to_vec(package_list, config::standard()).unwrap();

    #[cfg(feature = "compress")]
    let compressed_data = compress_prepend_size(&data);

    #[cfg(not(feature = "compress"))]
    let compressed_data = data;

    write(path, compressed_data).unwrap();
}

fn walk_dependencies<'a>(used_dependencies: &mut BTreeSet<&'a String>, dependencies: &'a Vec<MetadataResolveNode>, root: &String) {
    let package = match dependencies.iter().find(|&dep| dep.id == *root) {
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

fn generate_package_list() -> PackageList {
    let cargo_path = env::var_os("CARGO").unwrap();
    let manifest_path = env::var_os("CARGO_MANIFEST_DIR").unwrap();

    // Workaround: Get dependencies with `cargo tree`.
    // These are dependencies, which are compiled.
    // <https://github.com/rust-lang/cargo/issues/11444>

    let mut used_packages_tree: Option<BTreeSet<String>> = None;

    let output = Command::new(&cargo_path)
                            .current_dir(&manifest_path)
                            .args(["tree", "-e", "normal", "-f", "{p}", "--prefix", "none", "--frozen", "--color", "never", "--no-dedupe"])
                            .output();

    if let Ok(outp) = output {
        let tree_string = String::from_utf8(outp.stdout).unwrap();
        let mut used_package_set = BTreeSet::new();

        for package in tree_string.lines() {
            let mut split_line_iter = package.split_whitespace();
            if let Some(s) = split_line_iter.next() {
                used_package_set.insert(s.to_owned());
            }
        }

        used_packages_tree = Some(used_package_set);
    }


    // Walk dependencies.
    // This also finds packages which are not compiled.
    // See: <https://github.com/rust-lang/cargo/issues/10801>

    let metadata_output = Command::new(cargo_path)
                                        .current_dir(manifest_path)
                                        .args(["metadata", "--format-version", "1", "--color", "never", "--frozen"])
                                        .output()
                                        .unwrap();
    let metadata_parsed: Metadata = from_slice(&metadata_output.stdout).unwrap();

    let packages = metadata_parsed.packages;
    let package_id = metadata_parsed.resolve.root.unwrap();
    let dependencies = metadata_parsed.resolve.nodes;

    let mut used_packages = BTreeSet::new();

    walk_dependencies(&mut used_packages, &dependencies, &package_id);


    // Add dependencies:

    let mut package_list = vec![];

    for package in packages {
        let mut is_used = true;

        if let Some(tree_packages) = &used_packages_tree {
            is_used = tree_packages.contains(&package.name);
        }

        if is_used && used_packages.contains(&package.id) {
            package_list.push(Package {
                license_text: None,
                authors: package.authors,
                license_identifier: package.license,
                name: package.name,
                version: package.version,
                description: package.description,
                homepage: package.homepage,
                repository: package.repository,
            });
        }

    }

    PackageList(package_list)
}


pub fn generate_package_list_with_licenses() {
    let package_list = generate_package_list();

    //TODO get licenses

    write_package_list(package_list);
}

