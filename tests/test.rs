use std::sync::LazyLock;

use assert2::assert;
use license_fetcher::{package, Package};

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

    let licenses = license_fetcher::build::package_list_with_licenses(&config).unwrap();

    check!(
        licenses.len() > 0
            && licenses[0].license_identifier.is_some()
            && licenses[0].license_identifier.clone().unwrap() == "CC0-1.0"
            && licenses[0].name == "test_crate"
            && licenses[0].version == "0.1.0"
            && licenses[0]
                .license_text
                .clone()
                .expect("Failed fetching license of test crate.")
                .contains("THIS IS NOT A LICENSE")
            && licenses[1..].iter().any(|e| e.license_text.is_some())
            && licenses[1..].iter().any(|e| !e.name.is_empty())
            && licenses[1..].iter().any(|e| !e.version.is_empty())
            && licenses[1..].iter().any(|e| !e.authors.is_empty())
            && licenses[1..].iter().any(|e| e.description.is_some())
            && licenses[1..].iter().any(|e| e.homepage.is_some())
            && licenses[1..].iter().any(|e| e.repository.is_some())
    );
}

static LICENSE_FETCHER_ROOT_PACKAGE: LazyLock<Package> = LazyLock::new(|| {
    let cargo_toml_str = include_str!(env!("CARGO_MANIFEST_PATH"));
    let cargo_toml_parsed = boml::parse(cargo_toml_str).unwrap();
    let package_meta = cargo_toml_parsed.get_table("package").unwrap();

    let license_text = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/LICENSE"));
    let license_text = format!("{license_text}\n\n"); // This will likely break :(

    let mut pkg = package!(
        name: package_meta.get_string("name").unwrap().to_owned(),
        version: package_meta.get_string("version").unwrap().to_owned(),
        authors: package_meta
            .get_array("authors")
            .unwrap()
            .iter()
            .filter_map(|val| val.as_string().map(|s| s.to_owned()))
            .collect(),
        description: package_meta
            .get_string("description")
            .ok()
            .map(|value| value.to_owned()),
        homepage: package_meta
            .get_string("homepage")
            .ok()
            .map(|value| value.to_owned()),
        repository: package_meta
            .get_string("repository")
            .ok()
            .map(|value| value.to_owned()),
        license_identifier: package_meta
            .get_string("license")
            .ok()
            .map(|value| value.to_owned()),
        license_text: Some(license_text.to_owned()),
    );

    pkg.is_root_pkg = true;

    pkg
});

// I think even the optional ones will be here, as this is only run, when `build` feature is enabled.
static LICENSE_FETCHER_DIRECT_DEPENDENCIES: LazyLock<Vec<String>> = LazyLock::new(|| {
    let cargo_toml_str = include_str!(env!("CARGO_MANIFEST_PATH"));
    let cargo_toml_parsed = boml::parse(cargo_toml_str).unwrap();
    let dependencies = cargo_toml_parsed.get_table("dependencies").unwrap();

    dependencies
        .iter()
        .filter(|(_, v)| match v.as_table() {
            Some(v_arr) => {
                !v_arr.contains_key("optional") || !v_arr.get_boolean("optional").unwrap()
            }
            None => true,
        })
        .map(|(k, _)| format!("{k}"))
        .collect()
});

#[cfg(feature = "build")]
#[test]
fn test_fetching_and_serialization_from_env_var_license_fetcher() {
    use std::fs::read;

    use assert2::check;
    use license_fetcher::{
        build::{config::ConfigBuilder, package_list_with_licenses},
        PackageList,
    };

    let temp_dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("OUT_DIR", temp_dir.path());
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

    assert!(let Ok(read_binary) = read(temp_dir.path().join("LICENSE-3RD-PARTY.bincode.deflate")));

    assert!(let Ok(read_packages) = PackageList::from_encoded(&read_binary));

    check!(
        read_packages[0] == *LICENSE_FETCHER_ROOT_PACKAGE
            && LICENSE_FETCHER_DIRECT_DEPENDENCIES
                .iter()
                .all(|dep| read_packages[1..]
                    .iter()
                    .map(|d| dbg!(d.name.clone()))
                    .any(|d| dbg!(d.eq(dep))))
    );

    std::env::remove_var("OUT_DIR");
}

#[cfg(feature = "build")]
#[test]
fn test_fetching_and_serialization_from_env_var_license_fetcher_use_cache() {
    use license_fetcher::build::{config::ConfigBuilder, package_list_with_licenses};

    let temp_dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("OUT_DIR", temp_dir.path());
    }

    // Build config from env
    assert!(let Ok(config) = ConfigBuilder::from_build_env().cache(true).build(), "Failed fetchting license metadata.");

    // Fetch metadata and licenses.
    assert!(let Ok(packages) =  package_list_with_licenses(&config), "Failed fetching licenses for packages.");

    // Write packages to out dir to be embedded.
    assert!(
        packages.write_package_list_to_out_dir().is_ok(),
        "Failed writing license data."
    );

    // Fetch metadata and licenses again, hopefully with cache.
    assert!(let Ok(packages2) =  package_list_with_licenses(&config), "Failed fetching licenses for packages.");

    assert!(packages == packages2);

    std::env::remove_var("OUT_DIR");
}
