// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{ffi::OsStr, fmt, path::PathBuf};

use exn::{Exn, Frame};

#[derive(Debug, Clone, Default)]
pub(super) struct IE {
    msg: String,
    path_maybe: Option<PathBuf>,
    env_maybe: Option<String>,
    kind_maybe: Option<EK>,
}

impl fmt::Display for IE {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)?;
        if let Some(path) = &self.path_maybe {
            write!(f, " | path: \"{}\"", path.display())?;
        }
        if let Some(env_var) = &self.env_maybe {
            write!(f, " | env: \"{}\"", env_var)?;
        }
        Ok(())
    }
}

impl std::error::Error for IE {}

impl IE {
    pub(super) fn new(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            ..Self::default()
        }
    }

    pub(super) fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path_maybe = Some(path.into());
        self
    }

    pub(super) fn with_env(mut self, env_var: impl AsRef<OsStr>) -> Self {
        self.env_maybe = Some(env_var.as_ref().to_string_lossy().to_string());
        self
    }

    pub(super) fn with_kind(mut self, kind: EK) -> Self {
        self.kind_maybe = Some(kind);
        self
    }
}

/// The kind of error encountered when using `license-fetcher`.
#[derive(Debug, Clone, Copy)]
pub enum EK {
    /// The error is unrecoverable.
    Unrecoverable,
    /// The cache file should be a file, that exists, that can be read,
    /// that can be decompressed and deserialized into a [`PackageList`](crate::PackageList).
    ///
    /// To recover from this error, disable the use of the cache.
    /// ```
    /// # use crate::build::config::{Config, ConfigBuilder};
    /// # let your_config = ConfigBuilder::from_build_env().build().unwrap();
    /// let recovery_config: Config = Config {
    ///     cache_path: None,
    ///     ..your_config
    /// };
    /// ```
    Cache,
    /// The source registry folder should exist and be readable.
    ///
    /// There can be multiple causes:
    /// 1. The program does not have the permissions to read the folder.
    /// 2. The source registry folder does not exist.
    /// 3. The source registry folder is somewhere else.
    /// 4. The layout of the cargo home folder has changed.
    ///
    /// You could try to recover from this error by testing different common paths.
    /// ```
    /// # use crate::build::config::{Config, ConfigBuilder};
    /// # let your_config = ConfigBuilder::from_build_env().build().unwrap();
    /// let recovery_config: Config = Config {
    ///     // This folder is checked by default when using [`crate::build::config::ConfigBuilder`]
    ///     // if `CARGO_HOME` is not set.
    ///     cargo_home_dir: std::env::home_dir().unwrap().join(".cargo"),
    ///     ..your_config
    /// };
    /// ```
    RegistryFolder,
    /// Cargo should execute at all.
    ///
    /// This error is caused, when cargo cannot be executed.
    /// Maybe the path set is wrong or the program does not have the permissions to execute cargo.
    ///
    /// `ConfigBuilder` defaults to reading the `CARGO` environment variable and sets `cargo` as path on failure.
    /// *Maybe* setting the cargo path to `cargo` fixes the issue (I think, I actually encountered this once with `deps` in CI).
    /// ```
    /// # use std::path::PathBuf;
    /// # use crate::build::config::{Config, ConfigBuilder};
    /// # let your_config = ConfigBuilder::from_build_env().build().unwrap();
    /// let recovery_config: Config = Config {
    ///     metadata_config: {
    ///         cargo_path: PathBuf::from("cargo"),
    ///         ..your_config.metadata_config
    ///     },
    ///     ..your_config
    /// };
    /// ```
    CargoFailedExecution,
}

/// Error occurring when fetching licenses.
///
/// This error aims to be somewhat recoverable. The docs of [`EK`] (Error Kind) have some tips on recovery.
///
/// This error is likely returned being wrapped in [`Exn`]. `Exn` stores an human readable error chain with the module and lines attached, where the error stems from.
/// If you want to debug the error, I advise not to remove this wrapper.
#[derive(Debug, Clone)]
pub struct LicenseFetcherError {
    message: String,
    kind: EK,
}

impl fmt::Display for LicenseFetcherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LicenseFetcherError {}

impl LicenseFetcherError {
    fn find_next_non_generic(exn: &Exn<IE>) -> Option<(&IE, EK)> {
        fn walk(frame: &Frame) -> Option<(&IE, EK)> {
            if let Some(err) = frame.error().downcast_ref::<IE>() {
                if let Some(kind) = err.kind_maybe {
                    return Some((err, kind));
                }
            }
            frame.children().iter().find_map(walk)
        }

        walk(exn.frame())
    }

    pub(super) fn from_internal(err: Exn<IE>) -> Exn<LicenseFetcherError> {
        match Self::find_next_non_generic(&err) {
            Some((err_ref, kind)) => {
                let message = format!("{err_ref}");
                err.raise(LicenseFetcherError { message, kind })
            }
            None => {
                let message = err.msg.clone();
                err.raise(LicenseFetcherError {
                    message,
                    kind: EK::Unrecoverable,
                })
            }
        }
    }

    /// The kind of the error.
    ///
    /// See [`EK`] on the different kinds and how to handle them.
    pub fn kind(&self) -> EK {
        self.kind
    }

    /// The error message.
    ///
    /// This message is for humans and can change even in minor patches.
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug)]
pub(super) struct ErrorJoin {
    root_err: IE,
    errors: Vec<Exn<IE>>,
}

impl ErrorJoin {
    pub(super) fn new(root_err: IE) -> Self {
        Self {
            root_err,
            errors: vec![],
        }
    }

    pub(super) fn join(&mut self, e: impl Into<Exn<IE>>) {
        self.errors.push(e.into());
    }

    pub(super) fn result(self) -> Result<(), Exn<IE>> {
        if !self.errors.is_empty() {
            Err(Exn::raise_all(self.root_err, self.errors))
        } else {
            Ok(())
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub(super) fn err(self) -> Exn<IE> {
        if !self.errors.is_empty() {
            Exn::raise_all(self.root_err, self.errors)
        } else {
            Exn::new(IE::new("`ErrorJoin` should always be handled. `err` method was called even though join does not contain other errors."))
        }
    }
}
