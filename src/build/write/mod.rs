// Copyright Adam McKellar 2024, 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod error;

use std::{env::var_os, fs::write, path::PathBuf, time::Instant};

use log::info;
use lz4_flex::compress_prepend_size;
use nanoserde::SerBin;

use crate::{OUT_FILE_NAME, PackageList, build::write::error::WriteError};

impl PackageList {
    /// Encodes and compresses a [`PackageList`].
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let data = self.serialize_bin();

        info!("License data size: {} Bytes", data.len());
        let instant_before_compression = Instant::now();

        let compressed_data = compress_prepend_size(&data);

        info!(
            "Compressed data size: {} Bytes in {}ms",
            compressed_data.len(),
            instant_before_compression.elapsed().as_millis()
        );

        compressed_data
    }

    /// Writes the [`PackageList`] into [`$OUT_DIR/LICENSE-3RD-PARTY.bincode.deflate`](`env!("OUT_DIR")`)
    ///
    /// `$OUT_DIR` is set by cargo during build. This function is meant to be only used inside a build script
    /// and only in conjunction with [`read_package_list_from_out_dir`](crate::read_package_list_from_out_dir).
    ///
    /// [`env!("OUT_DIR")`]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
    ///
    /// ## Errors
    ///
    /// Returns [`WriteError`] if the license file was failed to be written to the `OUT_DIR` or if more importabntly this function was not called from a build script!
    /// The reason for the latter variant, [`WriteError::NotBuildScript`], is that this function depends on environment variables set during
    /// compilation.
    ///
    pub fn write_package_list_to_out_dir(&self) -> std::result::Result<(), WriteError> {
        let compressed_data = self.encode();

        let path =
            PathBuf::from(var_os("OUT_DIR").ok_or(WriteError::NotBuildScript)?).join(OUT_FILE_NAME);

        info!("Writing to file: {}", &path.display());
        write(path, compressed_data).map_err(WriteError::Write)?;

        Ok(())
    }
}
