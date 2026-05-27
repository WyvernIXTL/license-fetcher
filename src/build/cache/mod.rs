// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    error::Error,
    fmt,
    fs::read,
    path::{Path, PathBuf},
};

use exn::{ensure, Result, ResultExt};

use crate::{build::wrapper::PackageWrapper, Package, PackageList};

/// Error kinds occurring when reading cache file (old license data)
#[derive(Debug, Clone, Copy)]
pub enum CacheErrorKind {
    /// cache not found or cache is invalid
    Invalid,
    /// failed to read cache file
    ReadError,
}

/// Error occurring when reading cache file (old license data)
#[derive(Debug, Clone)]
pub struct CacheError {
    pub kind: CacheErrorKind,
    pub message: String,
    pub path: Option<PathBuf>,
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(path) = &self.path {
            write!(f, "{} with path '{}'", self.message, path.display())
        } else {
            f.write_str(&self.message)
        }
    }
}

impl Error for CacheError {}

impl CacheError {
    fn new(kind: CacheErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            path: None,
        }
    }

    fn add_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }
}

pub(super) fn read_package_list_with_tests(
    cache_file_path: &Path,
) -> Result<PackageList, CacheError> {
    ensure!(
        cache_file_path.try_exists().or_raise(|| CacheError::new(
            CacheErrorKind::Invalid,
            "cache path should not point to the root of a volume"
        )
        .add_path(cache_file_path))?,
        CacheError::new(
            CacheErrorKind::Invalid,
            "cache path should point to a file, currently points to nothing"
        )
        .add_path(cache_file_path)
    );

    ensure!(
        cache_file_path.is_file(),
        CacheError::new(CacheErrorKind::Invalid, "cache path should point to a file")
            .add_path(cache_file_path)
    );

    let cache_bin = read(cache_file_path).or_raise(|| {
        CacheError::new(CacheErrorKind::ReadError, "cache file should be readable")
            .add_path(cache_file_path)
    })?;
    PackageList::from_encoded(&cache_bin).or_raise(|| {
        CacheError::new(
            CacheErrorKind::Invalid,
            "cache file should be a valid serialized and compressed package list",
        )
        .add_path(cache_file_path)
    })
}

pub(super) fn populate_with_cache_from_package_list(
    package_iter: impl Iterator<Item = Package>,
    cache: PackageList,
) -> impl Iterator<Item = PackageWrapper> {
    package_iter.map(move |mut p| {
        let cached_package = cache
            .iter()
            .find(|c| c.name == p.name && c.version == p.version);
        p.license_texts = cached_package
            .map(|c| c.license_texts.clone())
            .unwrap_or(vec![]);
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
