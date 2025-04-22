Fetch licenses of dependencies at build time and embed them into your program.

`license-fetcher` is a crate for fetching actual license texts from the cargo source directory for
crates that are compiled with your project. It does this in the build step
in a build script. This means that the heavy dependencies of `license-fetcher`
aren't your dependencies!

## Example
Don't forget to import `license-fetcher` as a normal AND as a build dependency!
```sh
cargo add --build --features build license-fetcher
cargo add license-fetcher
```


### `src/main.rs`

```ignore
use license_fetcher::get_package_list_macro;
fn main() {
    let package_list = get_package_list_macro!().unwrap();
}
```


### `build.rs`

```ignore
use license_fetcher::build_script::generate_package_list_with_licenses;

fn main() {
    generate_package_list_with_licenses().write();
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=Cargo.lock");
    println!("cargo::rerun-if-changed=Cargo.toml");
}
```


## Adding Packages that are not Crates

Sometimes we have dependencies that are not crates. For these dependencies `license-fetcher` cannot
automatically generate information. These dependencies can be added manually:

```ignore
use std::fs::read_to_string;
use std::concat;

use license_fetcher::{
    Package,
    build_script::generate_package_list_with_licenses
};

fn main() {
    let mut packages = generate_package_list_with_licenses();

    packages.push(Package {
        name: "other dependency".to_owned(),
        version: "0.1.0".to_owned(),
        authors: vec!["Me".to_owned()],
        description: Some("A dependency that is not a rust crate.".to_owned()),
        homepage: None,
        repository: None,
        license_identifier: None,
        license_text: Some(
            read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/some_dependency/LICENSE"))
            .expect("Failed reading license of other dependency")
        )
    });

    packages.write();

    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=Cargo.lock");
    println!("cargo::rerun-if-changed=Cargo.toml");
    
}
```

## Feature Flags
| Feature    | Description                                                                                                      |
| ---------- | ---------------------------------------------------------------------------------------------------------------- |
| `compress` | *(default)* Enables compression.                                                                                 |
| `build`    | Used for build script component.                                                                                 |
| `frozen`   | Panics if `Cargo.lock` needs to be updated or if a network request needs to be made for `cargo metadata` to run. |
