use criterion::{criterion_group, criterion_main, Criterion};
use license_fetcher::build::config::ConfigBuilder;

fn test_fetch_licenses_test() {
    let config = ConfigBuilder::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_crate"))
        .unwrap()
        .build()
        .unwrap();

    let _a = license_fetcher::build::package_list_with_licenses(config);
}

fn bench_fetch_licenses(c: &mut Criterion) {
    let mut group = c.benchmark_group("test-crate");
    // group.measurement_time(Duration::from_secs(30));
    // group.sample_size(50);
    group.bench_function("fetch-licenses-test-crate", |b| {
        b.iter(|| test_fetch_licenses_test())
    });
    group.finish();
}

criterion_group!(benches, bench_fetch_licenses);
criterion_main!(benches);
