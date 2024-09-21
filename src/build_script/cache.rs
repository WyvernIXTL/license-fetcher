//               Copyright Adam McKellar 2024
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use std::env::var_os;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use bincode::{Decode, Encode, decode_from_slice, encode_to_vec, config};
use log::info;

use crate::PackageList;

const CACHE_FILE_NAME: &str = ".license-cache";

#[derive(Decode, Encode, Debug)]
pub(super) struct LicenseCache {
    data: BTreeMap<(String, String), String>,
}

fn cache_file() -> File {
    let mut path = var_os("CARGO_MANIFEST_DIR").unwrap();
    path.push("/");
    path.push(CACHE_FILE_NAME);

    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .expect("Failed opening license cache file.")
}

impl LicenseCache {
    pub(super) fn load() -> Self {
        let mut file = cache_file();
        
        let mut binary = vec![];
        let len = file.read_to_end(&mut binary).unwrap();
        info!("Read {} Bytes from license cache.", len);

        if len == 0 {
            return Self {
                data: BTreeMap::new()
            };
        }

        let (data, _) = decode_from_slice(&binary, config::standard())
                                                        .expect("Failed parsing license cache to HashMap");

        Self { data }
    }

    pub(super) fn write(&self) {
        let mut file = cache_file();

        let binary = encode_to_vec(&self.data, config::standard())
                                    .expect("Failed encoding license cache with bincode.");

        file.write_all(&binary).unwrap();
    }

    pub(super) fn load_licenses_for_package_list(&self, list: &mut PackageList) {
        for package in list.0.iter_mut() {
            package.license_text = self.data.get(&(package.name.clone(), package.version.clone())).cloned();
        }
    }

    pub(super) fn write_licenses_from_package_list(&mut self, list: &PackageList) {
        for package in list.0.iter() {
            if let Some(license_text) = &package.license_text {
                if let Entry::Vacant(entry) = self.data.entry((package.name.clone(), package.version.clone())) {
                    entry.insert(license_text.clone());
                }
            }
        }
    }
}
