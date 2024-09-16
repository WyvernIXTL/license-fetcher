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
use tokio::runtime::{Runtime, Builder};
use tokio::task::JoinSet;
use octocrab::instance;


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

    let mut output = Command::new(&cargo_path)
                            .current_dir(&manifest_path)
                            .args(["tree", "-e", "normal", "-f", "{p}", "--prefix", "none", "--frozen", "--color", "never", "--no-dedupe"])
                            .output()
                            .unwrap();
    
    #[cfg(not(feature = "frozen"))]
    if !output.status.success() {
        output = Command::new(&cargo_path)
                            .current_dir(&manifest_path)
                            .args(["tree", "-e", "normal", "-f", "{p}", "--prefix", "none", "--color", "never", "--no-dedupe"])
                            .output()
                            .unwrap();
    }

    #[cfg(feature = "frozen")]
    if !output.status.success() {
        panic!("Failed executing cargo tree with:\n{}", String::from_utf8_lossy(&output.stderr));
    }

    if output.status.success() {
        let tree_string = String::from_utf8(output.stdout).unwrap();
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

async fn get_license_text_from_github(url: &String) -> Option<String> {
    let split_url: Vec<&str> = url.split("/").collect();
    let length = split_url.len();

    if length < 3 {
        return None;        
    }

    let owner = split_url[length-2];
    let mut repo_str = split_url[length-1];
    if let Some(repo_str_stripped) = repo_str.strip_suffix(".git") {
        repo_str = repo_str_stripped;
    }
    let octo = instance();
    let repo = octo.repos(owner, repo_str);

    if let Ok(content) = repo.license().await {
        content.decoded_content()
    } else {
        None
    }
}

async fn get_license_text_from_git_for_package_list(package_list: PackageList) -> PackageList {
    let mut set = JoinSet::new();

    for package in package_list.0.into_iter() {
        if let Some(repo_url) = &package.repository {
            if repo_url.contains("github") {
                set.spawn(async move {
                    let mut pack = package;
                    pack.license_text = get_license_text_from_github(pack.repository.as_ref().unwrap()).await;
                    pack
                });
            }
        }
    }

    let mut packages_with_license = PackageList(vec![]);

    while let Some(pack_res) = set.join_next().await {
        if let Ok(pack) = pack_res {
            packages_with_license.0.push(pack)
        }
    }

    packages_with_license
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
///     generate_package_list_with_licenses();
///     println!("cargo::rerun-if-changed=build.rs");
///     println!("cargo::rerun-if-changed=Cargo.lock");
/// }
/// ```
pub fn generate_package_list_with_licenses() {
    let mut package_list = generate_package_list();

    let rt: Runtime  = Builder::new_current_thread().enable_all().build().unwrap();
    package_list = rt.block_on(async move {
        get_license_text_from_git_for_package_list(package_list).await
    });

    write_package_list(package_list);
}

