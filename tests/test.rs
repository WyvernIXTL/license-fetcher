// TODO: WTF is this!?

use std::sync::LazyLock;

use assert2::assert;
use license_fetcher::{Package, OUT_FILE_NAME};
use serial_test::serial;

static TEST_CRATE_ROOT_PACKAGE: LazyLock<Package> = LazyLock::new(|| {
    Package::builder("test_crate", "0.1.0")
        .authors(["Max Mustermann"])
        .license_identifier("CC0-1.0")
        .license_text("THIS IS NOT A LICENSE")
        .homepage("https://example.com")
        .build()
});

const TEST_CRATE_DIRECT_DEPS: [&str; 6] = [
    "bincode",
    "directories",
    "log",
    "miniz_oxide",
    "serde",
    "serde_json",
];

#[cfg(feature = "build")]
#[test]
fn test_generate_licenses_with_test_crate() {
    use assert2::check;
    use license_fetcher::build::config::{CargoDirective, ConfigBuilder};

    let config = ConfigBuilder::default()
        .with_path(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_crate"))
        .cargo_directives([CargoDirective::Locked])
        .build()
        .unwrap();

    let mut licenses = license_fetcher::build::package_list_with_licenses(&config).unwrap();
    if let Some(license_text) = licenses[0].license_text.as_mut() {
        *license_text = license_text.trim().to_string();
    }

    check!(
        licenses.len() > 0
            && licenses[0] == *TEST_CRATE_ROOT_PACKAGE
            && TEST_CRATE_DIRECT_DEPS
                .iter()
                .all(|name| licenses[1..].iter().any(|p| p.name == *name))
            && licenses[1..].iter().any(|e| e.license_text.is_some())
            && licenses[1..].iter().all(|e| !e.name.is_empty())
            && licenses[1..].iter().all(|e| !e.version.is_empty())
            && licenses[1..].iter().any(|e| !e.authors.is_empty())
            && licenses[1..].iter().any(|e| e.description.is_some())
            && licenses[1..].iter().any(|e| e.homepage.is_some())
            && licenses[1..].iter().any(|e| e.repository.is_some())
    );
}

const LICENSE_FETCHER_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[cfg(feature = "build")]
#[test]
#[serial]
fn test_fetching_and_serialization_from_env_var_license_fetcher() {
    use std::fs::read;

    use assert2::check;
    use license_fetcher::{
        build::{config::ConfigBuilder, package_list_with_licenses},
        PackageList,
    };

    let temp_dir = tempfile::tempdir().unwrap();
    unsafe {
        use std::path::PathBuf;

        std::env::set_var("OUT_DIR", temp_dir.path());
        std::env::set_var(
            "CARGO_MANIFEST_DIR",
            PathBuf::from(LICENSE_FETCHER_MANIFEST_DIR).join("tests/test_crate"),
        );
    }

    // Build config from env
    assert!(let Ok(config) = ConfigBuilder::from_build_env().build(), "Failed fetchting license metadata.");

    // Fetch metadata and licenses.
    assert!(let Ok(packages) =  package_list_with_licenses(&config), "Failed fetching licenses for packages.");

    // Write packages to out dir to be embedded.
    assert!(
        packages.write_package_list_to_out_dir().is_ok(),
        "Failed writing license data."
    );

    assert!(let Ok(read_binary) = read(temp_dir.path().join(OUT_FILE_NAME)));

    assert!(let Ok(mut read_packages) = PackageList::from_encoded(&read_binary));
    if let Some(license_text) = read_packages[0].license_text.as_mut() {
        *license_text = license_text.trim().to_string();
    }

    check!(
        read_packages[0] == *TEST_CRATE_ROOT_PACKAGE
            && TEST_CRATE_DIRECT_DEPS
                .iter()
                .all(|dep_name| read_packages[1..].iter().any(|d| d.name == *dep_name))
    );

    std::env::remove_var("OUT_DIR");
    std::env::remove_var("CARGO_MANIFEST_DIR");
}

#[cfg(feature = "build")]
#[test]
#[serial]
fn test_fetching_and_serialization_from_env_var_license_fetcher_use_cache() {
    use license_fetcher::build::{config::ConfigBuilder, package_list_with_licenses};

    let temp_dir = tempfile::tempdir().unwrap();
    unsafe {
        use std::path::PathBuf;

        std::env::set_var("OUT_DIR", temp_dir.path());
        std::env::set_var(
            "CARGO_MANIFEST_DIR",
            PathBuf::from(LICENSE_FETCHER_MANIFEST_DIR).join("tests/test_crate"),
        );
    }

    // Build config from env
    assert!(let Ok(config) = ConfigBuilder::from_build_env().build(), "Failed fetchting license metadata.");

    // Fetch metadata and licenses.
    assert!(let Ok(packages) =  package_list_with_licenses(&config), "Failed fetching licenses for packages.");

    // Write packages to out dir to be embedded.
    assert!(
        packages.write_package_list_to_out_dir().is_ok(),
        "Failed writing license data."
    );

    assert!(let Ok(packages2) =  package_list_with_licenses(&config), "Failed fetching licenses for packages the second time.");

    assert!(packages == packages2);

    std::env::remove_var("OUT_DIR");
    std::env::remove_var("CARGO_MANIFEST_DIR");
}
