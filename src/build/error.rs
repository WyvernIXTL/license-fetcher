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
pub(crate) struct ReportList<E: Context> {
    errors: Vec<Report<E>>,
}

impl<E> ReportList<E>
where
    E: Context,
{
    pub fn result(mut self) -> Result<(), Report<E>> {
        if self.errors.is_empty() {
            Ok(())
        } else if self.errors.len() == 1 {
            Err(self.errors.pop().unwrap())
        } else {
            let mut error = self.errors.pop().unwrap();
            for e in self.errors.into_iter() {
                error.extend_one(e);
            }
            Err(error)
        }
    }

    pub fn add(&mut self, e: Report<E>) {
        self.errors.push(e);
    }
}

impl<E> Default for ReportList<E>
where
    E: Context,
{
    fn default() -> Self {
        Self { errors: vec![] }
    }
}
