// Copyright Adam McKellar 2024, 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{fs::read_dir, path::PathBuf};

use error_stack::{ensure, Report, Result, ResultExt};
use thiserror::Error;

use crate::build::error::CPath;

#[derive(Debug, Clone, Copy, Error)]
pub enum SrcRegistryInferenceError {
    #[error("Source registry folder does not exist at the inferred path.")]
    DoesNotExist,
    #[error("The inferred path of the source registry is not a folder.")]
    IsNotAFolder,
    #[error("Failed to read the inferred source registry path.")]
    FailedReadDir,
}

pub fn src_registry_folders(
    path: PathBuf,
) -> Result<impl Iterator<Item = PathBuf>, SrcRegistryInferenceError> {
    let src_subfolder = PathBuf::from("registry/src");
    let src_dir = path.join(src_subfolder);
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
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map_or(false, |ft| ft.is_dir()))
        .map(|e| e.path()))
}
