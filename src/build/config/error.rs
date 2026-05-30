// Copyright Adam McKellar 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// This is a copy paste from crate::build::fetcher::error.
// I also tinkered with an implementation using generics,
// but that would have been terrible to use for users of this crate.
// The advantage compared to a unfied error is, that there is a separation.

use std::{error::Error, fmt};

use exn::{Exn, Frame};

#[derive(Debug, Clone)]
pub(super) struct CIE(pub String);

impl fmt::Display for CIE {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed building config, {}", self.0)
    }
}

impl std::error::Error for CIE {}

/// The kind of error encountered when using [`ConfigBuilder`](super::ConfigBuilder)
#[derive(Debug, Clone, Copy)]
pub enum ConfigBuilderErrorKind {
    /// The error is unrecoverable.
    Unrecoverable,
    /// Build environment variables should have been set.
    ///
    /// This error kind is returned if `CARGO_MANIFEST_DIR` or `CARGO` environment variables are not set.
    /// This could imply that you are
    /// 1.  not in a build script
    /// 2.  or that your build system does not set
    ///     the required environment variables.
    ///     [Cargo should have set these variables for the build script.](https://doc.rust-lang.org/cargo/reference/environment-variables.html)
    ///
    /// To handle this error you could use [`ConfigBuilder::from_path`](super::ConfigBuilder::from_path).
    FailedFromEnvVars,
}

/// Error occurring when using [`ConfigBuilder`](super::ConfigBuilder)
///
/// This error aims to be somewhat recoverable. The docs of [`CEK`] (Config Error Kind) have some tips on recovery.
///
/// This error is always returned being wrapped in [`Exn`]. `Exn` stores an human readable error chain with the module and lines attached, where the error stems from.
/// If you want to debug the error, I advise you not to remove this wrapper.
#[derive(Debug, Clone)]
pub struct ConfigBuilderError {
    message: String,
    kind: ConfigBuilderErrorKind,
}

impl fmt::Display for ConfigBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ConfigBuilderError {}

impl ConfigBuilderError {
    fn find_error<T: Error + 'static>(exn: &Exn<impl Error + Send + Sync>) -> Option<&T> {
        fn walk<T: Error + 'static>(frame: &Frame) -> Option<&T> {
            if let Some(err) = frame.error().downcast_ref::<T>() {
                return Some(err);
            }
            frame.children().iter().find_map(walk::<T>)
        }

        walk(exn.frame())
    }

    pub(super) fn from_internal(err: Exn<CIE>) -> Exn<ConfigBuilderError> {
        todo!()
    }
}
