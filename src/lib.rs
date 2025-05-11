// Copyright Adam McKellar 2024, 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Fetch licenses of dependencies at build time and embed them into your program.
//!
//! `license-fetcher` is a crate for fetching actual license texts from the cargo source directory for
//! crates that are compiled with your project. It does this in the build step
//! in a build script. This means that the heavy dependencies of `license-fetcher`
//! aren't your dependencies!
//!
//! ## Example
//!
//! Import `license-fetcher` as a normal AND as a build dependency:
//! ```sh
//! cargo add --build --features build license-fetcher
//! cargo add license-fetcher
//! ```
//!
//!
//! `src/main.rs`
//!
//! ```no_run
//! use license_fetcher::read_package_list_from_out_dir;
//! fn main() {
//!     let package_list = read_package_list_from_out_dir!().unwrap();
//! }
//! ```
//!
//!
//! `build.rs`
//!
//! ```
//! use license_fetcher::build::config::{ConfigBuilder, Config};
//! use license_fetcher::build::package_list_with_licenses;
//! use license_fetcher::PackageList;
//!
//! fn main() {
//!     // Config with environment variables set by cargo, to fetch licenses at build time.
//!     let config: Config = ConfigBuilder::from_build_env()
//!         .build()
//!         .expect("Failed to build configuration.");
//!
//!     let packages: PackageList = package_list_with_licenses(config)
//!                                     .expect("Failed to fetch metadata or licenses.");
//!
//!     // Write packages to out dir to be embedded.
//!     packages.write_package_list_to_out_dir().expect("Failed to write package list.");
//!
//!     // Rerun only if one of the following files changed:
//!     println!("cargo::rerun-if-changed=build.rs");
//!     println!("cargo::rerun-if-changed=Cargo.lock");
//!     println!("cargo::rerun-if-changed=Cargo.toml");
//! }
//! ```
//!
//! For a more advanced example visit the [`build` module documentation](crate::build).
//!
//! ## Adding Packages that are not Crates
//!
//! Sometimes we have dependencies that are not crates. For these dependencies `license-fetcher` cannot
//! automatically generate information. These dependencies can be added manually:
//!
//! ```
//! use std::fs::read_to_string;
//! use std::concat;
//!
//! use license_fetcher::build::config::{ConfigBuilder, Config};
//! use license_fetcher::build::metadata::package_list;
//! use license_fetcher::{PackageList, Package, package};
//!
//! fn main() {
//!     // Config with environment variables set by cargo, to fetch licenses at build time.
//!     let config: Config = ConfigBuilder::from_build_env()
//!         .build()
//!         .expect("Failed to build configuration.");
//!
//!     // `packages` does not hold any licenses!
//!     let mut packages: PackageList = package_list(&config.metadata_config)
//!                                                 .expect("Failed to fetch metadata.");
//!
//!     packages.push(package! {
//!         name: "other dependency".to_owned(),
//!         version: "0.1.0".to_owned(),
//!         authors: vec!["Me".to_owned()],
//!         description: Some("A dependency that is not a rust crate.".to_owned()),
//!         homepage: None,
//!         repository: None,
//!         license_identifier: None,
//!         license_text: Some(
//!             read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/LICENSE"))
//!             .expect("Failed reading license of other dependency")
//!         )
//!     });
//!
//!     // Write packages to out dir to be embedded.
//!     packages.write_package_list_to_out_dir().expect("Failed to write package list.");
//!
//!     // Rerun only if one of the following files changed:
//!     println!("cargo::rerun-if-changed=build.rs");
//!     println!("cargo::rerun-if-changed=Cargo.lock");
//!     println!("cargo::rerun-if-changed=Cargo.toml");
//! }
//! ```
//!

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
    #[doc(hidden)]
    pub restored_from_cache: bool,
    #[doc(hidden)]
    pub is_root_pkg: bool,
    #[doc(hidden)]
    pub name_version: String,
}

// TODO: Is there an alternative?
/// Construct a [Package].
#[macro_export]
macro_rules! package {
    (
        name: $name:expr,
        version: $version:expr,
        authors: $authors:expr,
        description: $description:expr,
        homepage: $homepage:expr,
        repository: $repository:expr,
        license_identifier: $license_identifier:expr,
        license_text: $license_text:expr $(,)?
    ) => {
        $crate::Package {
            name: $name.clone(),
            version: $version.clone(),
            authors: $authors,
            description: $description,
            homepage: $homepage,
            repository: $repository,
            license_identifier: $license_identifier,
            license_text: $license_text,
            restored_from_cache: false,
            is_root_pkg: false,
            name_version: format!("{}-{}", $name, $version),
        }
    };
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
        if bytes.is_empty() {
            return Err(UnpackError::Empty);
        }

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
/// If you get an error that `OUT_DIR` is not set, then please compile your project once and restart rust analyzer.
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

#[cfg(test)]
mod test {
    use std::fs::read_to_string;

    use super::*;

    #[test]
    fn test_package_macro() {
        let _pkg: Package = package! {
            name: "dependency".to_owned(),
            version: "0.1.0".to_owned(),
            authors: vec!["Me".to_owned()],
            description: Some("A dependency that is not a rust crate.".to_owned()),
            homepage: None,
            repository: None,
            license_identifier: None,
            license_text: Some(
                read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/LICENSE"))
                    .expect("Failed reading license of other dependency")
            )
        };
    }
}
