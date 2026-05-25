# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## v0.10.0

### Added

- Add text wrapping for crate description and license in display trait.
- The license texts of a crate are now stored and shown separately.
- Documentation for all errors.


### Changed

- The `Package` struct now does not have private fields and can be freely constructed and inspected.
- Replaced the `package!` macro with `PackageBuilder` builder.
- The `cache` method of the `ConfigBuilder` now requires a path to a cache file.
- The `package_list` method is moved into the `build` module.
- The `package_list` method now returns the packages sorted with the first package being the root package.
- The `package_list` function now may either take `Config`, `&Config`, `MetadataConfig` or `&MetadataConfig` as argument.
- The `package_list_with_licenses` function now may either take `Config` or `&Config` as argument.


## v0.9.3

### Fixed

- Fixed packages with numbers in their name not being parsed.


## v0.9.2

### Fixed

- Fix unexpected [deprecation of `doc_auto_cfg`](https://github.com/rust-lang/rust/pull/138907).


## v0.9.1

### Added

- Added `serde` feature, that enables the derivation of `Serialize` and `Deserialize` traits.


## v0.9.0

The release of v0.9.0 brings a speed-up of compilation compared to v0.8.4. The API remains mostly unchanged to v0.8.4, while the compression and serialization formats change.


### Changed

- Removed `thiserror` and `fnv` crate for a small compilation speed-up.
- Switched from `bincode` to `nanoserde` crate, as the former was abandoned.
- Switched back to `lz4_flex` from `miniz_oxide` for a small compilation speed-up.
- Moved from `serde` and `serde_json` to the much smaller `nanoserde` for a major speed-up of compilation.
- Turned off `kv` feature of `log` crate. Maybe there is a compilation speed-up?


## v0.8.4

### Added

- Categories in `Cargo.toml` for visibility for package.

## v0.8.3

### Added

- Added caveat section in README.

### Changed

- Error instead of panic when root package is missing in package list during metadata fetch step.

## v0.8.2

## Fixed

- Some root ID of cargo metadata output not being parsed correctly.

## v0.8.1

## Fixed

- Docs in docs.rs not building.

## v0.8.0

### Added

- Configuration builder
  - From build environment
  - From manifest file
- Caching
- Execution without panic
- Nice error traces

### Changed

- Switched to MPL-2.0 license.
- Also fetches licenses in direct sub folders (`.*license.*` and the like).
