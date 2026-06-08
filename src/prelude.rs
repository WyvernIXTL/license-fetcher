// Copyright Adam McKellar 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/*!
 * Prelude of `license-fetcher`.
 *
 * ```
 * use license_fetcher::prelude::*;
 * ```
 */

pub use crate::{
    OUT_FILE_NAME, Package, PackageList, error::UnpackError, read_package_list_from_out_dir,
};

#[cfg(feature = "build")]
pub use crate::build::{
    config::{
        CargoDirective, CargoDirectiveList, Config, ConfigBuilder, MetadataConfig, error::CEK,
        error::ConfigBuilderError,
    },
    fetcher::{
        error::{EK, LicenseFetcherError},
        package_list, package_list_with_licenses,
    },
    write::error::WriteError,
};
