// Copyright Adam McKellar 2024, 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{env::var_os, error::Error, fmt, path::PathBuf};

use error_stack::{ensure, Report, Result};

use crate::build::error::CPath;

/// Error dealing with failed attempts to determine the users cargo home folder.
#[derive(Debug, Clone, Copy)]
pub enum CargoFolderError {
    /// failed to find the home directory
    HomeDirNotFound,
    /// given or inferred cargo home folder location does not exist
    DoesNotExist,
    /// inferred or supplied cargo home folder is not a folder
    IsNotDir,
}

impl fmt::Display for CargoFolderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HomeDirNotFound => write!(f, "failed to find the home directory"),
            Self::DoesNotExist => write!(
                f,
                "given or inferred cargo home folder location does not exist"
            ),
            Self::IsNotDir => write!(f, "inferred or supplied cargo home folder is not a folder"),
        }
    }
}

impl Error for CargoFolderError {}

pub fn cargo_folder() -> Result<PathBuf, CargoFolderError> {
    let cargo_home: PathBuf;

    if let Some(path) = var_os("CARGO_HOME") {
        cargo_home = path.into();
    } else {
        let home_dir =
            std::env::home_dir().ok_or(Report::new(CargoFolderError::HomeDirNotFound))?;
        cargo_home = home_dir.join(".cargo");
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
