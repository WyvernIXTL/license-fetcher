//               Copyright Adam McKellar 2024
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)


use std::env;
use std::fmt;
use std::error::Error;

use bincode::{config, Decode, Encode};

#[cfg(feature = "compress")]
use lz4_flex::block::decompress_size_prepended;

#[cfg(feature = "build")]
pub mod build_script;


#[derive(Encode, Decode, Debug, PartialEq, Eq)]
pub struct Package {
    name: String,
    version: String,
    authors: Vec<String>,
    description: Option<String>,
    homepage: Option<String>,
    repository: Option<String>,
    license_identifier: Option<String>,
    license_text: Option<String>,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq)]
pub struct PackageList(Vec<Package>);


impl fmt::Display for PackageList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const SEPERATOR_WIDTH: usize = 80;
        let separator: String = "=".repeat(SEPERATOR_WIDTH);
        let separator_light: String = "-".repeat(SEPERATOR_WIDTH);

        writeln!(f, "{}\n", separator)?;

        for package in self.0.iter() {
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


pub fn get_package_list(bytes: &[u8]) -> Result<PackageList, Box<dyn Error + 'static>> {
    #[cfg(feature = "compress")]
    let uncompressed_bytes = decompress_size_prepended(bytes)?;

    #[cfg(not(feature = "compress"))]
    let uncompressed_bytes = bytes;

    let (package_list, _) = bincode::decode_from_slice(&uncompressed_bytes[..], config::standard())?;
    Ok(package_list)
}

#[macro_export]
macro_rules! get_package_list_macro {
    () => {
        license_fetcher::get_package_list(std::include_bytes!(std::concat!(env!("OUT_DIR"), "/LICENSE-3RD-PARTY.bincode"))).unwrap()
    };
}

