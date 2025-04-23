#[cfg(feature = "build")]
#[test]
fn test_generate_licenses() {
    let licenses = license_fetcher::build::generate_package_list_with_licenses_without_env_calls(
        Some(env!("CARGO").into()),
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_crate").into(),
        "test_crate".to_owned(),
    );

    assert!(licenses.len() > 0);
    assert!(licenses[0].license_identifier.is_some());
    assert_eq!(licenses[0].license_identifier.clone().unwrap(), "CC0-1.0");
    assert_eq!(licenses[0].name, "test_crate");
    assert_eq!(licenses[0].version, "0.1.0");
    assert!(licenses[0]
        .license_text
        .clone()
        .expect("Failed fetching license of test crate.")
        .contains("THIS IS NOT A LICENSE"));
    assert!(licenses[1..].iter().any(|e| e.license_text.is_some()));
    assert!(licenses[1..].iter().any(|e| !e.name.is_empty()));
    assert!(licenses[1..].iter().any(|e| !e.version.is_empty()));
    assert!(licenses[1..].iter().any(|e| !e.authors.is_empty()));
    assert!(licenses[1..].iter().any(|e| e.description.is_some()));
    assert!(licenses[1..].iter().any(|e| e.homepage.is_some()));
    assert!(licenses[1..].iter().any(|e| e.repository.is_some()));
}
