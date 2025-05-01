// Copyright Adam McKellar 2024, 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::HashMap;
use std::fs::{read_dir, read_to_string};
use std::path::PathBuf;
use std::sync::LazyLock;

use error_stack::{Result, ResultExt};
use log::{error, info, trace, warn};
use regex_lite::Regex;

mod src_registry_folders;

use thiserror::Error;

use crate::build::error::CPath;
use crate::PackageList;
use src_registry_folders::src_registry_folders;

#[derive(Debug, Clone, Copy, Error)]
pub enum LicenseFetchError {
    #[error("Failed to infer the cargo folders location.")]
    CargoFolder,
    #[error("Failed to infer the registry src folder location.")]
    RegistrySrc,
    #[error("Failure during the fetching of licenses for a package.")]
    LicenseFetchForPackage,
    #[error("Failed reading a src folder of a registry.")]
    SrcFolderRecursion,
}

pub(crate) fn license_text_from_folder(path: &PathBuf) -> Result<Option<String>, std::io::Error> {
    trace!("Fetching license in folder: {:?}", &path);

    static LICENSE_FILE_NAME_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i).*(license|copying|authors|notice|eula).*").unwrap());

    let license_text = read_dir(&path)
        .attach_printable_lazy(|| CPath::from(path))?
        .filter_map(|e| e.ok())
        .filter(|e| LICENSE_FILE_NAME_REGEX.is_match(&e.file_name().to_string_lossy()))
        .filter(|e| e.file_type().map_or(false, |e| e.is_file()))
        .filter_map(|e| {
            read_to_string(e.path())
                .map_err(|err| {
                    let path = e.path();
                    error!(path:debug, err:err ; "Error during reading of license file. Skipping.")
                })
                .ok()
        })
        .fold(String::new(), |mut a, b| {
            a += &b;
            a += "\n\n";
            a
        });

    if license_text.is_empty() {
        warn!("Found no licenses in folder: {:?}", &path);
        return Ok(None);
    }

    Ok(Some(license_text))
}

pub(crate) fn licenses_text_from_cargo_src_folder(
    package_list: &mut PackageList,
    cargo_home_dir: PathBuf,
) -> Result<(), LicenseFetchError> {
    let mut package_hash_map = HashMap::new();
    for p in package_list.iter_mut().filter(|p| p.license_text.is_none()) {
        package_hash_map.insert(format!("{}-{}", &p.name, &p.version), p);
    }

    let mut src_folder_iterator =
        src_registry_folders(cargo_home_dir).change_context(LicenseFetchError::RegistrySrc)?;

    let mut result: Result<(), LicenseFetchError> = Ok(());

    while let Some(src_folder) = src_folder_iterator.next() {
        info!("src folder: {:?}", &src_folder);

        read_dir(&src_folder)
            .attach_printable_lazy(|| CPath::from(src_folder))
            .change_context(LicenseFetchError::SrcFolderRecursion)?
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

                    match license_text_from_folder(&e.path()) {
                        Ok(res) => (**p).license_text = res,
                        Err(err) => {
                            error!("Failure");
                            let err = err.change_context(LicenseFetchError::LicenseFetchForPackage);
                            match result.as_mut() {
                                Ok(_) => result = Err(err),
                                Err(e) => e.extend_one(err),
                            }
                        }
                    }
                }
            });
    }

    result
}
