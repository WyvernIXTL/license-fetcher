// Known issue, this build script gets executed on every build of the library, even if it does do nothging.

#[cfg(test)]
use std::env;
#[cfg(test)]
use std::fs::File;

// This build script only has one use: stfu rust-analyzer.

#[cfg(test)]
fn main() {
    let mut path = env::var_os("OUT_DIR").unwrap();
    path.push("/LICENSE-3RD-PARTY.bincode.deflate");
    let _ = File::create(path);
    println!("cargo::rerun-if-changed=Cargo.lock");
}

#[cfg(not(test))]
fn main() {}
