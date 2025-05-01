// Copyright Adam McKellar 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{ffi::OsStr, fmt, path::PathBuf};

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
