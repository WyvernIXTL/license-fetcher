// Copyright Adam McKellar 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// This is a copy paste from crate::build::fetcher::error.
// I also tinkered with an implementation using generics,
// but that would have been terrible to use for users of this crate.
// The advantage compared to a unfied error is, that there is a separation.

use std::{
    fmt::{self, Write},
    path::PathBuf,
};

use exn::{Exn, Frame};

#[derive(Debug, Clone, Default)]
pub(super) struct Cie {
    msg: String,
    path_maybe: Option<PathBuf>,
    kind_maybe: Option<CEK>,
}

impl fmt::Display for Cie {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl std::error::Error for Cie {}

impl Cie {
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

    pub(super) fn with_kind(mut self, kind: CEK) -> Self {
        self.kind_maybe = Some(kind);
        self
    }
}

/// The kind of error encountered when using [`ConfigBuilder`](super::ConfigBuilder)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CEK {
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
    /// Path is invalid or does not point to folder with manifest or directly to manifest file.
    ///
    /// The [`ConfigBuilder::from_path`](super::ConfigBuilder::from_path) differentiates between folders and files.
    /// Files are checked for being named `Cargo.toml`. Folders are checked for containing a file named `Cargo.toml`.
    /// This error also occurs, when a file or folder cannot be shown to exist.
    FailedFromPath,
    /// Failed to infer path to cargo home folder.
    ///
    /// Either the users home directory cannot be correctly be determined, or the path does not point to a valid folder.
    ///
    /// You could handle this error my manually setting the path or by setting the `CARGO_HOME` environmental variable.
    CargoHome,
}

/// Error occurring when using [`ConfigBuilder`](super::ConfigBuilder)
///
/// This error aims to be somewhat recoverable. The docs of [`CEK`] (Config Error Kind) have some tips on recovery.
#[derive(Clone)]
pub struct ConfigBuilderError {
    /// Verbose message with error chain to root cause.
    pub message: String,
    /// Machine readable error enum.
    pub kind: CEK,
}

impl fmt::Display for ConfigBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl fmt::Debug for ConfigBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.kind == CEK::Unrecoverable {
            write!(
                f,
                "\n--- {{ UNRECOVERABLE CONFIG BUILDER ERROR }} ---\n{}",
                self.message
            )
        } else {
            write!(
                f,
                "\n--- {{ RECOVERABLE CONFIG BUILDER ERROR }} ---\n{}",
                self.message
            )
        }
    }
}

impl std::error::Error for ConfigBuilderError {}

fn get_error_kind(exn: &Exn<Cie>) -> CEK {
    fn walk(frame: &Frame) -> Option<CEK> {
        if let Some(err) = frame.error().downcast_ref::<Cie>()
            && let Some(kind) = err.kind_maybe
        {
            return Some(kind);
        }
        frame.children().iter().find_map(walk)
    }

    walk(exn.frame()).unwrap_or(CEK::Unrecoverable)
}

fn get_message(exn: &Exn<Cie>) -> String {
    fn collect_frames(report: &mut String, i: usize, frame: &Frame) {
        if i > 0 {
            report.push('\n');
        }
        writeln!(report, "{:>2}: Msg: {}", i, frame.error()).unwrap();
        if let Some(err) = frame.error().downcast_ref::<Cie>()
            && let Some(path) = &err.path_maybe
        {
            writeln!(report, "    Pth: {}", path.display()).unwrap();
        }
        writeln!(report, "    Loc: {}", frame.location()).unwrap();
        for child in frame.children() {
            collect_frames(report, i + 1, child);
        }
    }

    let mut message = String::new();
    collect_frames(&mut message, 0, exn.frame());
    message
}

impl ConfigBuilderError {
    #[allow(clippy::needless_pass_by_value)]
    pub(super) fn from_internal(err: Exn<Cie>) -> Self {
        Self {
            message: get_message(&err),
            kind: get_error_kind(&err),
        }
    }
}
