// Copyright Adam McKellar 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fs::{read_dir, read_to_string};

use error_stack::{ensure, Report};
use error_stack::{Result, ResultExt};
use serde::Deserialize;
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
    #[error("Failure parsing 'Cargo.toml'.")]
    TomlParseError,
}

#[derive(Deserialize)]
struct CargoToml {
    package: CargoPackage,
}

#[derive(Deserialize)]
struct CargoPackage {
    name: String,
    // path: PathBuf,
}

fn valid_cargo_toml_path(uncertain_file_path: PathBuf) -> Result<PathBuf, FromPathError> {
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

fn find_valid_cargo_toml_path(uncertain_dir_path: PathBuf) -> Result<PathBuf, FromPathError> {
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

fn manifest_file_path(uncertain_path: PathBuf) -> Result<PathBuf, FromPathError> {
    if uncertain_path.is_file() {
        valid_cargo_toml_path(uncertain_path)
    } else {
        find_valid_cargo_toml_path(uncertain_path)
    }
}

struct MetadataManifest {
    package_name: String,
    manifest_dir: PathBuf,
}

impl MetadataManifest {
    fn new(manifest_path: impl Into<PathBuf>) -> Result<Self, FromPathError> {
        let manifest_path: PathBuf = manifest_path.into();

        ensure!(
            manifest_path
                .try_exists()
                .attach_printable_lazy(|| CPath::from(&manifest_path))
                .attach_printable("Failed verifying existence of provided path.")
                .change_context(FromPathError::Io)?,
            Report::new(FromPathError::PathDoesNotExist)
                .attach_printable(CPath::from(&manifest_path))
        );

        let manifest_file_path = manifest_file_path(manifest_path)?;

        let cargo_toml: CargoToml = toml::from_str(
            &read_to_string(&manifest_file_path)
                .attach_printable_lazy(|| CPath::from(&manifest_file_path))
                .change_context(FromPathError::Io)?,
        )
        .change_context(FromPathError::TomlParseError)?;

        let package_name = cargo_toml.package.name;
        let manifest_dir = manifest_file_path
            .parent()
            .ok_or_else(|| FromPathError::Io)
            .attach_printable_lazy(|| CPath::from(&manifest_file_path))?
            .to_path_buf();

        Ok(MetadataManifest {
            package_name,
            manifest_dir,
        })
    }
}

impl ConfigBuilder {
    /// Fills in needed values from a manifest (`Cargo.toml`).
    ///
    /// Expects either a path directly to the `Cargo.toml` file or to it's parent directory.
    pub fn with_path(self, manifest_path: impl Into<PathBuf>) -> Result<Self, ConfigBuildError> {
        let meta = MetadataManifest::new(manifest_path)
            .change_context(ConfigBuildError::FailedFromPath)?;

        let builder = self
            .package_name(meta.package_name)
            .manifest_dir(meta.manifest_dir);

        Ok(builder)
    }

    /// New builder with needed values being filled from a manifest (`Cargo.toml`).
    ///
    /// Expects either a path directly to the `Cargo.toml` file or to it's parent directory.
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
        assert_eq!(conf.package_name, env!("CARGO_PKG_NAME"));
        assert_eq!(conf.manifest_dir, PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        assert_eq!(conf.cargo_path, PathBuf::from("cargo"));

        Ok(())
    }

    #[test]
    fn test_from_path_with_dir_path() -> Result<(), ConfigBuildError> {
        setup_test();
        let conf = ConfigBuilder::from_path(env!("CARGO_MANIFEST_DIR"))?.build()?;
        assert_eq!(conf.package_name, env!("CARGO_PKG_NAME"));
        assert_eq!(conf.manifest_dir, PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        assert_eq!(conf.cargo_path, PathBuf::from("cargo"));

        Ok(())
    }
}
