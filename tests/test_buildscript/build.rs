use license_fetcher::prelude::*;

fn main() {
    // Config with environment variables set by cargo, to fetch licenses at build time.
    let config: Config = ConfigBuilder::from_build_env()
        .build()
        .expect("Failed to build configuration.");

    // Fetch metadata and licenses.
    let packages: PackageList =
        package_list_with_licenses(&config).expect("Failed to fetch metadata or licenses.");

    // Write packages to out dir to be embedded.
    packages
        .write_package_list_to_out_dir()
        .expect("Failed to write package list.");

    // Rerun only if one of the following files changed:
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=Cargo.lock");
    println!("cargo::rerun-if-changed=Cargo.toml");
}
