use std::fs::File;
use std::env;

fn main() {
    let mut path = env::var_os("OUT_DIR").unwrap();
    path.push("/LICENSE-3RD-PARTY");
    let _ = File::create(path);
    println!("cargo::rerun-if-changed=Cargo.lock");
}