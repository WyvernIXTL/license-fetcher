use std::fs::{read_dir, read_to_string};

use error_stack::ensure;
use error_stack::{Result, ResultExt};
use serde::Deserialize;
use thiserror::Error;

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
    #[error("Manifest found but parent path not. This might imply that your manifest is at the root '/Cargo.toml' or 'C:/Cargo.toml'.")]
    ManifestParentPathNotFound,
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
            .attach_printable("Path to file provided has an invalid file name.")?
            == "Cargo.toml",
        FromPathError::ManifestNotFound
    );

    Ok(uncertain_file_path)
}

fn find_valid_cargo_toml_path(uncertain_dir_path: PathBuf) -> Result<PathBuf, FromPathError> {
    debug_assert!(uncertain_dir_path.is_dir());

    read_dir(&uncertain_dir_path)
        .change_context(FromPathError::Io)?
        .filter_map(|e| e.ok())
        .find(|e| e.file_type().map_or(false, |e| e.is_file()) && e.file_name() == "Cargo.toml")
        .map(|e| e.path())
        .ok_or(FromPathError::ManifestNotFound.into())
}

fn manifest_file_path(uncertain_path: PathBuf) -> Result<PathBuf, FromPathError> {
    if uncertain_path.is_file() {
        valid_cargo_toml_path(uncertain_path)
    } else {
        find_valid_cargo_toml_path(uncertain_path)
    }
}

impl ConfigBuilder {
    /// New builder with needed values being filled from a manifest (`Cargo.toml`).
    ///
    /// Expects either a path directly to the `Cargo.toml` file or to it's parent directory.
    pub fn from_path(manifest_path: impl Into<PathBuf>) -> Result<Self, FromPathError> {
        let manifest_path: PathBuf = manifest_path.into();

        ensure!(
            manifest_path
                .try_exists()
                .change_context(FromPathError::Io)?,
            FromPathError::PathDoesNotExist
        );

        let manifest_file_path = manifest_file_path(manifest_path)?;

        let cargo_toml: CargoToml =
            toml::from_str(&read_to_string(&manifest_file_path).change_context(FromPathError::Io)?)
                .change_context(FromPathError::TomlParseError)?;

        let package_name = cargo_toml.package.name;
        let manifest_dir = manifest_file_path
            .parent()
            .ok_or_else(|| FromPathError::ManifestParentPathNotFound)?
            .to_path_buf();

        Ok(ConfigBuilder::custom(
            package_name,
            manifest_dir,
            PathBuf::from("cargo"),
        ))
    }
}

#[cfg(test)]
mod test {
    use crate::build::debug::test_setup;

    use super::*;

    #[test]
    fn test_from_path_with_file_path() -> Result<(), FromPathError> {
        test_setup();
        let conf = ConfigBuilder::from_path(env!("CARGO_MANIFEST_PATH"))?.build();
        assert_eq!(conf.package_name, env!("CARGO_PKG_NAME"));
        assert_eq!(conf.manifest_dir, PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        assert_eq!(conf.cargo_path, PathBuf::from("cargo"));

        Ok(())
    }

    #[test]
    fn test_from_path_with_dir_path() -> Result<(), FromPathError> {
        test_setup();
        let conf = ConfigBuilder::from_path(env!("CARGO_MANIFEST_DIR"))?.build();
        assert_eq!(conf.package_name, env!("CARGO_PKG_NAME"));
        assert_eq!(conf.manifest_dir, PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        assert_eq!(conf.cargo_path, PathBuf::from("cargo"));

        Ok(())
    }
}
