// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::error::Error;

/// Error union representing errors that might occur during unpacking of embedded license data.
#[derive(Debug, displaydoc::Display)]
pub enum UnpackError {
    /// failed decompressing embedded license data
    DecompressError(lz4_flex::block::DecompressError),
    /// failed deserialization of embedded decompressed license data
    DecodeError(nanoserde::DeBinErr),
    /// embedded license data is empty
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

impl Error for UnpackError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::DecompressError(e) => Some(e),
            Self::DecodeError(e) => Some(e),
            Self::Empty => None,
        }
    }
}
