// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{error::Error, fs::read, path::Path};

use error_stack::{ensure, report, Result, ResultExt};

use crate::{
    build::{error::CPath, wrapper::PackageWrapper},
    Package, PackageList,
};

/// Error occuring when reading cache file (old license data)
#[derive(Debug, Clone, Copy, displaydoc::Display)]
pub enum CacheError {
    /// cache not found or cache is invalid
    Invalid,
    /// failed to read cache file
    ReadError,
}

impl Error for CacheError {}

pub fn read_package_list_with_tests(cache_file_path: &Path) -> Result<PackageList, CacheError> {
    ensure!(
        cache_file_path
            .try_exists()
            .change_context(CacheError::Invalid)
            .attach_printable_lazy(|| CPath::from(&cache_file_path))?
            && cache_file_path.is_file(),
        report!(CacheError::Invalid).attach_printable(CPath::from(&cache_file_path))
    );
    let cache_bin = read(cache_file_path).change_context(CacheError::ReadError)?;
    PackageList::from_encoded(&cache_bin).change_context(CacheError::Invalid)
}

pub fn populate_with_cache_from_package_list(
    package_iter: impl Iterator<Item = Package>,
    cache: PackageList,
) -> impl Iterator<Item = PackageWrapper> {
    package_iter.map(move |mut p| {
        let cached_package = cache
            .iter()
            .find(|c| c.name == p.name && c.version == p.version);
        p.license_text = cached_package.and_then(|c| c.license_text.clone());
        PackageWrapper {
            package: p,
            restored_from_cache: cached_package.is_some(),
        }
    })
}

/* -------------------------------------------------------------------------- */
/*                                 Unit Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod test {
    // TODO: add tests for parsing here
}
