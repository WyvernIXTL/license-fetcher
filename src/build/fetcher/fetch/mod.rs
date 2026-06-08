// Copyright Adam McKellar 2024, 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::HashMap;
use std::fs::{read_dir, read_to_string};
use std::path::Path;
use std::sync::LazyLock;

use exn::{Result, ResultExt};
use log::{error, info, trace, warn};
use regex_lite::Regex;

mod src_registry_folders;

use crate::build::fetcher::error::{EK, ErrorJoin, IE};
use crate::build::fetcher::wrapper::PackageWrapper;
use src_registry_folders::src_registry_folders;

pub(super) fn license_texts_from_folder(path: &Path) -> Result<Vec<(String, String)>, IE> {
    static LICENSE_FILE_NAME_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i).*(license|copying|authors|notice|eula).*").unwrap());

    trace!("Fetching license in folder: {}", path.display());

    // TODO: Split this up.
    let license_texts: Vec<(String, String)> = read_dir(path)
        .or_raise(|| IE::new("path to crate in cargo src folder should be readable, exist and be a folder").with_path(path))?
        .filter_map(std::result::Result::ok)
        .filter(|e| LICENSE_FILE_NAME_REGEX.is_match(&e.file_name().to_string_lossy()))
        .filter_map(|e| {
            if e.file_type().is_ok_and(|e| e.is_dir()) {
                Some(
                    read_dir(e.path())
                        .map_err(|err| {
                            let path = e.path();
                            error!("Failed reading sub license directory. Path: '{}'. Error: \n {err}", path.display());
                        })
                        .ok()?
                        .filter_map(std::result::Result::ok)
                        .filter(|e| {
                            LICENSE_FILE_NAME_REGEX.is_match(&e.file_name().to_string_lossy())
                        })
                        .collect(),
                )
            } else {
                Some(vec![e])
            }
        })
        .flat_map(std::iter::IntoIterator::into_iter)
        .filter(|e| e.file_type().is_ok_and(|e| e.is_file()))
        .filter_map(|e| {
            let abs_path = e.path();
            read_to_string(&abs_path)
                .map_err(|err| {
                    error!("Error during reading of license file. Skipping. Path: '{}'. Error: \n {err}", abs_path.display());
                })
                .ok()
                .map(|text| {
                    let rel_path = abs_path.strip_prefix(path).unwrap_or(&abs_path).to_string_lossy().into_owned();
                    (rel_path, text)
                })
        })
        .collect();

    if license_texts.is_empty() {
        warn!("Found no licenses in folder: {}", path.display());
    }

    Ok(license_texts)
}

/// Populate a package list with licenses from the cargo source folder.
///
/// If a package was loaded from a cache, it is ignored.
/// Failure of reading directories of packages are ignored.
pub(super) fn populate_package_list_licenses(
    package_list: &mut [PackageWrapper],
    cargo_home_dir: &Path,
) -> Result<(), IE> {
    let mut package_hash_map: HashMap<String, &mut PackageWrapper> = package_list
        .iter_mut()
        .filter(|p| p.package.license_texts.is_empty() && !p.restored_from_cache)
        .map(|p| (format!("{}-{}", p.package.name, p.package.version), p))
        .collect::<HashMap<_, _>>();

    let src_folder_iterator = src_registry_folders(cargo_home_dir).or_raise(|| {
        IE::new("src registry foulders should be in cargo home dir and should be readable")
            .with_path(cargo_home_dir)
    })?;

    let mut result = ErrorJoin::new(IE::new(
        "recursively searching for licenses in src registries and fetching them should succeed",
    ));

    for src_folder in src_folder_iterator {
        info!("src folder: {}", &src_folder.display());

        read_dir(&src_folder)
            .or_raise(|| {
                IE::new("reading src registry folder should succeed")
                    .with_path(src_folder)
                    .with_kind(EK::RegistryFolder)
            })?
            .filter_map(std::result::Result::ok)
            .filter(|e| e.file_type().is_ok_and(|e| e.is_dir()))
            .for_each(|e| {
                let folder_name_os = e.file_name();
                let folder_name = folder_name_os.to_string_lossy();
                if let Some((e, p)) = package_hash_map
                    .get_mut(folder_name.as_ref())
                    .map(|p| (e, p))
                {
                    info!("Fetching license for: {}", &p.package.name);

                    match license_texts_from_folder(&e.path()) {
                        Ok(res) => p.package.license_texts = res,
                        Err(err) => {
                            error!("Failure");
                            result.join(
                                err.raise(
                                    IE::new(format!(
                                        "fetching licenese for package \"{}\" should succeed",
                                        &p.package.name
                                    ))
                                    .with_path(e.path()),
                                ),
                            );
                        }
                    }
                }
            });
    }

    result.result()
}
