//               Copyright Adam McKellar 2024, 2025
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use std::collections::HashMap;
use std::env::var_os;
use std::fs::{read_dir, read_to_string};
use std::path::PathBuf;
use std::sync::LazyLock;

use directories::BaseDirs;
use log::{info, trace, warn};
use regex::Regex;

use crate::{Package, PackageList};

fn cargo_folder() -> PathBuf {
    if let Some(path) = var_os("CARGO_HOME") {
        path.into()
    } else {
        let base_dir = BaseDirs::new().expect("Failed to find home dir.");
        let home_dir = base_dir.home_dir();
        let mut cargo_dir = home_dir.to_path_buf();
        cargo_dir.push(".cargo");
        if !cargo_dir.exists() {
            panic!(
                "Failed finding cargo dir: {:#?}. Set it manually with CARGO_HOME variable.",
                &cargo_dir
            );
        }
        cargo_dir
    }
}

fn src_registry_folders(path: PathBuf) -> Vec<PathBuf> {
    let src_subfolder = PathBuf::from("registry/src");
    let src_dir = path.join(src_subfolder);
    read_dir(src_dir)
        .expect("Src path is not a dir.")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map_or(false, |ft| ft.is_dir()))
        .map(|e| e.path())
        .collect()
}

pub(super) fn license_text_from_folder(path: &PathBuf) -> Option<String> {
    trace!("Fetching license in folder: {:?}", &path);

    static LICENSE_FILE_NAME_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i).*(license|copying|authors|notice|eula).*").unwrap());

    let license_text_vec: Vec<String> = read_dir(&path)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| LICENSE_FILE_NAME_REGEX.is_match(&e.file_name().to_string_lossy()))
        .filter(|e| e.file_type().map_or(false, |e| e.is_file()))
        .filter_map(|e| read_to_string(e.path()).ok())
        .collect();

    if license_text_vec.is_empty() {
        warn!("Found no licenses in folder: {:?}", &path);
        return None;
    }

    Some(license_text_vec.join("\n\n"))
}

pub(super) fn licenses_text_from_cargo_src_folder(package_list: &PackageList) -> PackageList {
    let mut package_hash_map = HashMap::new();
    for p in package_list.iter().filter(|p| p.license_text.is_none()) {
        package_hash_map.insert(format!("{}-{}", &p.name, &p.version), p);
    }

    src_registry_folders(cargo_folder())
        .iter()
        .map(|src_folder| {
            info!("src folder: {:?}", &src_folder);

            read_dir(src_folder)
                .expect("Failed reading source folder.")
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let folder_name_os = e.file_name();
                    let folder_name = folder_name_os.to_string_lossy();
                    package_hash_map
                        .get(folder_name.as_ref())
                        .and_then(|p| Some((e, p)))
                })
                .filter(|(e, _)| e.file_type().map_or(false, |e| e.is_dir()))
                .map(|(e, p)| {
                    let mut package_with_license = (*p).clone();
                    info!("Fetching license for: {}", &p.name);
                    package_with_license.license_text = license_text_from_folder(&e.path());
                    package_with_license
                })
                .collect::<Vec<Package>>()
        })
        .flatten()
        .collect::<Vec<Package>>()
        .into()
}
