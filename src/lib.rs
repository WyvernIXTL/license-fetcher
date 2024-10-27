//               Copyright Adam McKellar 2024
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)


//! Fetch licenses of dependencies at build time and embed them into your program.
//! 
//! `license-fetcher` is a crate for fetching actual license texts from the cargo source directory for
//! crates that are compiled with your project. It does this in the build step
//! in a build script. This means that the heavy dependencies of `license-fetcher`
//! aren't your dependencies!
//! 
//! ## Example
//! Don't forget to import `license-fetcher` as a normal AND as a build dependency!
//! ```sh
//! cargo add --build --features build license-fetcher
//! cargo add license-fetcher
//! ```
//! 
//! ### `src/main.rs`
//! 
//! ```ignore
//! use license_fetcher::get_package_list_macro;
//! fn main() {
//!     let package_list = get_package_list_macro!();
//! }
//! 
//! ```
//! ### `build.rs`
//! ```ignore
//! use license_fetcher::build_script::generate_package_list_with_licenses;
//!
//! fn main() {
//!     generate_package_list_with_licenses();
//!     println!("cargo::rerun-if-changed=build.rs");
//!     println!("cargo::rerun-if-changed=Cargo.lock");
//!     println!("cargo::rerun-if-changed=Cargo.toml");
//! }
//! ```
//! 
//! ## Feature Flags
//! | Feature    | Description                                                             |
//! | ---------- | ----------------------------------------------------------------------- |
//! | `compress` | *(default)* Enables compression.                                        |
//! | `build`    | Used for build script component.                                        |
//! | `frozen`   | Panics if `Cargo.lock` needs to be updated for `cargo metadata` to run. |
//! 



use std::fmt;
use std::error::Error;
use std::ops::{Deref, DerefMut};

use bincode::{config, Decode, Encode};

#[cfg(feature = "compress")]
use miniz_oxide::inflate::decompress_to_vec;

#[cfg(feature = "build")]
pub mod build_script;


/// Information regarding a crate.
/// 
/// This struct holds information like package name, authors and of course license text.
#[derive(Encode, Decode, Debug, PartialEq, Eq)]
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

/// Holds information of all crates and licenses used for release build.
#[derive(Encode, Decode, Debug, PartialEq, Eq)]
pub struct PackageList(pub Vec<Package>);


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
        const SEPERATOR_WIDTH: usize = 80;
        let separator: String = "=".repeat(SEPERATOR_WIDTH);
        let separator_light: String = "-".repeat(SEPERATOR_WIDTH);

        writeln!(f, "{}\n", separator)?;

        for package in self.iter() {
            writeln!(f, "Package:     {} {}", package.name, package.version)?;
            if let Some(description) = &package.description {
                writeln!(f, "Description: {}", description)?;
            }
            if !package.authors.is_empty() {
                writeln!(f, "Authors:     - {}", package.authors.get(0).unwrap_or(&"".to_owned()))?;
                for author in package.authors.iter().skip(1) {
                writeln!(f, "             - {}", author)?;
                }
                //writeln!(f, "")?;
            }
            if let Some(homepage) = &package.homepage {
                writeln!(f, "Homepage:    {}", homepage)?;
            }
            if let Some(repository) = &package.repository {
                writeln!(f, "Repository:  {}", repository)?;
            }
            if let Some(license_identifier) = &package.license_identifier {
                writeln!(f, "SPDX Ident:  {}", license_identifier)?;
            }
            
            if let Some(license_text) = &package.license_text {
                writeln!(f, "\n{}\n{}", separator_light, license_text)?;
            }

            writeln!(f, "\n{}\n", separator)?;
        }

        Ok(())
    }
}

/// Decopresses and deserializes the crate and license information.
/// 
/// Thise function decompresses the input, if `compress` feature was not disabled and
/// then deserializes the input. The input should be the embeded license information from
/// the build step.
/// 
/// # Example
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
pub fn get_package_list(bytes: &[u8]) -> Result<PackageList, Box<dyn Error + 'static>> {
    #[cfg(feature = "compress")]
    let uncompressed_bytes = decompress_to_vec(bytes)
                                            .expect("Failed decompressing license data.");
    #[cfg(not(feature = "compress"))]
    let uncompressed_bytes = bytes;

    let (package_list, _) = bincode::decode_from_slice(&uncompressed_bytes[..], config::standard())?;
    Ok(package_list)
}

/// Calls [get_package_list] with parameters expected from a call from `main.rs`.
/// 
/// # Example
/// ```no_run
/// use license_fetcher::get_package_list_macro;
/// fn main() {
///     let package_list = get_package_list_macro!();
/// }
/// ```
#[macro_export]
macro_rules! get_package_list_macro {
    () => {
        license_fetcher::get_package_list(std::include_bytes!(std::concat!(env!("OUT_DIR"), "/LICENSE-3RD-PARTY.bincode"))).unwrap()
    };
}

