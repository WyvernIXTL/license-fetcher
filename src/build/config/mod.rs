//              Copyright Adam McKellar 2025
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

#![doc = include_str!("../../../docs/build_config.md")]

use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use cargo_folder::cargo_folder;
use derive_builder::{Builder, UninitializedFieldError};
use error_stack::{Report, ResultExt};
use thiserror::Error;

pub mod from_env;

#[cfg(feature = "config_from_path")]
pub mod from_path;

mod cargo_folder;

/// Configures what backend is used for walking the registry source folder.
#[derive(Debug, Clone, Copy, Default)]
pub enum FetchBackend {
    /// Use functions provided by the rusts standard library.
    ///
    /// This is fairly performant and does not need an external dependency.
    #[default]
    Std,
}

/// Configures what type of cache is used.
#[derive(Debug, Clone, Copy, Default)]
pub enum CacheBackend {
    /// Serialize and compress to file.
    ///
    /// Use the default naive approach of saving all the cached licenses at once
    /// and reading the all again at the next build step.
    ///
    /// This approach brings the advantage of not pulling in more dependencies.
    #[default]
    BincodeZip,
}

/// Configure where the cache is saved.
#[derive(Debug, Clone, Copy, Default)]
pub enum CacheSaveLocation {
    /// Save the cache in a global cache.
    ///
    /// This results in a good performance, when using `license-fetcher` in many projects.
    ///
    /// Uses [ProjectDirs::cache_dir](directories::ProjectDirs::cache_dir) for the location.
    /// When compiling multiple projects at the same time and a [CacheBackend] is used,
    /// that does not support concurrent reads and writes, then there might be some minor waiting
    /// on file locks or some entries might be missing in the cache, as it was overwritten.
    #[default]
    Global,
    /// Uses the [`OUT_DIR`] for caching.
    ///
    /// Panics if [`OUT_DIR`] is not set!
    ///
    /// This should only be used in the context of fetching licenses during the building step and embedding them into your program.
    ///
    /// [`OUT_DIR`]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
    Local,
    /// Writes the cache into [`.license-fetcher/CARGO_MANIFEST_DIR`].
    ///
    /// Panics if [`CARGO_MANIFEST_DIR`] is not set!
    ///
    /// This is very useful if you wish to supply this cache with your sources. This then guarantees that
    /// builds never fail due errors during license fetching like `cargo` not being in path, or not having permissions to read the `~/.cargo` folder,
    /// or a file erroring and one of the many unwraps being called. That is if the cache was build with every operating system you are targeting.
    ///
    /// **Be sure to track said directory with [`git lfs`](https://git-lfs.com/)!**
    ///
    /// [`CARGO_MANIFEST_DIR`]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
    /// [`.license-fetcher/CARGO_MANIFEST_DIR`]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
    Repository,
    /// Disables writing cache.
    None,
}

/// Configures how the cache behaves during fetching.
#[derive(Debug, Clone, Copy, Default)]
pub enum CacheBehavior {
    /// The first cache that is found is used.
    ///
    /// The following order applies for the search:
    /// 1. [Repository](CacheSaveLocation::Repository) *(only if `CARGO_MANIFEST_DIR` env var is set)*
    /// 2. [Local](CacheSaveLocation::Local) *(only if `OUT_DIR` env var is set)*
    /// 3. [Global](CacheSaveLocation::Global)
    #[default]
    CheckAllTakeFirst,
    /// Checks only global cache.
    ///
    /// Useful if you do not intend to fetch licenses during a build step.
    Global,
    /// Checking for cache is disabled.
    Disabled,
}

/// Configures how Cargo [fetches metadata].
///
/// This configuration enum is meant to be used with [CargoDirectiveList].
///
/// [fetches metadata]: https://doc.rust-lang.org/cargo/commands/cargo-metadata.html#manifest-options
#[derive(Debug, Clone, Copy)]
pub enum CargoDirective {
    /// Fetch metadata normally.
    Default,
    /// Fetch metadata with versions locked to `Cargo.toml`.
    Locked,
    /// Fetch metadata with versions locked and offline.
    Frozen,
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

/// Struct to configure the behavior of the license fetching.
///
/// It is recommended to create this struct via [ConfigBuilder]:
/// ```
/// use license_fetcher::build::config::ConfigBuilder;
///
/// let config = ConfigBuilder::default()
///     .with_build_env()
///     .expect("Failed fetching build env metadata for config build.")
///     .build()
///     .expect("Failed validating config build.");
///
/// assert_eq!(config.package_name, "license-fetcher".to_owned());
/// ```
#[derive(Debug, Clone, Builder)]
#[builder(pattern = "owned")]
#[builder(build_fn(error = "ConfigBuildReport"))]
#[builder(derive(Debug, Clone))]
pub struct Config {
    /// Name (underscore name / module name) of the package that you are fetching licenses for.
    pub package_name: String,
    /// Path to directory that holds the `Cargo.toml` of the project you wish to fetch the licenses for.
    pub manifest_dir: PathBuf,
    /// Optional path to `cargo`.
    #[builder(default=PathBuf::from("cargo"))]
    pub cargo_path: PathBuf,
    /// Optional cargo home directory path.
    ///
    /// By default cargo home is inferred from the `CARGO_HOME` environment variable, or if not set,
    /// the standard location at the users home folder `~/.cargo`.
    #[builder(default=cargo_folder().change_context(ConfigBuildError::CargoHomeDir)?)]
    pub cargo_home_dir: PathBuf,
    /// Set the backend used for traversing the `~/.cargo/registry/src` folder and reading the license files.
    #[builder(default)]
    pub fetch_backend: FetchBackend,
    /// Set the cache type.
    #[builder(default)]
    pub cache_backend: CacheBackend,
    /// Set the location where the cache should be saved to.
    #[builder(default)]
    pub cache_save_location: CacheSaveLocation,
    /// Set Cargo directives for fetching metadata.
    #[builder(default)]
    pub cargo_directives: CargoDirectiveList,
    /// Set cache behavior during fetching.
    #[builder(default)]
    pub cache_behavior: CacheBehavior,
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

#[derive(Debug)]
pub struct ConfigBuildReport(pub Report<ConfigBuildError>);

impl Deref for ConfigBuildReport {
    type Target = Report<ConfigBuildError>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ConfigBuildReport {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<UninitializedFieldError> for ConfigBuildReport {
    fn from(value: UninitializedFieldError) -> Self {
        Report::new(value)
            .change_context(ConfigBuildError::UninitializedField)
            .into()
    }
}

impl From<Report<ConfigBuildError>> for ConfigBuildReport {
    fn from(value: Report<ConfigBuildError>) -> Self {
        ConfigBuildReport(value)
    }
}
