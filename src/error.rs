// Copyright Adam McKellar 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::error::Error;
use std::fmt;

/// Error union representing errors that might occur during unpacking of license data.
#[derive(Debug)]
pub enum UnpackError {
    DecompressError(miniz_oxide::inflate::DecompressError),
    DecodeError(bincode::error::DecodeError),
    /// The supplied byte array is empty.
    Empty,
}

impl From<miniz_oxide::inflate::DecompressError> for UnpackError {
    fn from(value: miniz_oxide::inflate::DecompressError) -> Self {
        Self::DecompressError(value)
    }
}

impl From<bincode::error::DecodeError> for UnpackError {
    fn from(value: bincode::error::DecodeError) -> Self {
        Self::DecodeError(value)
    }
}

impl fmt::Display for UnpackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DecompressError(e) => writeln!(f, "{}", e),
            Self::DecodeError(e) => writeln!(f, "{}", e),
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
