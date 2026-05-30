// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{error::Error, fmt, fs::read_dir};

use exn::{ensure, OptionExt, Result, ResultExt};

use super::{ConfigBuilder, PathBuf, CIE};

#[derive(Debug, Clone)]
struct FromPathError(String);

impl fmt::Display for FromPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to get and validate path of manifest, {}", self.0)
    }
}

impl Error for FromPathError {}

fn manifest_path_from_file_path(uncertain_file_path: PathBuf) -> Result<PathBuf, FromPathError> {
    debug_assert!(uncertain_file_path.is_file());

    ensure!(
        uncertain_file_path
            .file_name()
            .ok_or_raise(|| FromPathError(format!(
                "manifest file path is not allowed to end in `..` '{}',  it should point to 'Cargo.toml'",
                uncertain_file_path.display()
            )))?
            == "Cargo.toml",
            FromPathError(format!("manifest file path points to '{}', it should point to 'Cargo.toml'", uncertain_file_path.display()))
    );

    Ok(uncertain_file_path)
}

fn manifest_path_from_dir_path(uncertain_dir_path: &PathBuf) -> Result<PathBuf, FromPathError> {
    debug_assert!(uncertain_dir_path.is_dir());

    read_dir(uncertain_dir_path)
        .or_raise(|| {
            FromPathError(format!(
                "cannot read dir '{}'",
                uncertain_dir_path.display()
            ))
        })?
        .filter_map(std::result::Result::ok)
        .find(|e| e.file_type().is_ok_and(|e| e.is_file()) && e.file_name() == "Cargo.toml")
        .map(|e| e.path())
        .ok_or_raise(|| {
            FromPathError(format!(
                "failed to find manifest in '{}'",
                uncertain_dir_path.display()
            ))
        })
}

fn manifest_dir(uncertain_path: PathBuf) -> Result<PathBuf, FromPathError> {
    ensure!(
        uncertain_path
            .try_exists()
            .or_raise(|| FromPathError(format!(
                "failed to verify existence of path '{}'",
                uncertain_path.display()
            )))?,
        FromPathError(format!(
            "path '{}' does not exist",
            uncertain_path.display()
        ))
    );

    let manifest_path = if uncertain_path.is_file() {
        manifest_path_from_file_path(uncertain_path)
    } else {
        manifest_path_from_dir_path(&uncertain_path)
    }?;

    Ok(manifest_path
        .parent()
        .ok_or_raise(|| {
            FromPathError(format!(
                "failed to get parent dir of '{}'",
                manifest_path.display()
            ))
        })?
        .to_path_buf())
}

impl ConfigBuilder {
    /// Sets [`manifest_dir`](Self::manifest_dir) from a path to a manifest (`Cargo.toml`) or a directory that contains a manifest.
    ///
    /// The difference to the aforementioned method is, that this method checks that the directory contains a manifest.
    /// Essentially a sanity check.
    #[must_use]
    pub fn with_path(mut self, manifest_path: impl Into<PathBuf>) -> Self {
        match manifest_dir(manifest_path.into()) {
            Ok(manifest_dir) => self = self.manifest_dir(manifest_dir),
            Err(err) => self
                .errors
                .push(err.raise(CIE("failed to infer config from path".to_owned()))),
        }
        self
    }

    /// New builder with [`manifest_dir`](Self::manifest_dir) being set from a path to a manifest (`Cargo.toml`) or a directory that contains a manifest.
    ///
    /// The difference to the aforementioned method is, that this method checks that the directory contains a manifest.
    /// Essentially a sanity check.
    pub fn from_path(manifest_path: impl Into<PathBuf>) -> Self {
        ConfigBuilder::default().with_path(manifest_path)
    }
}

/* -------------------------------------------------------------------------- */
/*                                 Unit Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod test {
    use crate::build::{config::error::ConfigBuilderError, debug::setup_test};

    use super::*;

    #[test]
    fn test_from_path_with_file_path() -> Result<(), ConfigBuilderError> {
        setup_test();
        let conf = ConfigBuilder::from_path(env!("CARGO_MANIFEST_PATH")).build()?;
        assert_eq!(
            conf.metadata_config.manifest_dir,
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        );
        assert_eq!(
            conf.metadata_config.cargo_path,
            PathBuf::from(env!("CARGO"))
        );

        Ok(())
    }

    #[test]
    fn test_from_path_with_dir_path() -> Result<(), ConfigBuilderError> {
        setup_test();
        let conf = ConfigBuilder::from_path(env!("CARGO_MANIFEST_DIR")).build()?;
        assert_eq!(
            conf.metadata_config.manifest_dir,
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        );
        assert_eq!(
            conf.metadata_config.cargo_path,
            PathBuf::from(env!("CARGO"))
        );

        Ok(())
    }
}
