// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::error::Error;
use std::fmt;

/// Error union representing errors that might occur during unpacking of license data.
#[derive(Debug)]
pub enum UnpackError {
    DecompressError(lz4_flex::block::DecompressError),
    DecodeError(nanoserde::DeBinErr),
    /// The supplied byte array is empty.
    Empty,
}

impl From<lz4_flex::block::DecompressError> for UnpackError {
    fn from(value: lz4_flex::block::DecompressError) -> Self {
        Self::DecompressError(value)
    }
}

impl From<nanoserde::DeBinErr> for UnpackError {
    fn from(value: nanoserde::DeBinErr) -> Self {
        Self::DecodeError(value)
    }
}

// TODO: should display trait be included in the coverage?
#[cfg_attr(coverage_nightly, coverage(off))]
impl fmt::Display for UnpackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DecompressError(e) => writeln!(f, "{e}"),
            Self::DecodeError(e) => writeln!(f, "{e}"),
            Self::Empty => writeln!(f, "Supplied buffer is empty."),
        }
    }
}

impl Error for UnpackError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::DecompressError(e) => Some(e),
            Self::DecodeError(e) => Some(e),
            Self::Empty => None,
        }
    }
}
