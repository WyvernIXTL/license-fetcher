//               Copyright Adam McKellar 2024, 2025
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

#![doc = include_str!("../docs/lib.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

use std::cmp::Ordering;
use std::default::Default;
use std::fmt;
use std::ops::{Deref, DerefMut};

use bincode::{config, Decode, Encode};

#[cfg(feature = "compress")]
use miniz_oxide::inflate::decompress_to_vec;

/// Wrapper around `bincode` and `miniz_oxide` errors during unpacking of a serialized and compressed [PackageList].
pub mod error;
use error::UnpackError;

/// Functions for fetching metadata and licenses.
#[cfg(feature = "build")]
pub mod build_script;

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
#[derive(Encode, Decode, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "build", derive(serde::Serialize))]
pub struct PackageList(pub Vec<Package>);

impl From<Vec<Package>> for PackageList {
    fn from(value: Vec<Package>) -> Self {
        PackageList(value)
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

/// Decompresses and deserializes the crate and license information.
///
/// This function decompresses (`compress` feature) and then deserializes the supplied bytes.
/// The the the supplied bytes should be the embedded license information from
/// the build step, else this function is going to panic.
///
/// ## Example
/// Called from within main program:
/// ```no_run
/// use license_fetcher::get_package_list;
/// fn main() {
///     let package_list = get_package_list(
///                             std::include_bytes!(
///                                 std::concat!(env!("OUT_DIR"), "/LICENSE-3RD-PARTY.bincode")
///                             )
///                         ).unwrap();
/// }
/// ```
pub fn get_package_list(bytes: &[u8]) -> Result<PackageList, UnpackError> {
    #[cfg(feature = "compress")]
    let uncompressed_bytes = decompress_to_vec(bytes).expect("Failed decompressing license data.");
    #[cfg(not(feature = "compress"))]
    let uncompressed_bytes = bytes;

    let (package_list, _) =
        bincode::decode_from_slice(&uncompressed_bytes[..], config::standard())?;

    Ok(package_list)
}

/// Calls [get_package_list] with parameters expected from a call from `main.rs`.
///
/// ## Example
/// ```no_run
/// use license_fetcher::get_package_list_macro;
/// fn main() {
///     let package_list = get_package_list_macro!();
/// }
/// ```
#[macro_export]
macro_rules! get_package_list_macro {
    () => {
        license_fetcher::get_package_list(std::include_bytes!(std::concat!(
            env!("OUT_DIR"),
            "/LICENSE-3RD-PARTY.bincode"
        )))
    };
}
