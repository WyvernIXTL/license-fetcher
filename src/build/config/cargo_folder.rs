// Copyright Adam McKellar 2024, 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{env::var_os, path::PathBuf};

use exn::{ensure, OptionExt, Result};

use crate::build::config::Cie;

pub(super) fn cargo_folder() -> Result<PathBuf, Cie> {
    let cargo_home: PathBuf;

    if let Some(path) = var_os("CARGO_HOME") {
        cargo_home = path.into();
    } else {
        let home_dir = std::env::home_dir().ok_or_raise(|| {
            Cie::new("`std::env::home_dir` should return the home directory of the user")
        })?;
        cargo_home = home_dir.join(".cargo");
    }

    ensure!(
        cargo_home.exists(),
        Cie::new("cargo home folder should exist at path").with_path(cargo_home)
    );
    ensure!(
        cargo_home.is_dir(),
        Cie::new("cargo home path should point to a folder").with_path(cargo_home)
    );

    Ok(cargo_home)
}
