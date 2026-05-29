// Copyright Adam McKellar 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{ffi::OsStr, fmt, path::PathBuf};

pub struct ConfigBuildError {
    pub message: String,
    pub path_maybe: Option<PathBuf>,
    pub env_maybe: Option<String>,
}

impl fmt::Display for ConfigBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(path) = &self.path_maybe {}
    }
}

impl ConfigBuildError {
    pub(super) fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            path_maybe: None,
            env_maybe: None,
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
}
