//               Copyright Adam McKellar 2024
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)


use std::path::PathBuf;
use std::env::var_os;
use std::fs::{read_dir, read_to_string};

use directories::BaseDirs;
use log::{info, warn, trace};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::PackageList;


fn cargo_folder() -> PathBuf {
    if let Some(path) = var_os("CARGO_HOME") {
        path.into()
    } else {
        let base_dir = BaseDirs::new().expect("Failed to find home dir.");
        let home_dir = base_dir.home_dir();
        home_dir.into()
    }
}

fn src_folder(path: PathBuf) -> PathBuf {
    let src_subfolder = PathBuf::from("registry/src");
    let src_dir = path.join(src_subfolder);
    read_dir(src_dir).expect("Src path is not a dir.").filter(|p| {
        if let Ok(entry) = p {
            return entry.path().is_dir();
        }
        false
    }).next().expect("Failed to find src directory in .cargo/registry/src")
    .expect("Failed unwrapping src dir.")
    .path()
}

pub(super) fn license_text_from_folder(path: &PathBuf) ->  Option<String> {
    trace!("Fetching license in folder: {:?}", &path);

    let entries = read_dir(&path).unwrap();

    static LICENSE_FILE_NAME_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i).*(license|copying|authors|notice|eula).*").unwrap()
    });

    let mut potential_license_files = vec![];

    for entry in entries {
        if let Ok(entry) = entry {
            if let Ok(metadata) = entry.metadata() {
                if !metadata.is_file() {
                    continue;
                }
                if LICENSE_FILE_NAME_REGEX.is_match(&entry.file_name().to_string_lossy()) {
                    potential_license_files.push(entry.path());
                }
            }
        }
    }

    let mut license_text_vec = vec![];

    for license_file in potential_license_files {
        if let Ok(license_text) = read_to_string(license_file) {
            license_text_vec.push(license_text);
        }
    }

    if license_text_vec.is_empty() {
        warn!("Found no licenses in folder: {:?}", &path);
        return None;
    }

    Some(license_text_vec.join("\n\n"))
}

pub(super) fn licenses_text_from_cargo_src_folder(package_list: &mut PackageList) {
    let src_folder = src_folder(cargo_folder());
    info!("src folder: {:?}", &src_folder);

    for folder in read_dir(src_folder)
                            .expect("Failed reading source folder.")
                            .into_iter()
                            .filter(|e| e.is_ok())
                            .map(|e| e.unwrap())
                            .filter(|e| e.path().is_dir())
                            .map(|e| e.path()) {
        let folder_name = folder.as_path().iter().last().unwrap().to_str().unwrap();
        for package in package_list.iter_mut()
                                    .filter(|p| p.license_text.is_none()) {
            if folder_name.starts_with(&package.name) && folder_name.ends_with(&package.version) {
                info!("Fetching license for: {}", &package.name);
                package.license_text = license_text_from_folder(&folder);
            }
        }
    }
} 