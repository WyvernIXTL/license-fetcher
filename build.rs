use std::env;
use std::fs::File;

// This build script only has one use: stfu rust-analyzer.

fn main() {
    let mut path = env::var_os("OUT_DIR").unwrap();
    path.push("/LICENSE-3RD-PARTY.bincode.deflate");
    let _ = File::create(path);
    println!("cargo::rerun-if-changed=Cargo.lock");
}
