// Copyright Adam McKellar 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{ffi::OsStr, fmt, path::PathBuf};

use error_stack::{Context, Report};

#[derive(Debug, Clone)]
pub struct CPath(pub PathBuf);

impl<T: AsRef<OsStr>> From<T> for CPath {
    fn from(value: T) -> Self {
        Self(value.as_ref().into())
    }
}

impl fmt::Display for CPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Path: {}", self.0.to_string_lossy())
    }
}

#[derive(Debug, Clone)]
pub struct CEnvVar(pub String);

impl<T: AsRef<OsStr>> From<T> for CEnvVar {
    fn from(value: T) -> Self {
        Self(value.as_ref().to_string_lossy().to_string())
    }
}

impl fmt::Display for CEnvVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Environment Variable: {}", self.0)
    }
}

#[derive(Debug)]
pub(crate) struct ReportJoin<E: Context> {
    error: Result<(), Report<E>>,
}

impl<E> ReportJoin<E>
where
    E: Context,
{
    pub fn result(self) -> Result<(), Report<E>> {
        self.error
    }

    pub fn join(&mut self, e: Report<E>) {
        match self.error.as_mut() {
            Ok(_) => self.error = Err(e),
            Err(error) => error.extend_one(e),
        }
    }
}

impl<E> Default for ReportJoin<E>
where
    E: Context,
{
    fn default() -> Self {
        Self { error: Ok(()) }
    }
}
