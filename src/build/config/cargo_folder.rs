// Copyright Adam McKellar 2024, 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{env::var_os, error::Error, path::PathBuf};

use directories::BaseDirs;
use error_stack::{ensure, Report, Result};

use crate::build::error::CPath;

/// Error dealing with failed attempts to determine the users cargo home folder.
#[derive(Debug, Clone, Copy, displaydoc::Display)]
pub enum CargoFolderError {
    /// failed determining users home directory
    BaseDirs,
    /// given or inferred cargo home folder location does not exist
    DoesNotExist,
    /// inferred or supplied cargo home folder is not a folder
    IsNotDir,
}

impl Error for CargoFolderError {}

pub fn cargo_folder() -> Result<PathBuf, CargoFolderError> {
    let cargo_home: PathBuf;

    if let Some(path) = var_os("CARGO_HOME") {
        cargo_home = path.into();
    } else {
        let base_dir = BaseDirs::new().ok_or(CargoFolderError::BaseDirs)?;
        let home_dir = base_dir.home_dir();
        let mut cargo_dir = home_dir.to_path_buf();
        cargo_dir.push(".cargo");

        cargo_home = cargo_dir;
    }

    ensure!(
        cargo_home.exists(),
        Report::new(CargoFolderError::DoesNotExist).attach_printable(CPath::from(&cargo_home))
    );
    ensure!(
        cargo_home.is_dir(),
        Report::new(CargoFolderError::IsNotDir).attach_printable(CPath::from(&cargo_home))
    );

    Ok(cargo_home)
}
