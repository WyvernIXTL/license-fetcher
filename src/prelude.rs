// Copyright Adam McKellar 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/*!
 * The prelude of `license-fetcher`.
 *
 * ```
 * use license_fetcher::prelude::*;
 * ```
 */

pub use crate::{read_package_list_from_out_dir, Package, PackageList, OUT_FILE_NAME};

#[cfg(feature = "build")]
pub use crate::build::{
    config::{
        error::ConfigBuilderError, error::ConfigBuilderErrorKind, CargoDirective,
        CargoDirectiveList, Config, ConfigBuilder, MetadataConfig,
    },
    fetcher::{
        error::{LicenseFetcherError, EK},
        package_list, package_list_with_licenses,
    },
    write::error::WriteError,
};
