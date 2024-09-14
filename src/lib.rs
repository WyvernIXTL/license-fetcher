//               Copyright Adam McKellar 2024
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)


use std::env;
use std::fmt;
use std::concat;
use std::include_bytes;
use std::error::Error;
use std::fs::File;

use bincode::{config, Decode, Encode};

#[cfg(feature = "compress")]
use lz4_flex::block::decompress_size_prepended;

#[cfg(all(feature = "compress", feature = "build"))]
use lz4_flex::block::compress_prepend_size;

#[cfg(feature = "build")]
use std::fs::write;


#[derive(Encode, Decode, Debug, PartialEq, Eq)]
pub struct Package {
    name: String,
    version: String,
    authors: Option<Vec<String>>,
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

        writeln!(f, "{}\n", separator)?;

        for package in self.0.iter() {
            writeln!(f, "Package:     {} {}", package.name, package.version)?;
            if let Some(description) = &package.description {
                writeln!(f, "Description: {}", description)?;
            }
            if let Some(authors) = &package.authors {
                writeln!(f, "Authors:")?;
                for author in authors.iter() {
                    writeln!(f, " - {}", author)?;
                }
            }
            if let Some(homepage) = &package.homepage {
                writeln!(f, "Homepage:    {}", homepage)?;
            }
            if let Some(repository) = &package.repository {
                writeln!(f, "Repository:  {}", repository)?;
            }
            if let Some(license_identifier) = &package.license_identifier {
                writeln!(f, "SPDX Ident.: {}", license_identifier)?;
            }

            writeln!(f, "\n{}\n", separator)?;
            
            if let Some(license_text) = &package.license_text {
                writeln!(f, "{}\n{}\n", license_text, separator)?;
            }
        }

        Ok(())
    }
}


pub fn get_package_list() -> Result<PackageList, Box<dyn Error + 'static>> {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/LICENSE-3RD-PARTY"));

    #[cfg(feature = "compress")]
    let uncompressed_bytes = decompress_size_prepended(bytes)?;

    #[cfg(not(feature = "compress"))]
    let uncompressed_bytes = bytes;

    let (package_list, _) = bincode::decode_from_slice(&uncompressed_bytes[..], config::standard())?;
    Ok(package_list)
}

#[cfg(feature = "build")]
fn write_package_list(package_list: PackageList) {
    let mut path = env::var_os("OUT_DIR").unwrap();
    path.push("/LICENSE-3RD-PARTY");

    let data = bincode::encode_to_vec(package_list, config::standard()).unwrap();

    #[cfg(feature = "compress")]
    let compressed_data = compress_prepend_size(&data);

    #[cfg(not(feature = "compress"))]
    let compressed_data = data;

    write(path, compressed_data).unwrap();

    println!("cargo::rerun-if-changed=Cargo.lock");
}

