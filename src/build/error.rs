// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{ffi::OsStr, fmt, path::PathBuf};

use exn::Exn;

/// Encoded path for error handling.
#[derive(Debug, Clone)]
pub struct CPath(pub PathBuf);

impl<T: AsRef<OsStr>> From<T> for CPath {
    fn from(value: T) -> Self {
        Self(value.as_ref().into())
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl fmt::Display for CPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Path: {}", self.0.display())
    }
}

/// Encoded environment variable for error handling.
#[derive(Debug, Clone)]
pub struct CEnvVar(pub String);

impl<T: AsRef<OsStr>> From<T> for CEnvVar {
    fn from(value: T) -> Self {
        Self(value.as_ref().to_string_lossy().to_string())
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl fmt::Display for CEnvVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Environment Variable: {}", self.0)
    }
}

#[derive(Debug)]
pub(crate) struct ReportJoin<E: std::error::Error + Send + Sync + 'static> {
    root_err: E,
    errors: Vec<Exn<E>>,
}

impl<E> ReportJoin<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    pub fn new(root_err: E) -> Self {
        Self {
            root_err,
            errors: vec![],
        }
    }

    pub fn result(self) -> Result<(), Exn<E>> {
        if !self.errors.is_empty() {
            Err(Exn::raise_all(self.root_err, self.errors))
        } else {
            Ok(())
        }
    }

    pub fn join(&mut self, e: impl Into<Exn<E>>) {
        self.errors.push(e.into());
    }
}
