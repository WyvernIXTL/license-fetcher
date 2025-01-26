//               Copyright Adam McKellar 2024
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use std::env::{var_os, var};
use std::fs::write;
use std::process::Command;
use std::collections::BTreeSet;
use std::time::Instant;
use std::path::PathBuf;

#[cfg(feature = "compress")]
use miniz_oxide::deflate::compress_to_vec;

use serde_json::from_slice;
use log::info;
use simplelog::{TermLogger, LevelFilter, Config, TerminalMode, ColorChoice};

mod metadata;
mod cargo_source;

use crate::*;
use build_script::metadata::*;
use cargo_source::{licenses_text_from_cargo_src_folder, license_text_from_folder};


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
    let cargo_path = var_os("CARGO").unwrap();
    let manifest_path = var_os("CARGO_MANIFEST_DIR").unwrap();

    let mut metadata_output = Command::new(&cargo_path)
                                        .current_dir(&manifest_path)
                                        .args(["metadata", "--format-version", "1", "--frozen", "--color", "never"])
                                        .output()
                                        .unwrap();

    #[cfg(not(feature = "frozen"))]
    if !metadata_output.status.success() {
        metadata_output = Command::new(&cargo_path)
                            .current_dir(&manifest_path)
                            .args(["metadata", "--format-version", "1", "--color", "never"])
                            .output()
                            .unwrap();
    }

    if !metadata_output.status.success() {
        panic!("Failed executing cargo metadata with:\n{}", String::from_utf8_lossy(&metadata_output.stderr));
    }

    let metadata_parsed: Metadata = from_slice(&metadata_output.stdout).unwrap();

    let packages = metadata_parsed.packages;
    let package_id = metadata_parsed.resolve.root.unwrap();
    let dependencies = metadata_parsed.resolve.nodes;

    let mut used_packages = BTreeSet::new();

    walk_dependencies(&mut used_packages, &dependencies, &package_id);


    // Add dependencies:

    let mut package_list = vec![];

    for package in packages {
        if used_packages.contains(&package.id) {
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


/// Generates a package list with package name, authors and license text.
/// 
/// This function:
/// 1. Calls `cargo tree -e normal --frozen`. *(After error tries again online if not `frozen` feature is set.)*
/// 2. Calls `cargo metadata --frozen`. *(After error tries again online if not `frozen` feature is set.)*
/// 3. Takes the packages gotten from `cargo tree` with the metadata of `cargo metadata`.
/// 4. Fetches the licenses from github with the `repository` link if it includes `github` in name.
/// 5. Serializes, copmresses and writes said package list to `OUT_DIR/LICENSE-3RD-PARTY.bincode` file.
/// 
/// Needs the feature `build` and is only meant to be used in build scripts.
/// 
/// # Example
/// In `build.rs`:
/// ```no_run
/// use license_fetcher::build_script::generate_package_list_with_licenses;
///
/// fn main() {
///     generate_package_list_with_licenses().write();
///     println!("cargo::rerun-if-changed=build.rs");
///     println!("cargo::rerun-if-changed=Cargo.lock");
///     println!("cargo::rerun-if-changed=Cargo.toml");
/// }
/// ```
pub fn generate_package_list_with_licenses() -> PackageList {
    TermLogger::init(LevelFilter::Trace, Config::default(), TerminalMode::Stderr, ColorChoice::Auto).unwrap();

    let mut package_list = generate_package_list();

    licenses_text_from_cargo_src_folder(&mut package_list);

    let this_package_name = var("CARGO_PKG_NAME").unwrap();
    info!("Fetching license for: {}", &this_package_name);
    let this_package_path = var("CARGO_MANIFEST_DIR").unwrap();
    let this_package_index = package_list.iter().enumerate().filter(|(_, p)| p.name == this_package_name).map(|(i, _)| i).next().unwrap();
    package_list[this_package_index].license_text = license_text_from_folder(&PathBuf::from(this_package_path));
    package_list.swap(this_package_index, 0);

    package_list
}

impl PackageList {
    /// Writes the [PackageList] to the file and folder where they can be embedded into the program at compile time.
    /// 
    /// Copmresses and writes the PackageList into the `OUT_DIR` with file name `LICENSE-3RD-PARTY.bincode`.
    pub fn write(self) {
        let mut path = var_os("OUT_DIR").unwrap();
        path.push("/LICENSE-3RD-PARTY.bincode");
    
        let data = bincode::encode_to_vec(self, config::standard()).unwrap();
    
        info!("License data size: {} Bytes", data.len());
        let instant_before_compression = Instant::now();
    
        #[cfg(feature = "compress")]
        let compressed_data = compress_to_vec(&data, 10);
    
        #[cfg(not(feature = "compress"))]
        let compressed_data = data;
    
        info!("Compressed data size: {} Bytes in {}ms", compressed_data.len(), instant_before_compression.elapsed().as_millis());
    
        info!("Writing to file: {:?}", &path);
        write(path, compressed_data).unwrap();
    }
}
