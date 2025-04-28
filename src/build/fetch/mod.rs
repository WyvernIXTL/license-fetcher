//               Copyright Adam McKellar 2024, 2025
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use std::collections::HashMap;
use std::fs::{read_dir, read_to_string};
use std::path::PathBuf;
use std::sync::LazyLock;

use log::{info, trace, warn};
use regex_lite::Regex;

mod cargo_folder;

use cargo_folder::cargo_folder;

use crate::PackageList;

fn src_registry_folders(path: PathBuf) -> impl Iterator<Item = PathBuf> {
    let src_subfolder = PathBuf::from("registry/src");
    let src_dir = path.join(src_subfolder);
    read_dir(src_dir)
        .expect("Src path is not a dir.")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map_or(false, |ft| ft.is_dir()))
        .map(|e| e.path())
}

pub(super) fn license_text_from_folder(path: &PathBuf) -> Option<String> {
    trace!("Fetching license in folder: {:?}", &path);

    static LICENSE_FILE_NAME_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i).*(license|copying|authors|notice|eula).*").unwrap());

    let license_text = read_dir(&path)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| LICENSE_FILE_NAME_REGEX.is_match(&e.file_name().to_string_lossy()))
        .filter(|e| e.file_type().map_or(false, |e| e.is_file()))
        .filter_map(|e| read_to_string(e.path()).ok())
        .fold(String::new(), |mut a, b| {
            a += &b;
            a += "\n\n";
            a
        });

    if license_text.is_empty() {
        warn!("Found no licenses in folder: {:?}", &path);
        return None;
    }

    Some(license_text)
}

pub(super) fn licenses_text_from_cargo_src_folder(package_list: &mut PackageList) {
    let mut package_hash_map = HashMap::new();
    for p in package_list.iter_mut().filter(|p| p.license_text.is_none()) {
        package_hash_map.insert(format!("{}-{}", &p.name, &p.version), p);
    }

    src_registry_folders(cargo_folder().unwrap()).for_each(|src_folder| {
        info!("src folder: {:?}", &src_folder);

        read_dir(src_folder)
            .expect("Failed reading source folder.")
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map_or(false, |e| e.is_dir()))
            .for_each(|e| {
                let folder_name_os = e.file_name();
                let folder_name = folder_name_os.to_string_lossy();
                if let Some((e, p)) = package_hash_map
                    .get_mut(folder_name.as_ref())
                    .and_then(|p| Some((e, p)))
                {
                    info!("Fetching license for: {}", &p.name);
                    (**p).license_text = license_text_from_folder(&e.path());
                }
            });
    });
}
