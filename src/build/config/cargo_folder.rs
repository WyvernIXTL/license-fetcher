// Copyright Adam McKellar 2024, 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{env::var_os, error::Error, fmt, path::PathBuf};

use exn::{ensure, OptionExt, Result};

#[derive(Debug, Clone)]
pub(super) struct CargoFolderError(String);

impl fmt::Display for CargoFolderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to infer cargo home, {}", self.0)
    }
}

impl Error for CargoFolderError {}

pub fn cargo_folder() -> Result<PathBuf, CargoFolderError> {
    let cargo_home: PathBuf;

    if let Some(path) = var_os("CARGO_HOME") {
        cargo_home = path.into();
    } else {
        let home_dir = std::env::home_dir().ok_or_raise(|| {
            CargoFolderError(
                "system could not find user home directory, see docs of `std::env::home_dir`"
                    .to_string(),
            )
        })?;
        cargo_home = home_dir.join(".cargo");
    }

    ensure!(
        cargo_home.exists(),
        CargoFolderError(format!(
            "path '{}' was expected to exist",
            cargo_home.display()
        ))
    );
    ensure!(
        cargo_home.is_dir(),
        CargoFolderError(format!(
            "path '{}' was expected to be a directory",
            cargo_home.display()
        ))
    );

    Ok(cargo_home)
}
