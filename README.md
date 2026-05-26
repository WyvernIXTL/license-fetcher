<div align="center">

# `license-fetcher`

**Fetch licenses of dependencies at build time and embed them into your program.**

[![Crates.io Version](https://img.shields.io/crates/v/license-fetcher)](https://crates.io/crates/license-fetcher)
[![GitHub License](https://img.shields.io/github/license/WyvernIXTL/license-fetcher)](https://github.com/WyvernIXTL/license-fetcher/blob/main/LICENSE)
[![docs.rs](https://img.shields.io/docsrs/license-fetcher)](https://docs.rs/license-fetcher)
[![lib.rs link](https://badgen.net/badge/lib.rs/lib.rs/purple?label)](https://lib.rs/crates/license-fetcher)
[![dependency status](https://deps.rs/repo/github/WyvernIXTL/license-fetcher/status.svg)](https://deps.rs/repo/github/WyvernIXTL/license-fetcher)
[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/WyvernIXTL/license-fetcher/test.yml?branch=main)](https://github.com/WyvernIXTL/license-fetcher/actions/workflows/test.yml)
[![codecov](https://codecov.io/gh/WyvernIXTL/license-fetcher/graph/badge.svg?token=FUBAIYFXSP)](https://codecov.io/gh/WyvernIXTL/license-fetcher)

</div>

## Aspirations

1. Fetch licenses!
2. Fast!
3. Do it in the build step!

> [!NOTE]
> If you are in search for a CLI checkout [flicense](https://github.com/WyvernIXTL/flicense-rs).

## Workings

Crates that are compiled with your program are fetched via `cargo metadata` and `cargo tree`.
License texts are read from the `.cargo/registry/src` folder.
The data is then serialized, compressed and embedded.
At runtime `license-fetcher` has only very few and lightweight dependencies needed for the
decompression, deserialization and pretty print of the license data.

## Usage

Here is a small rundown how to use this library for fetching licenses during build time.
Though fetching licenses at runtime is also supported. See the [docs](https://docs.rs/license-fetcher/latest/license_fetcher/build/index.html).

### Include Dependency

> [!WARNING]
> Include this library as build dependency and as normal dependency!

```
cargo add --build --features build license-fetcher
cargo add license-fetcher
```

### Build Script

This library requires you to execute it for fetching licenses in a build script.
Create a file called `build.rs` in the root of your project and add following contents:

```rust
use license_fetcher::build::config::{ConfigBuilder, Config};
use license_fetcher::build::package_list_with_licenses;
use license_fetcher::PackageList;

fn main() {
    // Config with environment variables set by cargo, to fetch licenses at build time.
    let config: Config = ConfigBuilder::from_build_env()
        .build()
        .expect("Failed to build configuration.");

    // Fetch metadata and licenses.
    let packages: PackageList = package_list_with_licenses(&config)
                                    .expect("Failed to fetch metadata or licenses.");

    // Write packages to out dir to be embedded.
    packages.write_package_list_to_out_dir().expect("Failed to write package list.");

    // Rerun only if one of the following files changed:
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=Cargo.lock");
    println!("cargo::rerun-if-changed=Cargo.toml");
}
```

### Main

Add following content to your `main.rs`:

```rust
use license_fetcher::read_package_list_from_out_dir;

fn main() {
    let package_list = read_package_list_from_out_dir!().unwrap();
    println!("{package_list}");
}
```


### Much Better Setup With Leniency

**I added really great copy-pasta examples in the [`build` module documentation](https://docs.rs/license-fetcher/latest/license_fetcher/build/index.html). Take a look!**


## Caveats

`license-fetcher` fetches licenses that are at the root of a crate or in subfolders. This results in some caveats, mainly:

- Some projects do not upload licenses with their crates. This might happen if a project is split up into many crates.
- Some wrappers do not attribute the library they are wrapping.
- Dependencies that are not crates, like dictionaries, are not detected.

To work around the former points, it is advisable to use [`flicense --stats .`](https://github.com/WyvernIXTL/flicense-rs) on your crates,
to see what crates license-fetcher fetches.

For the latter point there is no workaround, as there is no automated way to detect the use of such dependencies.

## Alternatives

### [`license-retriever`](https://github.com/MRT-Map/license-retriever)

This project was a big inspiration for license-fetcher.
The idea of fetching licenses during build step did not even occur to me beforehand.

A big shout-out!

#### Pros

- Also retrieves licenses in the build step and embeds them.
- Can use repo URL to fetch licenses.
- Can use SPDX to fetch licenses.
- Smaller and simpler crate.

#### Cons

- Does not compress licenses.
- Uses the LGPL-3.0 license. 
  This means that your code also needs to be licenses LGPL-3.0 or compatible as rust links statically by default.
- Has many large dependencies and thus compiles slower.
- No option to disable fetching via git.
- Does not differentiate between dependencies for build and runtime, leading to larger executables with more dependencies.

### [`cargo-about`](https://github.com/EmbarkStudios/cargo-about)

#### Pros

- Can generate very nice HTML.
- Can use repo URL to fetch licenses.

#### Cons

- Is not a library to access said data but rather a command line tool.
  You need to keep the license file up to date (via CI for example).
- If you export and embed the license data as JSON, you need to handle compression, validation and display of it yourself.


## Screenshots

_Display trait included_ 😉

![Screenshot](./img/example_print_v0.10.0.png)

## Performance

With the advanced example of the docs, where licenses are fetched, compressed and embedded at build time 
and just printed at runtime in the main, I got these timings with "`cargo b --release --timings`":

![cargo timings for v0.10.0](./img/cargo_timinings_v0.10.0.png)

The full report is available in [`img/timings`](./img/timings).

_Please note that these timings are for **v0.10.0**. I used `cargo add --path` and just have not bumped the version yet, as I use `cargo-release` for that._

These timings are of course for a clean build. For incremental builds setting `cargo::rerun-if-changed` like shown in the example above, will result
in the build script only being run if `Cargo.toml`, `Cargo.lock` or `build.rs` change and build artifacts are of course also cached by cargo.

In conclusion, the build script is fast (0.14s). It is only run if necessary. Compiling `license-fetcher` is also very fast (1.4s for a clean build). 
As cargo compiles crates in parallel, you'll likely never notice :)


## License

This project is licensed under the [MPL 2.0 license](./LICENSE).
