// (This build script only has one use: stfu rust-analyzer.)
// This dummy is used not only by the IDE, but also by the doctests in CI.

use std::env;
use std::fs::File;

fn main() {
    let mut path = env::var_os("OUT_DIR").unwrap();
    path.push("/LICENSE-3RD-PARTY.bincode.deflate");
    let _ = File::create(path);
    println!("cargo::rerun-if-changed=Cargo.lock");

    println!("cargo::rustc-check-cfg=cfg(coverage_nightly)");
}
