//               Copyright Adam McKellar 2024, 2025
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use std::collections::BTreeSet;
use std::env::{var, var_os};
use std::ffi::OsString;
use std::fs::write;
use std::path::PathBuf;
use std::time::Instant;

#[cfg(feature = "compress")]
use miniz_oxide::deflate::compress_to_vec;

use log::info;
use serde_json::from_slice;
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};
use smol::process::Command;
use smol::{block_on, LocalExecutor};

mod cargo_source;
mod metadata;

use crate::*;
use build_script::metadata::*;
use cargo_source::{license_text_from_folder, licenses_text_from_cargo_src_folder};

fn walk_dependencies<'a>(
    used_dependencies: &mut BTreeSet<&'a String>,
    dependencies: &'a Vec<MetadataResolveNode>,
    root: &String,
) {
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

async fn generate_package_list(
    cargo_path: Option<OsString>,
    manifest_dir_path: OsString,
) -> PackageList {
    let cargo_path = cargo_path.unwrap_or_else(|| OsString::from("cargo"));

    let mut metadata_output = Command::new(&cargo_path)
        .current_dir(&manifest_dir_path)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--frozen",
            "--color",
            "never",
        ])
        .output()
        .await
        .unwrap();

    #[cfg(not(feature = "frozen"))]
    if !metadata_output.status.success() {
        metadata_output = Command::new(&cargo_path)
            .current_dir(&manifest_dir_path)
            .args(["metadata", "--format-version", "1", "--color", "never"])
            .output()
            .await
            .unwrap();
    }

    assert!(
        metadata_output.status.success(),
        "Failed executing cargo metadata with:\n{}",
        String::from_utf8_lossy(&metadata_output.stderr)
    );

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

/// Runs the cargo tree command and returns it output or None if an error occurred.
async fn execute_cargo_tree(
    cargo_path: Option<OsString>,
    manifest_dir_path: OsString,
) -> Option<String> {
    let cargo_path = cargo_path.unwrap_or_else(|| OsString::from("cargo"));

    let mut output = Command::new(&cargo_path)
        .current_dir(&manifest_dir_path)
        .args([
            "tree",
            "-e",
            "normal",
            "-f",
            "{p}",
            "--prefix",
            "none",
            "--frozen",
            "--color",
            "never",
            "--no-dedupe",
        ])
        .output()
        .await
        .unwrap();

    #[cfg(not(feature = "frozen"))]
    if !output.status.success() {
        output = Command::new(&cargo_path)
            .current_dir(&manifest_dir_path)
            .args([
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
            ])
            .output()
            .await
            .unwrap();
    }

    if !output.status.success() {
        log::error!(
            "Failed executing cargo tree with:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        return None;
    }

    Some(String::from_utf8(output.stdout).unwrap())
}

/// Filters [PackageList] with output of `cargo tree`.
///
/// Workaround for `cargo metadata`'s inability to differentiate between dependencies
/// of packages that are used in build scripts and normally.
fn filter_package_list_with_cargo_tree(
    package_list: PackageList,
    cargo_tree_output: String,
) -> PackageList {
    let mut used_package_set = BTreeSet::new();

    for package in cargo_tree_output.lines() {
        let mut split_line_iter = package.split_whitespace();
        if let Some(s) = split_line_iter.next() {
            used_package_set.insert(s.to_owned());
        }
    }

    let mut filtered_package_list = PackageList(vec![]);

    for pkg in package_list.iter() {
        if used_package_set.contains(&pkg.name) {
            filtered_package_list.push(pkg.clone());
        }
    }

    filtered_package_list
}

/// Generates a package list with package name, authors and license text. Uses supplied parameters for cargo path and manifest path.
///
/// This function is not as useful as [generate_package_list_with_licenses()] for build scripts.
/// [generate_package_list_with_licenses()] fetches `cargo_path` and `manifest_dir_path` automatically.
/// This function does not.
/// The main use is for other rust programs to fetch the metadata outside of a build script.
///
/// ### Arguments
///
/// * **cargo_path - Absolute path to cargo executable. If omitted tries to fetch the path from `PATH`.
/// * **manifest_dir_path** - Relative or absolute path to manifest dir.
/// * **this_package_name** - Name of the package. `cargo metadata` does not disclose the name, but it is needed for parsing the used licenses.
pub async fn generate_package_list_with_licenses_without_env_calls(
    ex: &LocalExecutor<'_>,
    cargo_path: Option<OsString>,
    manifest_dir_path: OsString,
    this_package_name: String,
) -> PackageList {
    let package_list_task = ex.spawn(generate_package_list(
        cargo_path.clone(),
        manifest_dir_path.clone(),
    ));
    let cargo_tree_options_task =
        ex.spawn(execute_cargo_tree(cargo_path, manifest_dir_path.clone()));

    let mut package_list = package_list_task.await;
    if let Some(cargo_tree_output) = cargo_tree_options_task.await {
        package_list = filter_package_list_with_cargo_tree(package_list, cargo_tree_output);
    }

    licenses_text_from_cargo_src_folder(&mut package_list);

    info!("Fetching license for: {}", &this_package_name);
    let this_package_index = package_list
        .iter()
        .enumerate()
        .filter(|(_, p)| p.name == this_package_name)
        .map(|(i, _)| i)
        .next()
        .unwrap();
    package_list[this_package_index].license_text =
        license_text_from_folder(&PathBuf::from(manifest_dir_path));
    package_list.swap(this_package_index, 0);

    package_list
}

/// Generates a package list with package name, authors and license text. Uses env variables supplied by cargo during build.
///
/// This function:
/// 1. Calls `cargo tree -e normal --frozen`. *(After error tries again online if not `frozen` feature is set.)*
/// 2. Calls `cargo metadata --frozen`. *(After error tries again online if not `frozen` feature is set.)*
/// 3. Takes the packages gotten from `cargo tree` with the metadata of `cargo metadata`.
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
    TermLogger::init(
        LevelFilter::Trace,
        Config::default(),
        TerminalMode::Stderr,
        ColorChoice::Auto,
    )
    .unwrap();

    let cargo_path = var_os("CARGO").unwrap();
    let manifest_dir_path = var_os("CARGO_MANIFEST_DIR").unwrap();
    let this_package_name = var("CARGO_PKG_NAME").unwrap();

    let ex = LocalExecutor::new();

    block_on(
        ex.run(generate_package_list_with_licenses_without_env_calls(
            &ex,
            Some(cargo_path),
            manifest_dir_path,
            this_package_name,
        )),
    )
}

impl PackageList {
    /// Writes the [PackageList] to the file and folder where they can be embedded into the program at compile time.
    ///
    /// Compresses and writes the PackageList into the `OUT_DIR` with file name `LICENSE-3RD-PARTY.bincode`.
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

        info!(
            "Compressed data size: {} Bytes in {}ms",
            compressed_data.len(),
            instant_before_compression.elapsed().as_millis()
        );

        info!("Writing to file: {:?}", &path);
        write(path, compressed_data).unwrap();
    }
}
