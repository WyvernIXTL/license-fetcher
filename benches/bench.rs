use criterion::{criterion_group, criterion_main, Criterion};

fn test_fetch_licenses_test() {
    let _a = license_fetcher::build_script::generate_package_list_with_licenses_without_env_calls(
        Some(env!("CARGO").into()),
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_crate").into(),
        "test_crate".to_owned(),
    );
}

fn bench_fetch_licenses(c: &mut Criterion) {
    c.bench_function("fetch-licenses-test-crate", |b| {
        b.iter(|| test_fetch_licenses_test())
    });
}

criterion_group!(benches, bench_fetch_licenses);
criterion_main!(benches);
