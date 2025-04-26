//              Copyright Adam McKellar 2025
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! This module holds the structs and enums to configure the fetching process.
//!
//! ## Examples
//!
//! TODO
use log::error;
use snafu::{Backtrace, OptionExt, ResultExt, Snafu};
use std::{
    env::{var, var_os},
    ffi::OsStr,
    ops::Deref,
    path::PathBuf,
};

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
/// It is recommended to create this struct via [ConfigBuilder].
#[derive(Debug, Clone)]
pub struct Config {
    /// Name (underscore name / module name) of the package that you are fetching licenses for.
    pub package_name: String,
    /// Path to directory that holds the `Cargo.toml` of the project you wish to fetch the licenses for.
    pub manifest_dir: PathBuf,
    /// Optional path to `cargo`.
    pub cargo_path: PathBuf,
    /// Set the backend used for traversing the `~/.cargo/registry/src` folder and reading the license files.
    pub fetch_backend: FetchBackend,
    /// Set the cache type.
    pub cache_backend: CacheBackend,
    /// Set the location where the cache should be saved to.
    pub cache_save_location: CacheSaveLocation,
    /// Set Cargo directives for fetching metadata.
    pub cargo_directives: CargoDirectiveList,
    /// Set cache behavior during fetching.
    pub cache_behavior: CacheBehavior,
}

/// Builder for [Config].
///
/// Use this builder to construct a [Config] struct with various options.
/// You can initialize the builder with required values using [ConfigBuilder::custom]
/// or populate them from environment variables using [ConfigBuilder::from_env].
pub struct ConfigBuilder {
    package_name: String,
    manifest_dir: PathBuf,
    cargo_path: PathBuf,
    fetch_backend: Option<FetchBackend>,
    cache_backend: Option<CacheBackend>,
    cache_save_location: Option<CacheSaveLocation>,
    cargo_directives: Option<CargoDirectiveList>,
    cache_behavior: Option<CacheBehavior>,
}

impl ConfigBuilder {
    /// New builder with needed values being filled in from environment variables.
    ///
    /// This constructor is meant to be used from a build script (`build.rs`)!
    /// The environment variables used are set by cargo during build.
    pub fn from_env() -> Result<Self, ConfigBuilderEnvError> {
        let package_name = string_from_env("CARGO_PKG_NAME")?;
        let manifest_dir = path_buf_from_env("CARGO_MANIFEST_DIR")?;
        let cargo_path = path_buf_from_env("CARGO")?;

        Ok(ConfigBuilder::custom(
            package_name,
            manifest_dir,
            cargo_path,
        ))
    }

    /// Creates a new builder with the required fields explicitly provided.
    pub fn custom(package_name: String, manifest_dir: PathBuf, cargo_path: PathBuf) -> Self {
        Self {
            package_name,
            manifest_dir,
            cargo_path,
            fetch_backend: None,
            cache_backend: None,
            cache_save_location: None,
            cargo_directives: None,
            cache_behavior: None,
        }
    }

    /// Set the backend used for traversing the `~/.cargo/registry/src` folder and reading the license files.
    pub fn fetch_backend(mut self, fetch_backend: FetchBackend) -> Self {
        self.fetch_backend = Some(fetch_backend);
        self
    }

    /// Set the cache type.
    pub fn cache_backend(mut self, cache_backend: CacheBackend) -> Self {
        self.cache_backend = Some(cache_backend);
        self
    }

    /// Set the location where the cache should be saved to.
    pub fn cache_save_location(mut self, cache_save_location: CacheSaveLocation) -> Self {
        self.cache_save_location = Some(cache_save_location);
        self
    }

    /// Set Cargo directives for fetching metadata.
    pub fn cargo_directives(mut self, cargo_directives: impl Into<CargoDirectiveList>) -> Self {
        self.cargo_directives = Some(cargo_directives.into());
        self
    }

    /// Set cache behavior during fetching.
    pub fn cache_behavior(mut self, cache_behavior: CacheBehavior) -> Self {
        self.cache_behavior = Some(cache_behavior);
        self
    }

    /// Builds the [Config] struct from the builder's current state.
    ///
    /// Default values will be used for any options that were not explicitly set.
    pub fn build(self) -> Config {
        Config {
            package_name: self.package_name,
            manifest_dir: self.manifest_dir,
            cargo_path: self.cargo_path,
            fetch_backend: self.fetch_backend.unwrap_or_default(),
            cache_backend: self.cache_backend.unwrap_or_default(),
            cache_save_location: self.cache_save_location.unwrap_or_default(),
            cargo_directives: self.cargo_directives.unwrap_or_default(),
            cache_behavior: self.cache_behavior.unwrap_or_default(),
        }
    }
}

/// Error that appears during failed build of config.
#[derive(Debug, Snafu)]
pub enum ConfigBuilderEnvError {
    /// Error that appears during execution of [ConfigBuilder::from_env()].
    ///
    /// This error might appear if this function is not called from a build script.
    /// Cargo sets during execution of the build script the needed environment variables.
    #[snafu(display(
        "Environment variable '{env_variable}' is not set. Was 'from_env()' not called from a build script ('build.rs')?"
    ))]
    EnvVarNotPresent {
        env_variable: String,
        backtrace: Backtrace,
    },
    /// Error that appears during execution of [ConfigBuilder::from_env()].
    #[snafu(display("Failure getting the environment variable '{env_variable}'."))]
    EnvVarError {
        source: std::env::VarError,
        env_variable: String,
        backtrace: Backtrace,
    },
}

fn path_buf_from_env(env: impl AsRef<OsStr>) -> Result<PathBuf, ConfigBuilderEnvError> {
    let env_value = var_os(&env)
        .with_context(|| EnvVarNotPresentSnafu {
            env_variable: env.as_ref().to_string_lossy(),
        })
        .inspect_err(|e| error!("{}", e))?;

    Ok(PathBuf::from(env_value))
}

fn string_from_env<K>(env: K) -> Result<String, ConfigBuilderEnvError>
where
    K: AsRef<OsStr>,
{
    let env_value = var(&env)
        .with_context(|_| EnvVarSnafu {
            env_variable: env.as_ref().to_string_lossy(),
        })
        .inspect_err(|e| error!("{}", e))?;

    Ok(env_value)
}

#[cfg(feature = "toml")]
pub mod parse_manifest {
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
                .find(|e| {
                    e.file_type().map_or(false, |e| e.is_file()) && e.file_name() == "Cargo.toml"
                })
                .with_context(|| ManifestNotFoundSnafu)?
                .path())
        }
    }

    impl ConfigBuilder {
        /// New builder with needed values being filled from a manifest (`Cargo.toml`).
        ///
        /// Expects either a path directly to the `Cargo.toml` file or to it's parent directory.
        pub fn from_toml(
            manifest_path: impl Into<PathBuf>,
        ) -> Result<Self, ConfigBuilderTomlError> {
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

            let cargo_toml: CargoToml = toml::from_str(
                &read_to_string(&manifest_file_path).with_context(|_| GenericIoSnafu)?,
            )
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
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[snafu::report]
    fn test_config_from_env() -> Result<(), ConfigBuilderEnvError> {
        let conf = ConfigBuilder::from_env()?.build();
        assert_eq!(conf.package_name, env!("CARGO_PKG_NAME"));
        assert_eq!(conf.manifest_dir, PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        assert_eq!(conf.cargo_path, PathBuf::from(env!("CARGO")));

        Ok(())
    }
}
