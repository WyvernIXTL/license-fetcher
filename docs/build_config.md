This module holds the structs and enums to configure the fetching process.

## Build Scripts (`build.rs`)

If you are using `license-fetcher` from within a build script to fetch licenses for your project,
it is recommended to use [`ConfigBuilder::from_build_env()`], as cargo sets the necessary environment
variables during build. [See the docs.](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates)

`build.rs`:

```rs
use license_fetcher::build::config::{ConfigBuilder, Config};

fn main() {
    let config: Config = ConfigBuilder::from_build_env()
        .build()
        .expect("Failed to build configuration.");

    // ...
}
```

## In an Application

If you are using `license-fetcher` from inside another application to fetch licenses,
you'll probably want to fetch licenses for another project.
In this case there exits the [`ConfigBuilder::from_path()`] method.

`main.rs`:

```rs
use license_fetcher::build::config::{ConfigBuilder, Config};

fn main() {
    let my_path = PathBuf::from(".");

    let config: Config = ConfigBuilder::from_path(my_path)
        .build()
        .expect("Failed loading configuration from path.");

    // ...
}
```
