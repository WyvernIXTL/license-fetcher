// Copyright Adam McKellar 2024, 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    error::Error,
    fs::read_dir,
    path::{Path, PathBuf},
};

use error_stack::{ensure, Report, Result, ResultExt};

use crate::build::error::CPath;

/// Error that may occur, when failing to read or determine cargo source registry folder.
#[derive(Debug, Clone, Copy, displaydoc::Display)]
pub enum SrcRegistryInferenceError {
    /// source registry folder does not exist at the inferred path
    DoesNotExist,
    /// the inferred path of the source registry is not a folder
    IsNotAFolder,
    /// failed to read the inferred source registry directory
    FailedReadDir,
}

impl Error for SrcRegistryInferenceError {}

pub fn src_registry_folders(
    path: &Path,
) -> Result<impl Iterator<Item = PathBuf>, SrcRegistryInferenceError> {
    let src_dir = path.join("registry/src");
    ensure!(
        src_dir.exists(),
        Report::new(SrcRegistryInferenceError::DoesNotExist).attach_printable(CPath::from(src_dir))
    );
    ensure!(
        src_dir.is_dir(),
        Report::new(SrcRegistryInferenceError::IsNotAFolder).attach_printable(CPath::from(src_dir))
    );
    Ok(read_dir(&src_dir)
        .attach_printable_lazy(|| CPath::from(&src_dir))
        .change_context(SrcRegistryInferenceError::FailedReadDir)?
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_ok_and(|ft| ft.is_dir()))
        .map(|e| e.path()))
}
