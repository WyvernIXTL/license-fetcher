//               Copyright Adam McKellar 2025
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use std::error::Error;
use std::fmt;

/// Error union representing errors that might occur during unpacking of license data.
#[derive(Debug)]
pub enum UnpackError {
    DecompressError(miniz_oxide::inflate::DecompressError),
    DecodeError(bincode::error::DecodeError),
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
        }
    }
}

impl Error for UnpackError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::DecompressError(e) => e,
            Self::DecodeError(e) => e,
        })
    }
}
