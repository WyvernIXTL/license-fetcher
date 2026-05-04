// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{collections::HashMap, env::var_os, error::Error, fmt, fs::read, path::PathBuf};

use error_stack::{ensure, report, Result, ResultExt};

use crate::{build::error::CPath, PackageList};

#[derive(Debug, Clone, Copy)]
pub enum CacheError {
    NotBuildScript,
    Invalid,
    ReadError,
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NotBuildScript => {
                "You are running a build script (`build.rs`) only function during runtime."
            }
            Self::Invalid => "Cache was not able to be found or is invalid.",
            Self::ReadError => "Failed to read valid cache path.",
        };
        f.write_str(message)
    }
}

impl Error for CacheError {}

fn load_package_list_from_out_dir_during_build_script() -> Result<PackageList, CacheError> {
    let mut old_pkg_list_path = PathBuf::from(var_os("OUT_DIR").ok_or(CacheError::NotBuildScript)?);
    old_pkg_list_path.push("LICENSE-3RD-PARTY.bincode.deflate");
    ensure!(
        old_pkg_list_path
            .try_exists()
            .change_context(CacheError::Invalid)
            .attach_printable_lazy(|| CPath::from(&old_pkg_list_path))?
            && old_pkg_list_path.is_file(),
        report!(CacheError::Invalid).attach_printable(CPath::from(&old_pkg_list_path))
    );
    let old_pkg_list_bin = read(&old_pkg_list_path).change_context(CacheError::ReadError)?;
    PackageList::from_encoded(&old_pkg_list_bin).change_context(CacheError::Invalid)
}

/// Use previously fetched licenses to fill in a [`PackageList`].
///
/// Beware to call this function only in build scripts (`build.rs`)!
pub fn populate_with_cache(pkg_list: &mut PackageList) -> Result<(), CacheError> {
    let cache = load_package_list_from_out_dir_during_build_script()?;

    // TODO: Check if a vec linear search is faster.
    let cache_map: HashMap<&String, &crate::Package> = cache
        .iter()
        .map(|e| (&e.name_version, e))
        .collect::<HashMap<_, _>>();
    for pkg in pkg_list.iter_mut() {
        if let Some(c) = cache_map.get(&pkg.name_version) {
            pkg.restored_from_cache = true;
            pkg.license_text = c.license_text.clone();
        }
    }

    Ok(())
}
