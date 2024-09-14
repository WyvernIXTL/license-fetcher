//               Copyright Adam McKellar 2024
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use std::fs::write;

#[cfg(feature = "compress")]
use lz4_flex::block::compress_prepend_size;

use crate::*;


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



