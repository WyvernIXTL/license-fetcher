// Copyright Adam McKellar 2024, 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;
use std::{
    error::Error,
    fs::{read_dir, read_to_string},
};

use error_stack::{Result, ResultExt};
use log::{error, info, trace, warn};
use regex_lite::Regex;

mod src_registry_folders;

use crate::build::error::CPath;
use crate::PackageList;
use src_registry_folders::src_registry_folders;

use super::error::ReportJoin;

/// Errors that may occur when reading and walking the cargo src registry folder.
#[derive(Debug, Clone, Copy, displaydoc::Display)]
pub enum LicenseFetchError {
    /// failed to infer or read cargo src registry folder
    RegistrySrc,
    /// failed to fetch license from a crates src folder
    LicenseFetchForPackage,
    /// failed to walk a crates src folder
    SrcFolderRecursion,
}

impl Error for LicenseFetchError {}

pub(crate) fn license_text_from_folder(path: &Path) -> Result<Option<String>, std::io::Error> {
    static LICENSE_FILE_NAME_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i).*(license|copying|authors|notice|eula).*").unwrap());

    trace!("Fetching license in folder: {}", path.display());

    // TODO: Split this up.
    let license_text = read_dir(path)
        .attach_printable_lazy(|| CPath::from(path))?
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
            read_to_string(e.path())
                .map_err(|err| {
                    let path = e.path();
                    error!("Error during reading of license file. Skipping. Path: '{}'. Error: \n {err}", path.display());
                })
                .ok()
        })
        .fold(String::new(), |mut a, b| {
            a += &b;
            a += "\n\n";
            a
        });

    if license_text.is_empty() {
        warn!("Found no licenses in folder: {}", path.display());
        return Ok(None);
    }

    Ok(Some(license_text))
}

/// Populate a package list with licenses from the cargo source folder.
///
/// If a package was loaded from a cache, it is ignored.
/// Failure of reading directories of packages are ignored.
pub fn populate_package_list_licenses(
    package_list: &mut PackageList,
    cargo_home_dir: &Path,
) -> Result<(), LicenseFetchError> {
    let mut package_hash_map: HashMap<String, &mut crate::Package> = package_list
        .iter_mut()
        .filter(|p| p.license_text.is_none() && !p.restored_from_cache)
        .map(|p| (p.name_version.clone(), p))
        .collect::<HashMap<_, _>>();

    let src_folder_iterator =
        src_registry_folders(cargo_home_dir).change_context(LicenseFetchError::RegistrySrc)?;

    let mut result = ReportJoin::default();

    for src_folder in src_folder_iterator {
        info!("src folder: {}", &src_folder.display());

        read_dir(&src_folder)
            .attach_printable_lazy(|| CPath::from(src_folder))
            .change_context(LicenseFetchError::SrcFolderRecursion)?
            .filter_map(std::result::Result::ok)
            .filter(|e| e.file_type().is_ok_and(|e| e.is_dir()))
            .for_each(|e| {
                let folder_name_os = e.file_name();
                let folder_name = folder_name_os.to_string_lossy();
                if let Some((e, p)) = package_hash_map
                    .get_mut(folder_name.as_ref())
                    .map(|p| (e, p))
                {
                    info!("Fetching license for: {}", &p.name);

                    match license_text_from_folder(&e.path()) {
                        Ok(res) => p.license_text = res,
                        Err(err) => {
                            error!("Failure");
                            let err = err.change_context(LicenseFetchError::LicenseFetchForPackage);
                            result.join(err);
                        }
                    }
                }
            });
    }

    result.result()
}
