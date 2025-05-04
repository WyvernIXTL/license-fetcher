// Copyright Adam McKellar 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../../../docs/build_config.md")]

use std::{env::var_os, ffi::OsString, fmt, ops::Deref, path::PathBuf};

use cargo_folder::cargo_folder;
use error_stack::{Report, Result, ResultExt};
use thiserror::Error;

pub mod from_env;
pub mod from_path;

mod cargo_folder;

/// Configures what backend is used for walking the registry source folder.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FetchBackend {
    /// Use functions provided by the rusts standard library.
    ///
    /// This is fairly performant and does not need an external dependency.
    #[default]
    Std,
}

/// Configures how Cargo [fetches metadata].
///
/// This configuration enum is meant to be used with [CargoDirectiveList].
///
/// [fetches metadata]: https://doc.rust-lang.org/cargo/commands/cargo-metadata.html#manifest-options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CargoDirective {
    /// Fetch metadata normally.
    Default,
    /// Fetch metadata with versions locked to `Cargo.toml`.
    Locked,
    /// Fetch metadata with versions locked and offline.
    Frozen,
}

impl Into<&'static str> for CargoDirective {
    fn into(self) -> &'static str {
        match self {
            Self::Default => "",
            Self::Locked => "--locked",
            Self::Frozen => "--frozen",
        }
    }
}

impl fmt::Display for CargoDirective {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let printable = match self {
            Self::Default => "default",
            Self::Locked => "locked",
            Self::Frozen => "frozen",
        };
        write!(f, "{}", printable)
    }
}

/// Configure how Cargo fetches metadata.
///
/// Each [CargoDirective] corresponds to one `cargo` command being called if the one prior failed.
/// This can be useful if you supply installation instructions that either set `--locked` or `--frozen`.
///
/// ### Examples
///
/// #### Default (not locked)
///
/// If your `README.md` states to install your program with:
/// ```sh
/// cargo install my-program
/// ```
/// and if your ci also builds your program without lock, then
/// ```
/// # use license_fetcher::build::config::CargoDirectiveList;
/// let cargo_directives = CargoDirectiveList::default();
/// ```
/// or
/// ```
/// # use license_fetcher::build::config::{CargoDirectiveList, CargoDirective};
/// let cargo_directives = CargoDirectiveList(vec![CargoDirective::Default]);
/// ```
/// is the right choice for you.
///
/// #### Locked
///
/// If you build your program in CI with `--locked` or `--frozen` and supply
/// installation instruction like:
/// ```sh
/// cargo install --locked my-program
/// ```
/// then be sure to set [CargoDirective::Locked] before [Default](CargoDirective::Default):
/// ```
/// # use license_fetcher::build::config::{CargoDirectiveList, CargoDirective};
/// let cargo_directives = CargoDirectiveList(vec![CargoDirective::Locked, CargoDirective::Default]);
/// ```
/// or
/// ```
/// # use license_fetcher::build::config::CargoDirectiveList;
/// let cargo_directives = CargoDirectiveList::prefer_locked();
/// ```
/// This results in `cargo metadata --locked` being called, and if it fails, `cargo metadata` without lock
/// being called.
///
/// If someone then installs your program with `cargo install`, there might be missing or wrong licensing
/// information.
///
#[derive(Debug, Clone)]
pub struct CargoDirectiveList(pub Vec<CargoDirective>);

impl Deref for CargoDirectiveList {
    type Target = Vec<CargoDirective>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for CargoDirectiveList {
    fn default() -> Self {
        CargoDirectiveList(vec![CargoDirective::Default])
    }
}

impl CargoDirectiveList {
    /// Shorthand for `CargoDirectiveList(vec![CargoDirective::Locked, CargoDirective::Default])`
    pub fn prefer_locked() -> Self {
        CargoDirectiveList(vec![CargoDirective::Locked, CargoDirective::Default])
    }
}

impl From<Vec<CargoDirective>> for CargoDirectiveList {
    fn from(value: Vec<CargoDirective>) -> Self {
        CargoDirectiveList(value)
    }
}

/// Struct to configure data that is needed to fetch metadata.
#[derive(Debug, Clone)]
pub struct MetadataConfig {
    /// Path to directory that holds the `Cargo.toml` of the project you wish to fetch the licenses for.
    pub manifest_dir: PathBuf,
    /// Optional path to `cargo`.
    pub cargo_path: PathBuf,
    /// Set Cargo directives for fetching metadata.
    pub cargo_directives: CargoDirectiveList,
    /// Set enabled features used when detecting package metadata.
    pub enabled_features: Option<OsString>,
}

/// Configures the fetching of the license texts.
#[derive(Debug, Clone)]
pub struct FetchConfig {
    // Optional cargo home directory path.
    ///
    /// By default cargo home is inferred from the `CARGO_HOME` environment variable, or if not set,
    /// the standard location at the users home folder `~/.cargo`.
    pub cargo_home_dir: PathBuf,
    /// Set the backend used for traversing the `~/.cargo/registry/src` folder and reading the license files.
    pub fetch_backend: FetchBackend,
    /// Enables cache during license fetching.
    ///
    /// Setting this will use the already fetched licenses from prior runs.
    pub cache: bool,
}

/// Struct to configure the behavior of the license fetching.
#[derive(Debug, Clone)]
pub struct Config {
    pub metadata_config: MetadataConfig,
    pub fetching_config: FetchConfig,
}

/// Builder for Config struct.
///
/// Default config for build scripts with cache:
/// ```
/// use license_fetcher::build::config::ConfigBuilder;
///
/// let config = ConfigBuilder::default()
///     .with_build_env()
///     .unwrap()
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone, Default)]
pub struct ConfigBuilder {
    manifest_dir: Option<PathBuf>,
    cargo_path: Option<PathBuf>,
    cargo_home_dir: Option<PathBuf>,
    fetch_backend: Option<FetchBackend>,
    cargo_directives: Option<CargoDirectiveList>,
    cache: Option<bool>,
    enabled_features: Option<OsString>,
}

impl ConfigBuilder {
    /// Sets the manifest directory path.
    pub fn manifest_dir(mut self, dir: PathBuf) -> Self {
        self.manifest_dir = Some(dir);
        self
    }

    /// Sets the cargo executable path.
    pub fn cargo_path(mut self, path: PathBuf) -> Self {
        self.cargo_path = Some(path);
        self
    }

    /// Sets the cargo home directory path
    pub fn cargo_home_dir(mut self, dir: PathBuf) -> Self {
        self.cargo_home_dir = Some(dir);
        self
    }

    /// Sets the fetch backend.
    pub fn fetch_backend(mut self, backend: FetchBackend) -> Self {
        self.fetch_backend = Some(backend);
        self
    }

    /// Sets the cargo directives.
    pub fn cargo_directives(mut self, directives: CargoDirectiveList) -> Self {
        self.cargo_directives = Some(directives);
        self
    }

    /// Enables cache during license fetching.
    ///
    /// Setting this will use the already fetched licenses from prior runs.
    ///
    ///  If not set, the builder defaults to a detection step with environment variables, that sets
    /// cache to `true` if this code is run inside a build script and `false` otherwise.
    ///
    /// [`CARGO_CFG_FEATURE`] is used.
    ///
    /// [`CARGO_CFG_FEATURE`]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
    pub fn cache(mut self, cache: bool) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Set features used explicitly.
    ///
    /// The format is a comma separated list of features described [here].
    ///
    /// If not set and inside a build script (`build.rs`), the builder defaults to features enabled via the [`CARGO_CFG_FEATURE`] environment variable.
    ///
    /// [`CARGO_CFG_FEATURE`]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
    /// [here]: https://doc.rust-lang.org/cargo/commands/cargo-metadata.html#feature-selection
    pub fn enabled_features(mut self, features: OsString) -> Self {
        self.enabled_features = Some(features);
        self
    }

    /// Builds the Config with all required fields.
    pub fn build(self) -> Result<Config, ConfigBuildError> {
        let metadata_config = MetadataConfig {
            manifest_dir: self.manifest_dir.ok_or_else(|| {
                Report::new(ConfigBuildError::UninitializedField)
                    .attach_printable("Field 'manifest_dir' is required but not set.")
            })?,
            cargo_path: self.cargo_path.unwrap_or_else(|| PathBuf::from("cargo")),
            cargo_directives: self.cargo_directives.unwrap_or_default(),
            enabled_features: self
                .enabled_features
                .or_else(|| var_os("CARGO_CFG_FEATURE")),
        };
        let fetching_config = FetchConfig {
            cargo_home_dir: match self.cargo_home_dir {
                Some(dir) => dir,
                None => cargo_folder().change_context(ConfigBuildError::CargoHomeDir)?,
            },
            fetch_backend: self.fetch_backend.unwrap_or_default(),

            cache: self
                .cache
                .unwrap_or_else(|| var_os("CARGO_CFG_FEATURE").is_some()),
        };

        Ok(Config {
            metadata_config,
            fetching_config,
        })
    }
}

#[derive(Debug, Error)]
pub enum ConfigBuildError {
    #[error("Required field in builder is not initialized.")]
    UninitializedField,
    #[error("Validation of input failed.")]
    ValidationError,
    #[error("Failed fetching required fields from build environment variables.")]
    FailedFromEnvVars,
    #[error("Failed fetching  required fields from manifest in path.")]
    FailedFromPath,
    #[error(
        "Failed inferring cargo home dir from environment variables or standard home dir location."
    )]
    CargoHomeDir,
}
