use std::fs::{read_dir, read_to_string};

use log::error;
use serde::Deserialize;
use snafu::ensure;

use super::*;

/// Error that appears during failed build of config via [ConfigBuilder::from_toml()].
#[derive(Debug, Snafu)]
pub enum ConfigBuilderTomlError {
    #[snafu(display(
            "Path '{}' does not exist or this program does not have the permission to access it.",
            path.display()
        ))]
    PathDoesNotExist {
        path: PathBuf,
        backtrace: Backtrace,
    },
    #[snafu(display("Manifest not found."))]
    ManifestNotFound {
        backtrace: Backtrace,
    },
    #[snafu(display("Failure during IO operation."))]
    GenericIoError {
        source: std::io::Error,
        backtrace: Backtrace,
    },
    #[snafu(display("Failure parsing 'Cargo.toml'."))]
    TomlParseError {
        source: toml::de::Error,
    },
    GenericError,
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

fn manifest_file_path(uncertain_path: &PathBuf) -> Result<PathBuf, ConfigBuilderTomlError> {
    if uncertain_path.is_file() {
        ensure!(
            uncertain_path
                .file_name()
                .with_context(|| ManifestNotFoundSnafu)
                .inspect_err(|e| error!("{}", e))?
                == "Cargo.toml",
            ManifestNotFoundSnafu
        );

        Ok(uncertain_path.clone())
    } else {
        debug_assert!(uncertain_path.is_dir());

        Ok(read_dir(&uncertain_path)
            .with_context(|_| GenericIoSnafu)?
            .filter_map(|e| e.ok())
            .find(|e| e.file_type().map_or(false, |e| e.is_file()) && e.file_name() == "Cargo.toml")
            .with_context(|| ManifestNotFoundSnafu)?
            .path())
    }
}

impl ConfigBuilder {
    /// New builder with needed values being filled from a manifest (`Cargo.toml`).
    ///
    /// Expects either a path directly to the `Cargo.toml` file or to it's parent directory.
    pub fn from_path(manifest_path: impl Into<PathBuf>) -> Result<Self, ConfigBuilderTomlError> {
        let manifest_path: PathBuf = manifest_path.into();

        ensure!(
            manifest_path
                .try_exists()
                .with_context(|_| GenericIoSnafu)?,
            {
                error!("Path does not exist: '{}'", manifest_path.display());
                PathDoesNotExistSnafu {
                    path: manifest_path.clone(),
                }
            }
        );

        let manifest_file_path = manifest_file_path(&manifest_path)?;

        let cargo_toml: CargoToml =
            toml::from_str(&read_to_string(&manifest_file_path).with_context(|_| GenericIoSnafu)?)
                .with_context(|_| TomlParseSnafu)?;

        let name = cargo_toml.package.name;

        Ok(ConfigBuilder::custom(
            name,
            manifest_file_path
                .parent()
                .with_context(|| GenericSnafu)?
                .to_path_buf(),
            PathBuf::from("cargo"),
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[snafu::report]
    fn test_from_path_with_file_path() -> Result<(), ConfigBuilderTomlError> {
        let conf = ConfigBuilder::from_path(env!("CARGO_MANIFEST_PATH"))?.build();
        assert_eq!(conf.package_name, env!("CARGO_PKG_NAME"));
        assert_eq!(conf.manifest_dir, PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        assert_eq!(conf.cargo_path, PathBuf::from("cargo"));

        Ok(())
    }

    #[test]
    #[snafu::report]
    fn test_from_path_with_dir_path() -> Result<(), ConfigBuilderTomlError> {
        let conf = ConfigBuilder::from_path(env!("CARGO_MANIFEST_DIR"))?.build();
        assert_eq!(conf.package_name, env!("CARGO_PKG_NAME"));
        assert_eq!(conf.manifest_dir, PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        assert_eq!(conf.cargo_path, PathBuf::from("cargo"));

        Ok(())
    }
}
