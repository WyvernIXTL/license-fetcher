use std::fs::{read_dir, read_to_string};

use error_stack::{ensure, report};
use error_stack::{Result, ResultExt};
use log::error;
use serde::Deserialize;
use thiserror::Error;

use super::*;

/// Error that appears during failed build of config via [ConfigBuilder::from_toml()].
#[derive(Debug, Error)]
pub enum FromTomlError {
    #[error("The requested path does not exist or this program does not have the permission to access it.")]
    PathDoesNotExist,
    #[error("Manifest not found.")]
    ManifestNotFound,
    #[error("Failure during IO operation.")]
    Io(#[from] std::io::Error),
    #[error("Failure parsing 'Cargo.toml'.")]
    TomlParseError(#[from] toml::de::Error),
    #[error("Manifest found but parent path not. This might imply that your manifest is at the root '/' or 'C:/'.")]
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

fn valid_cargo_toml_path(uncertain_file_path: PathBuf) -> Result<PathBuf, FromTomlError> {
    debug_assert!(uncertain_file_path.is_file());

    ensure!(
        uncertain_file_path
            .file_name()
            .ok_or(FromTomlError::ManifestNotFound)
            .attach_printable_lazy(|| "Path to file provided has an invalid file name.")
            .inspect_err(|e| error!("{}", e))?
            == "Cargo.toml",
        FromTomlError::ManifestNotFound
    );

    Ok(uncertain_file_path)
}

fn find_valid_cargo_toml_path(uncertain_dir_path: PathBuf) -> Result<PathBuf, FromTomlError> {
    debug_assert!(uncertain_dir_path.is_dir());

    read_dir(&uncertain_dir_path)
        .map_err(|e| FromTomlError::from(e))
        .attach_printable_lazy(|| {
            format!(
                "Error during reading of directory: '{:?}'",
                &uncertain_dir_path
            )
        })?
        .filter_map(|e| e.ok())
        .find(|e| e.file_type().map_or(false, |e| e.is_file()) && e.file_name() == "Cargo.toml")
        .map(|e| e.path())
        .ok_or(FromTomlError::ManifestNotFound.into())
}

fn manifest_file_path(uncertain_path: PathBuf) -> Result<PathBuf, FromTomlError> {
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
    pub fn from_path(manifest_path: impl Into<PathBuf>) -> Result<Self, FromTomlError> {
        let manifest_path: PathBuf = manifest_path.into();

        ensure!(
            manifest_path.try_exists().map_err(FromTomlError::from)?,
            FromTomlError::PathDoesNotExist
        );

        let manifest_file_path = manifest_file_path(manifest_path)?;

        let cargo_toml: CargoToml = toml::from_str(
            &read_to_string(&manifest_file_path)
                .map_err(FromTomlError::from)
                .attach_printable_lazy(|| {
                    format!(
                        "Failed to read 'Cargo.toml' file at path: '{:?}'",
                        &manifest_file_path
                    )
                })?,
        )
        .map_err(FromTomlError::from)?;

        let package_name = cargo_toml.package.name;
        let manifest_dir = manifest_file_path
            .parent()
            .ok_or_else(|| FromTomlError::ManifestParentPathNotFound)?
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
    use super::*;

    #[test]
    fn test_from_path_with_file_path() -> Result<(), FromTomlError> {
        let conf = ConfigBuilder::from_path(env!("CARGO_MANIFEST_PATH"))?.build();
        assert_eq!(conf.package_name, env!("CARGO_PKG_NAME"));
        assert_eq!(conf.manifest_dir, PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        assert_eq!(conf.cargo_path, PathBuf::from("cargo"));

        Ok(())
    }

    #[test]
    fn test_from_path_with_dir_path() -> Result<(), FromTomlError> {
        let conf = ConfigBuilder::from_path(env!("CARGO_MANIFEST_DIR"))?.build();
        assert_eq!(conf.package_name, env!("CARGO_PKG_NAME"));
        assert_eq!(conf.manifest_dir, PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        assert_eq!(conf.cargo_path, PathBuf::from("cargo"));

        Ok(())
    }
}
