use std::sync::LazyLock;

use criterion::{criterion_group, criterion_main, Criterion};
use license_fetcher::{
    build::{
        config::{CargoDirective, Config, ConfigBuilder},
        fetch::populate_package_list_licenses,
        metadata::package_list,
    },
    PackageList,
};

static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    ConfigBuilder::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_crate"))
        .unwrap()
        .cargo_directives([CargoDirective::Locked])
        .cache(false)
        .build()
        .unwrap()
});

fn bench_fetch_licenses(c: &mut Criterion) {
    c.bench_function("package_list_with_licenses", |b| {
        b.iter(|| {
            let _a = license_fetcher::build::package_list_with_licenses(CONFIG.clone());
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

fn bench_licenses_only(c: &mut Criterion) {
    static PKGS: LazyLock<PackageList> =
        LazyLock::new(|| package_list(&CONFIG.metadata_config).unwrap());

    c.bench_function("licenses_text_from_cargo_src_folder", |b| {
        b.iter(|| {
            let mut pkgs = PKGS.clone();
            let _a = populate_package_list_licenses(&mut pkgs, CONFIG.cargo_home_dir.clone());
        })
    });
}

criterion_group!(
    benches,
    bench_fetch_licenses,
    bench_fetching_metadata,
    bench_licenses_only
);
criterion_main!(benches);
