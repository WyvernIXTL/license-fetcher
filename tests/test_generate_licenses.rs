#[cfg(feature = "build")]
#[test]
fn test_generate_licenses() {
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
