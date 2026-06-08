// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{error::Error, fmt};

/// Error union representing errors that might occur during unpacking of embedded license data.
#[derive(Debug)]
pub enum UnpackError {
    /// failed decompressing embedded license data
    DecompressError(lz4_flex::block::DecompressError),
    /// failed deserialization of embedded decompressed license data
    DecodeError(nanoserde::DeBinErr),
    /// embedded license data is empty
    ///
    /// This error occurs, when the file being decompressed and deserialized is empty / has zero bytes.
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

impl fmt::Display for UnpackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DecompressError(_) => write!(f, "failed decompressing embedded license data"),
            Self::DecodeError(_) => write!(
                f,
                "failed deserialization of embedded decompressed license data"
            ),
            Self::Empty => write!(f, "embedded license data is empty"),
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
