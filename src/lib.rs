// Copyright Adam McKellar 2024, 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../docs/lib.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

use std::cmp::Ordering;
use std::default::Default;
use std::fmt;
use std::ops::{Deref, DerefMut};

use bincode::{Decode, Encode};

use miniz_oxide::inflate::decompress_to_vec;

/// Wrapper around `bincode` and `miniz_oxide` errors during unpacking of a serialized and compressed [PackageList].
pub mod error;
use error::UnpackError;

/// Functions for fetching metadata and licenses.
#[cfg(feature = "build")]
pub mod build;

/// Information regarding a crate / package.
///
/// This struct holds information like package name, authors and of course license text.
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "build", derive(serde::Serialize))]
pub struct Package {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license_identifier: Option<String>,
    pub license_text: Option<String>,
    restored_from_cache: bool,
    is_root_pkg: bool,
    name_version: String,
}

impl Package {
    fn fmt_package(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const SEPARATOR_WIDTH: usize = 80;
        let separator: String = "=".repeat(SEPARATOR_WIDTH);
        let separator_light: String = "-".repeat(SEPARATOR_WIDTH);

        writeln!(f, "Package:     {} {}", self.name, self.version)?;
        if let Some(description) = &self.description {
            writeln!(f, "Description: {}", description)?;
        }
        if !self.authors.is_empty() {
            writeln!(
                f,
                "Authors:     - {}",
                self.authors.get(0).unwrap_or(&"".to_owned())
            )?;
            for author in self.authors.iter().skip(1) {
                writeln!(f, "             - {}", author)?;
            }
        }
        if let Some(homepage) = &self.homepage {
            writeln!(f, "Homepage:    {}", homepage)?;
        }
        if let Some(repository) = &self.repository {
            writeln!(f, "Repository:  {}", repository)?;
        }
        if let Some(license_identifier) = &self.license_identifier {
            writeln!(f, "SPDX Ident:  {}", license_identifier)?;
        }

        if let Some(license_text) = &self.license_text {
            writeln!(f, "\n{}\n{}", separator_light, license_text)?;
        }

        writeln!(f, "\n{}\n", separator)?;

        Ok(())
    }
}

impl Ord for Package {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.name < other.name {
            Ordering::Less
        } else if self.name > other.name {
            Ordering::Greater
        } else {
            if self.version < other.version {
                Ordering::Less
            } else if self.version > other.version {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }
    }
}

impl PartialOrd for Package {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const SEPARATOR_WIDTH: usize = 80;
        let separator: String = "=".repeat(SEPARATOR_WIDTH);

        writeln!(f, "{}\n", separator)?;

        self.fmt_package(f)
    }
}

/// Holds information of all crates and licenses used for a release build.
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "build", derive(serde::Serialize))]
pub struct PackageList(pub Vec<Package>);

impl From<Vec<Package>> for PackageList {
    fn from(value: Vec<Package>) -> Self {
        Self(value)
    }
}

impl Default for PackageList {
    fn default() -> Self {
        PackageList(vec![])
    }
}

impl Deref for PackageList {
    type Target = Vec<Package>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PackageList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl fmt::Display for PackageList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const SEPARATOR_WIDTH: usize = 80;
        let separator: String = "=".repeat(SEPARATOR_WIDTH);

        writeln!(f, "{}\n", separator)?;

        for package in self.iter() {
            package.fmt_package(f)?;
        }

        Ok(())
    }
}

impl PackageList {
    /// Decompresses and deserializes the crate and license information.
    ///
    /// ## Example
    /// If you intend to embed license information:
    /// ```no_run
    /// use license_fetcher::PackageList;
    /// fn main() {
    ///     let package_list = PackageList::from_encoded(std::include_bytes!(std::concat!(
    ///        env!("OUT_DIR"),
    ///        "/LICENSE-3RD-PARTY.bincode.deflate"
    ///     ))).unwrap();
    /// }
    /// ```
    pub fn from_encoded(bytes: &[u8]) -> Result<PackageList, UnpackError> {
        let uncompressed_bytes = decompress_to_vec(bytes)?;

        let (package_list, _) =
            bincode::decode_from_slice(&uncompressed_bytes, bincode::config::standard())?;

        Ok(package_list)
    }
}

/// Embed and decode a [PackageList], which you expect to be in `$OUT_DIR/LICENSE-3RD-PARTY.bincode.deflate`, via [PackageList::from_encoded].
///
/// This macro is only meant to be used in conjunction with [PackageList::write_package_list_to_out_dir].
///
/// ## Example
/// ```no_run
/// use license_fetcher::read_package_list_from_out_dir;
/// fn main() {
///     let package_list = read_package_list_from_out_dir!().expect("Failed to decode the embedded package list.");
/// }
/// ```
#[macro_export]
macro_rules! read_package_list_from_out_dir {
    () => {
        license_fetcher::PackageList::from_encoded(std::include_bytes!(std::concat!(
            env!("OUT_DIR"),
            "/LICENSE-3RD-PARTY.bincode.deflate"
        )))
    };
}
