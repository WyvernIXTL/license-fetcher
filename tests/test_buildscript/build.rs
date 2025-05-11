use license_fetcher::build::{config::ConfigBuilder, package_list_with_licenses};

fn main() {
    let config = ConfigBuilder::from_build_env().build().unwrap();

    package_list_with_licenses(config)
        .unwrap()
        .write_package_list_to_out_dir()
        .unwrap();
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=Cargo.lock");
    println!("cargo::rerun-if-changed=Cargo.toml");
}
