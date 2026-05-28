// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{ffi::OsStr, fmt, path::PathBuf};

use exn::{Exn, Frame};

#[derive(Debug, Clone, Default)]
pub(crate) struct IE {
    msg: String,
    maybe_path: Option<PathBuf>,
    maybe_env: Option<String>,
    maybe_kind: Option<EK>,
}

impl fmt::Display for IE {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)?;
        if let Some(path) = &self.maybe_path {
            write!(f, " | {}", path.display())?;
        }
        if let Some(env_var) = &self.maybe_env {
            write!(f, " | {}", env_var)?;
        }
        Ok(())
    }
}

impl std::error::Error for IE {}

impl IE {
    pub(crate) fn new(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            ..Self::default()
        }
    }

    pub(crate) fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.maybe_path = Some(path.into());
        self
    }

    pub(crate) fn with_env(mut self, env_var: impl AsRef<OsStr>) -> Self {
        self.maybe_env = Some(env_var.as_ref().to_string_lossy().to_string());
        self
    }

    pub(crate) fn with_kind(mut self, kind: EK) -> Self {
        self.maybe_kind = Some(kind);
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EK {
    Unrecoverable,
    Cache,
}

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
                if let Some(kind) = err.maybe_kind {
                    return Some((err, kind));
                }
            }
            frame.children().iter().find_map(walk)
        }

        walk(exn.frame())
    }

    pub(crate) fn from_internal(err: Exn<IE>) -> Exn<LicenseFetcherError> {
        match Self::find_next_non_generic(&err) {
            Some((err_ref, kind)) => {
                let message = err_ref.msg.clone();
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
}

#[derive(Debug)]
pub(crate) struct ReportJoin {
    root_err: IE,
    errors: Vec<Exn<IE>>,
}

impl ReportJoin {
    pub fn new(root_err: IE) -> Self {
        Self {
            root_err,
            errors: vec![],
        }
    }

    pub fn result(self) -> Result<(), Exn<IE>> {
        if !self.errors.is_empty() {
            Err(Exn::raise_all(self.root_err, self.errors))
        } else {
            Ok(())
        }
    }

    pub fn join(&mut self, e: impl Into<Exn<IE>>) {
        self.errors.push(e.into());
    }
}
