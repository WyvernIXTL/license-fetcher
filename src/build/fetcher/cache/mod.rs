// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{fs::read, path::Path};

use exn::{ensure, Result, ResultExt};

use crate::{
    build::{
        fetcher::error::{EK, IE},
        fetcher::wrapper::PackageWrapper,
    },
    Package, PackageList,
};

pub(super) fn read_package_list_with_tests(cache_file_path: &Path) -> Result<PackageList, IE> {
    ensure!(
        cache_file_path.try_exists().or_raise(|| IE::new(
            "cache path should not point to the root of a volume"
        )
        .with_path(cache_file_path)
        .with_kind(EK::Cache))?,
        IE::new("cache path should point to a file, currently points to nothing")
            .with_path(cache_file_path)
            .with_kind(EK::Cache)
    );

    ensure!(
        cache_file_path.is_file(),
        IE::new("cache path should point to a file")
            .with_path(cache_file_path)
            .with_kind(EK::Cache)
    );

    let cache_bin = read(cache_file_path).or_raise(|| {
        IE::new("cache file should be readable")
            .with_path(cache_file_path)
            .with_kind(EK::Cache)
    })?;
    PackageList::from_encoded(&cache_bin).or_raise(|| {
        IE::new("cache file should be a valid serialized and compressed package list")
            .with_path(cache_file_path)
            .with_kind(EK::Cache)
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
