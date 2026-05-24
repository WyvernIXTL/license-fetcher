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
//! *For a more advanced example visit the [`build` module documentation](crate::build).*
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
//! use license_fetcher::build::package_list;
//! use license_fetcher::{PackageList, Package};
//!
//! fn main() {
//!     // Config with environment variables set by cargo, to fetch licenses at build time.
//!     let config: Config = ConfigBuilder::from_build_env()
//!         .build()
//!         .expect("Failed to build configuration.");
//!
//!     // `packages` does not hold any licenses!
//!     let mut packages: PackageList = package_list(&config)
//!         .expect("Failed to fetch metadata.");
//!
//!     let other_dependency = Package::builder("other dependency", "0.1.0")
//!         .add_author("Me")
//!         .description("A dependency that is not a rust crate.")
//!         .add_license_text(
//!             "other dependency license",
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

use error::UnpackError;
use lz4_flex::decompress_size_prepended;
use nanoserde::DeBin;

/// Wrapper around deserialization and decompression errors during unpacking of a serialized and compressed [`PackageList`].
pub mod error;

/// Functions for fetching metadata and licenses.
#[cfg(feature = "build")]
pub mod build;

/// The file name used for writing and reading the serialized package list.
pub const OUT_FILE_NAME: &str = "LICENSE-3RD-PARTY.nanoserde.lz4";

/// Struct holding information like package name, authors and of course license text.
///
/// ## Example
///
/// It is recommended to build instances of [`Package`] with the [`PackageBuilder`] builder via [`Package::builder()`].
///
/// ```
/// use license_fetcher::Package;
///
/// let my_package: Package = Package {
///     name: "test-package".to_owned(),
///     version: "0.1.0".to_owned(),
///     authors: vec!["Max Mustermann <max@example.com>".to_owned(), "Erika Mustermann".to_owned()],
///     description: Some("A test package.".to_owned()),
///     homepage: Some("https://codeberg.org/".to_owned()),
///     repository: Some("https://codeberg.org/".to_owned()),
///     license_identifier: Some("MPL-2.0".to_owned()),
///     license_texts: vec![
///         ("Mozilla Public License Version 2.0".to_owned(), "1. Definitions ... 2. License Grants and Conditions ...".to_owned()),
///         ("MIT License".to_owned(), "Permission is hereby granted, ...".to_owned())
///     ],
/// };
/// ```
///
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
    pub license_texts: Vec<(String, String)>,
}

impl Package {
    /// Returns a [`PackageBuilder`] for easy initialization of a package.
    ///
    /// ## Example
    ///
    /// ```
    /// use license_fetcher::Package;
    ///
    /// let my_package: Package = Package::builder("test-package", "0.1.0")
    ///     .add_author("Max Mustermann <max@example.com>")
    ///     .add_author("Erika Mustermann")
    ///     .description("A test package.")
    ///     .homepage("https://codeberg.org/")
    ///     .repository("https://codeberg.org/")
    ///     .license_identifier("MPL-2.0")
    ///     .add_license_text("Mozilla Public License Version 2.0", "1. Definitions ... 2. License Grants and Conditions ...")
    ///     .add_license_text("MIT License", "Permission is hereby granted, ...")
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

        for (lic_location, lic_text) in &self.license_texts {
            // TODO: Test and tune new license printing.

            // ? provisional implementation
            writeln!(
                f,
                "\n{separator_light}\n{lic_location}\n{separator_light}\n\n{lic_text}"
            )?;
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
/// use license_fetcher::Package;
///
/// let my_package: Package = Package::builder("test-package", "0.1.0")
///     .build();
///
/// assert_eq!(
///     my_package,
///     Package {
///         name: "test-package".to_owned(),
///         version: "0.1.0".to_owned(),
///         authors: vec![],
///         description: None,
///         homepage: None,
///         repository: None,
///         license_identifier: None,
///         license_texts: vec![],
///     }
/// );
/// ```
///
/// Declare everything:
/// ```
/// use license_fetcher::Package;
///
/// let my_package: Package = Package::builder("test-package", "0.1.0")
///     .add_author("Max Mustermann <max@example.com>")
///     .add_author("Erika Mustermann")
///     .description("A test package.")
///     .homepage("https://codeberg.org/")
///     .repository("https://codeberg.org/")
///     .license_identifier("MPL-2.0")
///     .add_license_text("Mozilla Public License Version 2.0", "1. Definitions ... 2. License Grants and Conditions ...")
///     .add_license_text("MIT License", "Permission is hereby granted, ...")
///     .build();
///
/// assert_eq!(
///     my_package,
///     Package {
///         name: "test-package".to_owned(),
///         version: "0.1.0".to_owned(),
///         authors: vec!["Max Mustermann <max@example.com>".to_owned(), "Erika Mustermann".to_owned()],
///         description: Some("A test package.".to_owned()),
///         homepage: Some("https://codeberg.org/".to_owned()),
///         repository: Some("https://codeberg.org/".to_owned()),
///         license_identifier: Some("MPL-2.0".to_owned()),
///         license_texts: vec![
///             ("Mozilla Public License Version 2.0".to_owned(), "1. Definitions ... 2. License Grants and Conditions ...".to_owned()),
///             ("MIT License".to_owned(), "Permission is hereby granted, ...".to_owned())
///         ],
///     }
/// );
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
            license_texts: vec![],
        })
    }

    // Add an author to the package.
    //
    // This method can be used repeatedly to add more authors.
    #[must_use]
    pub fn add_author(mut self, author: impl Into<String>) -> Self {
        self.0.authors.push(author.into());
        self
    }

    /// Set the description of the package.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.0.description = Some(description.into());
        self
    }

    /// Set the homepage URL of the package.
    #[must_use]
    pub fn homepage(mut self, homepage: impl Into<String>) -> Self {
        self.0.homepage = Some(homepage.into());
        self
    }

    /// Set the repository URL of the package.
    #[must_use]
    pub fn repository(mut self, repository: impl Into<String>) -> Self {
        self.0.repository = Some(repository.into());
        self
    }

    /// Set the SPDX license identifier of the package.
    #[must_use]
    pub fn license_identifier(mut self, license_identifier: impl Into<String>) -> Self {
        self.0.license_identifier = Some(license_identifier.into());
        self
    }

    /// Add a license text.
    ///
    /// The `name` parameter can be anything from file location to license name.
    /// This method can be used repeatedly to add more license texts.
    #[must_use]
    pub fn add_license_text(
        mut self,
        name: impl Into<String>,
        license_text: impl Into<String>,
    ) -> Self {
        self.0
            .license_texts
            .push((name.into(), license_text.into()));
        self
    }

    /// Build the [`Package`].
    #[must_use]
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
    ///         env!("OUT_DIR"),
    ///         "/",
    ///         "LICENSE-3RD-PARTY.nanoserde.lz4"
    ///     )))
    ///     .unwrap();
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
            "/",
            "LICENSE-3RD-PARTY.nanoserde.lz4"
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
    use assert2::{assert, check};

    use super::*;

    fn fuzz_package_builder_property(u: &mut arbitrary::Unstructured) -> arbitrary::Result<()> {
        let pkg: Package = u.arbitrary()?;

        let mut builder = Package::builder(&pkg.name, &pkg.version);

        for a in &pkg.authors {
            builder = builder.add_author(a);
        }
        if let Some(desc) = &pkg.description {
            builder = builder.description(desc);
        }
        if let Some(homepage) = &pkg.homepage {
            builder = builder.homepage(homepage);
        }
        if let Some(repo) = &pkg.repository {
            builder = builder.repository(repo);
        }
        if let Some(ident) = &pkg.license_identifier {
            builder = builder.license_identifier(ident);
        }
        for (lic_name, lic_text) in &pkg.license_texts {
            builder = builder.add_license_text(lic_name, lic_text);
        }
        let pkg_build = builder.build();

        assert!(pkg == pkg_build);

        Ok(())
    }

    #[test]
    fn fuzz_package_builder() {
        arbtest(fuzz_package_builder_property).run();
    }

    fn check_string_contains_package_data(display: &str, pkg: &Package) {
        check!(display.contains(&pkg.name) && display.contains(&pkg.version));

        for author in &pkg.authors {
            assert!(display.contains(author));
        }
        if let Some(desc) = &pkg.description {
            assert!(display.contains(desc));
        }
        if let Some(homepage) = &pkg.homepage {
            assert!(display.contains(homepage));
        }
        if let Some(repo) = &pkg.repository {
            assert!(display.contains(repo));
        }
        if let Some(ident) = &pkg.license_identifier {
            assert!(display.contains(ident));
        }
        for (lic_name, lic_text) in &pkg.license_texts {
            assert!(display.contains(lic_name));
            assert!(display.contains(lic_text));
        }
    }

    fn fuzz_display_package_contains_input_property(
        u: &mut arbitrary::Unstructured,
    ) -> arbitrary::Result<()> {
        let pkg: Package = u.arbitrary()?;
        check_string_contains_package_data(&format!("{pkg}"), &pkg);
        Ok(())
    }

    #[test]
    fn fuzz_display_package_contains_input() {
        arbtest(fuzz_display_package_contains_input_property).run();
    }

    fn fuzz_display_package_list_contains_input_property(
        u: &mut arbitrary::Unstructured,
    ) -> arbitrary::Result<()> {
        let pkg_list: PackageList = u.arbitrary()?;
        for pkg in pkg_list.0 {
            check_string_contains_package_data(&format!("{pkg}"), &pkg);
        }
        Ok(())
    }

    #[test]
    fn fuzz_display_package_list_contains_input() {
        arbtest(fuzz_display_package_list_contains_input_property).run();
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
    fn test_ord_trait_for_package() {
        let create_test_package = |name: &str, id: &str| {
            Package::builder(name, id)
                .add_author("Max Mustermann")
                .add_author("Erika Mustermann")
                .description("Some weird ass test package.")
                .homepage("https://example.com")
                .repository("https://github.com/example/test-package")
                .license_identifier("MPL-2.0")
                .add_license_text("NaN", "NaN")
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
        })
        .run();
    }
}
