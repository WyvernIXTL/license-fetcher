//               Copyright Adam McKellar 2024
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use std::fs::write;
use std::process::Command;
use std::collections::BTreeSet;

#[cfg(feature = "compress")]
use lz4_flex::block::compress_prepend_size;

use serde_json::{from_slice, Value};


mod metadata;

use crate::*;
use crate::build_script::metadata::*;


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

fn generate_package_list() -> PackageList {
    let cargo_path = env::var_os("CARGO").unwrap();
    let manifest_path = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    /* let output = Command::new(&cargo_path)
                            .current_dir(&manifest_path)
                            .args(["tree", "-e", "normal", "-f", "'{p};{l};{r};'", "--prefix", "none"])
                            .output()
                            .unwrap();
    let tree_string = String::from_utf8(output.stdout).unwrap();
    let mut used_packages = BTreeSet::new();
    for line in tree_string.lines() {
        let split: Vec<&str> = line.split(";").collect();
        let split_option: Vec<Option<String>> = split.into_iter().map(|i| 
            if !i.is_empty() { 
                Some(i.to_owned()) 
            } else {
                None
            }).collect();

        used_packages.insert((split_option[0].clone(), split_option[1].clone(), split_option[2].clone()));
    } */

    let mut package_list = vec![];

    let metadata_output = Command::new(cargo_path)
                                        .current_dir(manifest_path)
                                        .args(["metadata", "--format-version", "1", "--color", "never"])
                                        .output()
                                        .unwrap();
    let metadata_parsed: Metadata = from_slice(&metadata_output.stdout).unwrap();

    

    PackageList(package_list)
}

