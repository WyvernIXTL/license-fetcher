// Copyright Adam McKellar 2024, 2025, 2026
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
//!     let packages: PackageList = package_list_with_licenses(&config)
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
//! use license_fetcher::{PackageList, Package};
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
//!     let other_dependency = Package::builder("other dependency", "0.1.0")
//!         .authors(vec!["Me".to_owned()])
//!         .description("A dependency that is not a rust crate.")
//!         .license_text(
//!             read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/LICENSE"))
//!             .expect("Failed reading license of other dependency")
//!         )
//!         .build();
//!
//!     packages.push(other_dependency);
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
//! ## Features
//!
//! - `serde` enables the derivation of `Serialize` and `Deserialize` for `Package` and `PackageList`.
//!

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(clippy::correctness, clippy::suspicious)]
#![warn(clippy::complexity, clippy::perf, clippy::style, clippy::cargo)]
#![warn(clippy::pedantic)]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![allow(clippy::missing_errors_doc)]

use std::cmp::Ordering;
use std::default::Default;
use std::fmt;
use std::ops::{Deref, DerefMut};

/// Wrapper around `bincode` and `miniz_oxide` errors during unpacking of a serialized and compressed [`PackageList`].
pub mod error;
use error::UnpackError;
use lz4_flex::decompress_size_prepended;
use nanoserde::DeBin;

/// Functions for fetching metadata and licenses.
#[cfg(feature = "build")]
pub mod build;

/// Information regarding a crate / package.
///
/// This struct holds information like package name, authors and of course license text.
#[derive(DeBin, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "build", derive(nanoserde::SerBin))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(test, derive(arbitrary::Arbitrary))]
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
    /// Returns a [`PackageBuilder`] for easy initialization of a package.
    ///
    /// ## Example
    ///
    /// ```
    /// let my_package: Package = Package::builder("test_package", "0.1.0")
    ///     .authors(vec!["Max Mustermann"])
    ///     .description("A test package.")
    ///     .homepage("https://codeberg.org/")
    ///     .repository("https://codeberg.org/")
    ///     .license_identifier("MPL-2.0")
    ///     .license_text("Mozilla Public License Version 2.0...")
    ///     .build();
    /// ```
    pub fn builder(name: impl Into<String>, version: impl Into<String>) -> PackageBuilder {
        PackageBuilder::new(name, version)
    }

    fn fmt_package(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const SEPARATOR_WIDTH: usize = 80;
        let separator: String = "=".repeat(SEPARATOR_WIDTH);
        let separator_light: String = "-".repeat(SEPARATOR_WIDTH);

        writeln!(f, "Package:     {} {}", self.name, self.version)?;
        if let Some(description) = &self.description {
            writeln!(f, "Description: {description}")?;
        }
        if !self.authors.is_empty() {
            writeln!(
                f,
                "Authors:     - {}",
                self.authors.first().unwrap_or(&String::new())
            )?;
            for author in self.authors.iter().skip(1) {
                writeln!(f, "             - {author}")?;
            }
        }
        if let Some(homepage) = &self.homepage {
            writeln!(f, "Homepage:    {homepage}")?;
        }
        if let Some(repository) = &self.repository {
            writeln!(f, "Repository:  {repository}")?;
        }
        if let Some(license_identifier) = &self.license_identifier {
            writeln!(f, "SPDX Ident:  {license_identifier}")?;
        }

        if let Some(license_text) = &self.license_text {
            writeln!(f, "\n{separator_light}\n{license_text}")?;
        }

        writeln!(f, "\n{separator}\n")?;

        Ok(())
    }
}

/// Very naive [Ord] implementation for [Package].
///
/// This implementation is very basic and just for returning the package list in a somewhat ordered state.
/// This order implementation does not take into consideration like alpha or beta release.
impl Ord for Package {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.name < other.name {
            Ordering::Less
        } else if self.name > other.name {
            Ordering::Greater
        } else if self.version < other.version {
            Ordering::Less
        } else if self.version > other.version {
            Ordering::Greater
        } else {
            Ordering::Equal
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

        writeln!(f, "{separator}\n")?;

        self.fmt_package(f)
    }
}

/// A builder for [`Package`].
///
/// ## Examples
///
/// Minimal example:
/// ```
/// let my_package: Package = Package::builder("test_package", "0.1.0")
///     .build();
/// ```
///
/// Declare everything:
/// ```
/// let my_package: Package = Package::builder("test_package", "0.1.0")
///     .authors(vec!["Max Mustermann"])
///     .description("A test package.")
///     .homepage("https://codeberg.org/")
///     .repository("https://codeberg.org/")
///     .license_identifier("MPL-2.0")
///     .license_text("Mozilla Public License Version 2.0...")
///     .build();
/// ```
pub struct PackageBuilder(Package);

impl PackageBuilder {
    /// Creates a new [`PackageBuilder`].
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        PackageBuilder(Package {
            name: name.into(),
            version: version.into(),
            authors: vec![],
            description: None,
            homepage: None,
            repository: None,
            license_identifier: None,
            license_text: None,
        })
    }

    pub fn authors(mut self, authors: impl Into<Vec<String>>) -> Self {
        self.0.authors = authors.into();
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.0.description = Some(description.into());
        self
    }

    pub fn homepage(mut self, homepage: impl Into<String>) -> Self {
        self.0.homepage = Some(homepage.into());
        self
    }

    pub fn repository(mut self, repository: impl Into<String>) -> Self {
        self.0.repository = Some(repository.into());
        self
    }

    pub fn license_identifier(mut self, license_identifier: impl Into<String>) -> Self {
        self.0.license_identifier = Some(license_identifier.into());
        self
    }

    pub fn license_text(mut self, license_text: impl Into<String>) -> Self {
        self.0.license_text = Some(license_text.into());
        self
    }

    pub fn build(self) -> Package {
        self.0
    }
}

/// Holds information of all crates and licenses used for a release build.
#[derive(DeBin, Debug, PartialEq, Eq, Clone, Default)]
#[cfg_attr(feature = "build", derive(nanoserde::SerBin))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(test, derive(arbitrary::Arbitrary))]
pub struct PackageList(pub Vec<Package>);

impl From<Vec<Package>> for PackageList {
    fn from(value: Vec<Package>) -> Self {
        Self(value)
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

        writeln!(f, "{separator}\n")?;

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

        let uncompressed_bytes = decompress_size_prepended(bytes)?;

        let package_list = PackageList::deserialize_bin(&uncompressed_bytes)?;

        Ok(package_list)
    }
}

/// Embed and decode a [`PackageList`], which you expect to be in `$OUT_DIR/LICENSE-3RD-PARTY.bincode.deflate`, via [`PackageList::from_encoded`].
///
/// This macro is only meant to be used in conjunction with [`PackageList::write_package_list_to_out_dir`].
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

/* -------------------------------------------------------------------------- */
/*                                 Unit Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod test {
    use arbtest::arbtest;
    use assert2::check;

    use super::*;

    #[test]
    fn test_package_builder_compiles() {
        let _pkg = Package::builder("dependency", "0.1.0")
            .authors(vec!["Me".to_owned()])
            .description("A dependency that is not a rust crate.")
            .license_text("Some random ass license")
            .build();
    }

    #[test]
    fn test_display_package_contains_inputs() {
        let test_package = Package::builder("test_package", "0.1.0")
            .authors(vec![
                "Max Mustermann".to_owned(),
                "Erika Mustermann".to_owned(),
            ])
            .description("Some weird ass test package.")
            .homepage("https://example.com")
            .repository("https://github.com/example/test_package")
            .license_identifier("MPL-2.0")
            .license_text("NaN")
            .build();

        let display = format!("{test_package}");

        check!(
            display.contains("test_package")
                && display.contains("0.1.0")
                && display.contains("Max Mustermann")
                && display.contains("Erika Mustermann")
                && display.contains("Some weird ass test package.")
                && display.contains("https://example.com")
                && display.contains("https://github.com/example/test_package")
                && display.contains("MPL-2.0")
        );
    }

    #[test]
    fn test_display_package_does_not_panic() {
        arbtest(|u| {
            let test_package: Package = u.arbitrary()?;
            let _ = format!("{test_package}");
            Ok(())
        });
    }

    #[test]
    fn test_display_package_list_does_not_panic() {
        arbtest(|u| {
            let test_package_list: PackageList = u.arbitrary()?;
            let _ = format!("{test_package_list}");
            Ok(())
        });
    }

    #[test]
    fn test_ord_trait_for_package() {
        let create_test_package = |name: &str, id: &str| {
            Package::builder(name, id)
                .authors(vec![
                    "Max Mustermann".to_owned(),
                    "Erika Mustermann".to_owned(),
                ])
                .description("Some weird ass test package.")
                .homepage("https://example.com")
                .repository("https://github.com/example/test_package")
                .license_identifier("MPL-2.0")
                .license_text("NaN")
                .build()
        };

        check!(
            create_test_package("test1", "0.1.0") <= create_test_package("test1", "0.1.0")
                && create_test_package("test1", "0.1.0") < create_test_package("test2", "0.1.0")
                && create_test_package("test1", "0.1.0") < create_test_package("test1", "0.1.1")
                && create_test_package("test1", "0.1.0") < create_test_package("test1", "1.1.0")
                && create_test_package("test1", "0.1.0") < create_test_package("test1", "10.0.0")
                && create_test_package("test1", "0.1.0") < create_test_package("test2", "0.0.0")
                && create_test_package("test2", "0.1.0") > create_test_package("test1", "0.1.0")
                && create_test_package("test1", "0.2.0") > create_test_package("test1", "0.1.0")
        );
    }

    #[test]
    fn test_ord_trait_for_package_does_not_panic() {
        arbtest(|u| {
            let p1: Package = u.arbitrary()?;
            let p2: Package = u.arbitrary()?;

            let _ = p1 <= p2;
            let _ = p1 < p2;
            let _ = p1 >= p2;
            let _ = p1 > p2;

            Ok(())
        });
    }
}
