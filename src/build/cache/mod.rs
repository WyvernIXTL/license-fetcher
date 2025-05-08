// Copyright Adam McKellar 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{env::var_os, fs::read, path::PathBuf};

use error_stack::{ensure, report, Result, ResultExt};
use fnv::FnvHashMap;
use thiserror::Error;

use crate::{build::error::CPath, PackageList};

#[derive(Debug, Clone, Copy, Error)]
pub enum CacheError {
    #[error("You are running a build script (`build.rs`) only function during runtime.")]
    NotBuildScript,
    #[error("Cache was not able to be found or is invalid.")]
    Invalid,
    #[error("Failed to read valid cache path.")]
    ReadError,
}

fn load_package_list_from_out_dir_during_build_script() -> Result<PackageList, CacheError> {
    let mut old_pkg_list_path =
        PathBuf::from(var_os("OUT_DIR").ok_or_else(|| CacheError::NotBuildScript)?);
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

/// Use previously fetched licenses to fill in a [PackageList].
///
/// Beware to call this function only in build scripts (`build.rs`)!
pub fn populate_with_cache(pkg_list: &mut PackageList) -> Result<(), CacheError> {
    let cache = load_package_list_from_out_dir_during_build_script()?;

    // TODO: Check if a vec linear search is faster.
    let cache_map = FnvHashMap::from_iter(cache.iter().map(|e| (&e.name_version, e)));
    for pkg in pkg_list.iter_mut() {
        if let Some(c) = cache_map.get(&pkg.name_version) {
            pkg.restored_from_cache = true;
            pkg.license_text = c.license_text.clone();
        }
    }

    Ok(())
}
