// (This build script only has one use: stfu rust-analyzer.)
// This dummy is used not only by the IDE, but also by the doctests in CI.

use std::env;
use std::fs::File;
use std::path::PathBuf;

fn main() {
    let path =
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("LICENSE-3RD-PARTY.nanoserde.lz4");

    let _ = File::create(path);

    println!("cargo::rerun-if-changed=Cargo.lock");

    println!("cargo::rustc-check-cfg=cfg(coverage_nightly)");
}
