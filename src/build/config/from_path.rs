// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fs::read_dir;

use exn::{OptionExt, Result, ResultExt, ensure};

use crate::build::config::error::CEK;

use super::{Cie, ConfigBuilder, PathBuf};

fn manifest_path_from_file_path(uncertain_file_path: PathBuf) -> Result<PathBuf, Cie> {
    debug_assert!(uncertain_file_path.is_file());

    ensure!(
        uncertain_file_path.file_name().ok_or_raise(|| Cie::new(
            "path to manifest file should point to a file with a file name not `..`"
        )
        .with_path(&uncertain_file_path)
        .with_kind(CEK::FailedFromPath))?
            == "Cargo.toml",
        Cie::new("path to manifest file should point to a file with the name `Cargo.toml`")
            .with_path(&uncertain_file_path)
            .with_kind(CEK::FailedFromPath)
    );

    Ok(uncertain_file_path)
}

fn manifest_path_from_dir_path(uncertain_dir_path: &PathBuf) -> Result<PathBuf, Cie> {
    debug_assert!(uncertain_dir_path.is_dir());

    read_dir(uncertain_dir_path)
        .or_raise(|| {
            Cie::new("directory with manifest file should be readable")
                .with_path(uncertain_dir_path)
                .with_kind(CEK::FailedFromPath)
        })?
        .filter_map(std::result::Result::ok)
        .find(|e| e.file_type().is_ok_and(|e| e.is_file()) && e.file_name() == "Cargo.toml")
        .map(|e| e.path())
        .ok_or_raise(|| {
            Cie::new("manifest file should be in directory")
                .with_path(uncertain_dir_path)
                .with_kind(CEK::FailedFromPath)
        })
}

fn manifest_dir(uncertain_path: PathBuf) -> Result<PathBuf, Cie> {
    ensure!(
        uncertain_path.try_exists().or_raise(|| Cie::new(
            "path to manifest file or dir should be verifiable to exist or not exist"
        )
        .with_path(&uncertain_path)
        .with_kind(CEK::FailedFromPath))?,
        Cie::new(
            "path to manifest file or dir should point to an existing entity (file or folder)"
        )
        .with_path(uncertain_path)
        .with_kind(CEK::FailedFromPath)
    );

    let manifest_path = if uncertain_path.is_file() {
        manifest_path_from_file_path(uncertain_path)
    } else {
        manifest_path_from_dir_path(&uncertain_path)
    }?;

    Ok(manifest_path
        .parent()
        .ok_or_raise(|| {
            Cie::new("path to manifest was determined, but path to the parent directory should also be determinable")
                .with_path(&manifest_path)
                .with_kind(CEK::FailedFromPath)
        })?
        .to_path_buf())
}

impl ConfigBuilder {
    /// Sets the required `manifest_dir` field from a path to a manifest (`Cargo.toml`) or a directory that contains a manifest.
    ///
    /// This method is almost equivalent to the [`manifest_dir`](Self::manifest_dir) method, with the main difference being
    /// that [`with_path`](Self::with_path) has many build in checks.
    #[must_use]
    pub fn with_path(mut self, manifest_path: impl Into<PathBuf>) -> Self {
        match manifest_dir(manifest_path.into()) {
            Ok(manifest_dir) => self = self.manifest_dir(manifest_dir),
            Err(err) => self
                .errors
                .push(err.raise(Cie::new("manifest directory should be determinable"))),
        }
        self
    }

    /// New builder with the required `manifest_dir` field being set from a path to a manifest (`Cargo.toml`) or a directory that contains a manifest.
    ///
    /// The [`from_path`](Self::from_path) method uses [`with_path`](Self::with_path) under the hood.
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
    use crate::build::config::error::ConfigBuilderError;

    use super::*;

    #[test]
    fn test_from_path_with_file_path() -> Result<(), ConfigBuilderError> {
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
