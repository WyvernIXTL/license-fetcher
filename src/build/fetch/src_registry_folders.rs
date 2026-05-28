// Copyright Adam McKellar 2024, 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

use exn::{ensure, Result, ResultExt};

use crate::build::fetching_error::{EK, IE};

pub(crate) fn src_registry_folders(path: &Path) -> Result<impl Iterator<Item = PathBuf>, IE> {
    let src_dir = path.join("registry/src");
    ensure!(
        src_dir.exists(),
        IE::new("source registry folder should exist")
            .with_path(src_dir)
            .with_kind(EK::RegistryFolder)
    );
    ensure!(
        src_dir.is_dir(),
        IE::new("path to source registry folder should point to a folder")
            .with_path(src_dir)
            .with_kind(EK::RegistryFolder)
    );
    Ok(read_dir(&src_dir)
        .or_raise(|| {
            IE::new("source registry foulder should be readable")
                .with_path(src_dir)
                .with_kind(EK::RegistryFolder)
        })?
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_ok_and(|ft| ft.is_dir()))
        .map(|e| e.path()))
}
