//               Copyright Adam McKellar 2024, 2025
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use std::env::var_os;
use std::path::PathBuf;

use directories::BaseDirs;
use log::{info, trace, warn};
use once_cell::sync::Lazy;
use regex::Regex;
use smol::fs::{read_dir, read_to_string};
use smol::stream::{Stream, StreamExt};

use crate::PackageList;

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

async fn src_registry_folders(path: PathBuf) -> impl Stream<Item = PathBuf> {
    let src_subfolder = PathBuf::from("registry/src");
    let src_dir = path.join(src_subfolder);

    read_dir(src_dir)
        .await
        .expect("Src path is not a dir.")
        .filter_map(|e| e.ok())
        .then(|e| async move {
            if e.file_type().await.map_or(false, |t| t.is_dir()) {
                Some(e.path())
            } else {
                None
            }
        })
        .filter_map(|e| e)
}

pub(super) async fn license_text_from_folder(path: &PathBuf) -> Option<String> {
    trace!("Fetching license in folder: {:?}", &path);

    static LICENSE_FILE_NAME_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i).*(license|copying|authors|notice|eula).*").unwrap());

    let license_texts: Vec<String> = (&mut read_dir(&path)
        .await
        .expect("Failed reading source dir of dependency."))
        .filter_map(|e| e.ok())
        .filter(|e| LICENSE_FILE_NAME_REGEX.is_match(&e.file_name().to_string_lossy()))
        .then(|e| async move {
            if e.file_type().await.map_or(false, |t| t.is_file()) {
                Some(read_to_string(e.path()).await)
            } else {
                None
            }
        })
        .filter_map(|e| e?.ok())
        .collect()
        .await;

    if license_texts.is_empty() {
        warn!("Found no licenses in folder: {:?}", &path);
        return None;
    }

    Some(license_texts.join("\n\n"))
}

pub(super) async fn licenses_text_from_cargo_src_folder(package_list: &PackageList) -> PackageList {
    PackageList(
        src_registry_folders(cargo_folder())
            .await
            .inspect(|e| info!("src folder: {:?}", &e))
            .then(|d| read_dir(d))
            .filter_map(|r| r.ok())
            .flatten()
            .filter_map(|d| d.ok())
            .then(|e| async move {
                if e.file_type().await.map_or(false, |t| t.is_dir()) {
                    Some(e)
                } else {
                    None
                }
            })
            .filter_map(|e| e)
            .filter_map(|e| {
                let name_os = e.file_name();
                let name = name_os.to_str().unwrap();
                if let Some(p) = package_list
                    .iter()
                    .find(|p| name.starts_with(&p.name) && name.ends_with(&p.version))
                {
                    Some((e, p.clone()))
                } else {
                    None
                }
            })
            .inspect(|(_, p)| info!("Fetching license for: {}", p.name))
            .then(|(e, mut p)| async move {
                p.license_text = license_text_from_folder(&e.path()).await;
                p
            })
            .collect()
            .await,
    )
}
