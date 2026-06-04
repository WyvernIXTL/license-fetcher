// Copyright Adam McKellar 2024, 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{error::Error, fmt};

/// Error that might occur during the writing process of the license data to the output file.
#[derive(Debug)]
pub enum WriteError {
    /// serialized and compressed license data should write to file
    Write(std::io::Error),
    /// should only be called if in build script
    NotBuildScript,
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Write(err) => write!(
                f,
                "serialized and compressed license data should write to file\n{err}"
            ),
            Self::NotBuildScript => write!(f, "should only be called if in build script"),
        }
    }
}

impl Error for WriteError {}
