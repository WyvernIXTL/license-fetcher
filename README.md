<div align="center">

# `license-fetcher`
**Fetch licenses of dependencies at build time and embed them into your program.**

[![Crates.io Version](https://img.shields.io/crates/v/license-fetcher)](https://crates.io/crates/license-fetcher)
[![GitHub License](https://img.shields.io/github/license/WyvernIXTL/license-fetcher)](https://github.com/WyvernIXTL/license-fetcher/blob/main/LICENSE)
[![docs.rs](https://img.shields.io/docsrs/license-fetcher)](https://docs.rs/license-fetcher)

</div>

## Aspirations

1. Fetch licenses!
2. Fast!
3. Do it in the build step!


## Workings

This library gets all packages that are actually copmiled with your program via `cargo tree`.
This is needed to circumvent a [cargo bug](https://github.com/rust-lang/cargo/issues/10801).
Afterwards it fetches the metadata of said packages with `cargo metadata`.
The `repository` field of the crates is used for fetching the license files.
Licenses from github are fetched directly via its api!
All licenses that aren't fetched via api are fetched with `git`.
The data is then serialized and compressed.

> [!NOTE]  
> Octocrab (github api) does not support rate limits.


## Usage

### Include Dependency

> [!WARNING]
> Include this library as build dependency and as normal dependeny!

```
cargo add --build --features build license-fetcher
cargo add license-fetcher
```

### Build Script

This library requires you to execute it for fetching licenses in a build script.
Creat a file called `build.rs` in the root of your project and add following contents:
```rust
use license_fetcher::build_script::generate_package_list_with_licenses;

fn main() {
    generate_package_list_with_licenses();
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=Cargo.lock");
}
```

### Main

Add following content to your `main.rs`:
```rust
use license_fetcher::get_package_list_macro;

fn main() {
    let packages = get_package_list_macro!();
    println!("{}", packages);
}
```


## Alternatives

### [`license-retriever`](https://github.com/MRT-Map/license-retriever)

#### Pros
+ Also retrieves licenses in the build step and loads them into the program.

#### Cons
- Does not fetch licenses directly via api access.
- Very slow.
- Does not compress licenses.


### [`cargo-about`](https://github.com/EmbarkStudios/cargo-about)

#### Pros
+ Generates very nice html.

#### Cons
- Is not a library to access said data but rather a command line tool.


## Screenshots

*Display trait included* 😉

![Screenshot](./img/example_print.png)