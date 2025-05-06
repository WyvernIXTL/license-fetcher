// Copyright Adam McKellar 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fs::read_dir;

use error_stack::{ensure, Report};
use error_stack::{Result, ResultExt};
use thiserror::Error;

use crate::build::error::CPath;

use super::*;

/// Error that appears during failed build of config via [ConfigBuilder::from_path()].
#[derive(Debug, Error)]
pub enum FromPathError {
    #[error("The requested path does not exist or this program does not have the permission to access it.")]
    PathDoesNotExist,
    #[error("Manifest not found.")]
    ManifestNotFound,
    #[error("Io error.")]
    Io,
}

fn manifest_path_from_file_path(uncertain_file_path: PathBuf) -> Result<PathBuf, FromPathError> {
    debug_assert!(uncertain_file_path.is_file());

    ensure!(
        uncertain_file_path
            .file_name()
            .ok_or(FromPathError::ManifestNotFound)
            .attach_printable("Path to file provided has an invalid file name.")
            .attach_printable_lazy(|| CPath::from(&uncertain_file_path))?
            == "Cargo.toml",
        Report::new(FromPathError::ManifestNotFound)
            .attach_printable("The provided path points to a file that is not 'Cargo.toml'.")
            .attach_printable(CPath::from(&uncertain_file_path))
    );

    Ok(uncertain_file_path)
}

fn manifest_path_from_dir_path(uncertain_dir_path: PathBuf) -> Result<PathBuf, FromPathError> {
    debug_assert!(uncertain_dir_path.is_dir());

    read_dir(&uncertain_dir_path)
        .attach_printable_lazy(|| CPath::from(&uncertain_dir_path))
        .change_context(FromPathError::Io)?
        .filter_map(|e| e.ok())
        .find(|e| e.file_type().map_or(false, |e| e.is_file()) && e.file_name() == "Cargo.toml")
        .map(|e| e.path())
        .ok_or_else(|| Report::new(FromPathError::ManifestNotFound))
        .attach_printable_lazy(|| CPath::from(&uncertain_dir_path))
}

fn manifest_dir(uncertain_path: PathBuf) -> Result<PathBuf, FromPathError> {
    ensure!(
        uncertain_path
            .try_exists()
            .attach_printable_lazy(|| CPath::from(&uncertain_path))
            .attach_printable("Failed verifying existence of provided path.")
            .change_context(FromPathError::Io)?,
        Report::new(FromPathError::PathDoesNotExist).attach_printable(CPath::from(&uncertain_path))
    );

    let manifest_path = if uncertain_path.is_file() {
        manifest_path_from_file_path(uncertain_path)
    } else {
        manifest_path_from_dir_path(uncertain_path)
    }?;

    Ok(manifest_path
        .parent()
        .ok_or_else(|| FromPathError::Io)
        .attach_printable_lazy(|| CPath::from(&manifest_path))?
        .to_path_buf())
}

impl ConfigBuilder {
    /// Sets [manifest_dir](Self::manifest_dir) from a path to a manifest (`Cargo.toml`) or a directory that contains a manifest.
    ///
    /// The difference to the aforementioned method is, that this method checks that the directory contains a manifest.
    /// Essentially a sanity check.
    pub fn with_path(self, manifest_path: impl Into<PathBuf>) -> Result<Self, ConfigBuildError> {
        let manifest_dir =
            manifest_dir(manifest_path.into()).change_context(ConfigBuildError::FailedFromPath)?;

        let builder = self.manifest_dir(manifest_dir);

        Ok(builder)
    }

    /// New builder with [manifest_dir](Self::manifest_dir) being set from a path to a manifest (`Cargo.toml`) or a directory that contains a manifest.
    ///
    /// The difference to the aforementioned method is, that this method checks that the directory contains a manifest.
    /// Essentially a sanity check.
    pub fn from_path(manifest_path: impl Into<PathBuf>) -> Result<Self, ConfigBuildError> {
        Ok(ConfigBuilder::default().with_path(manifest_path)?)
    }
}

#[cfg(test)]
mod test {
    use crate::build::debug::setup_test;

    use super::*;

    #[test]
    fn test_from_path_with_file_path() -> Result<(), ConfigBuildError> {
        setup_test();
        let conf = ConfigBuilder::from_path(env!("CARGO_MANIFEST_PATH"))?.build()?;
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
    fn test_from_path_with_dir_path() -> Result<(), ConfigBuildError> {
        setup_test();
        let conf = ConfigBuilder::from_path(env!("CARGO_MANIFEST_DIR"))?.build()?;
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
