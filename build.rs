use std::fs::File;
use std::env;

// This build script only has one use: stfu rust-analyzer.

fn main() {
    let mut path = env::var_os("OUT_DIR").unwrap();
    path.push("/LICENSE-3RD-PARTY.bincode");
    let _ = File::create(path);
    println!("cargo::rerun-if-changed=Cargo.lock");
}