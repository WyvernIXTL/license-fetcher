use std::sync::LazyLock;

use criterion::{criterion_group, criterion_main, Criterion};
use license_fetcher::build::{
    config::{CargoDirective, Config, ConfigBuilder},
    metadata::package_list,
};

static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    ConfigBuilder::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_crate"))
        .cargo_directives([CargoDirective::Locked])
        .cache_path(false)
        .build()
        .unwrap()
});

fn bench_fetch_licenses(c: &mut Criterion) {
    c.bench_function("package_list_with_licenses", |b| {
        b.iter(|| {
            let _a = license_fetcher::build::package_list_with_licenses(&CONFIG);
        })
    });
}

fn bench_fetching_metadata(c: &mut Criterion) {
    c.bench_function("package_list", |b| {
        b.iter(|| {
            let _a = package_list(&CONFIG.metadata_config);
        })
    });
}

criterion_group!(benches, bench_fetch_licenses, bench_fetching_metadata);
criterion_main!(benches);
