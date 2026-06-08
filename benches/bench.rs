use std::sync::LazyLock;

use criterion::{Criterion, criterion_group, criterion_main};
use license_fetcher::prelude::*;

static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    ConfigBuilder::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_crate"))
        .cargo_directives([CargoDirective::Locked])
        .build()
        .unwrap()
});

fn bench_fetch_licenses(c: &mut Criterion) {
    c.bench_function("package_list_with_licenses", |b| {
        b.iter(|| {
            let _a = package_list_with_licenses(&*CONFIG);
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
