//               Copyright Adam McKellar 2025
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use std::collections::HashMap;
use std::env::current_dir;
use std::fs::{read_dir, read_to_string};
use std::io::prelude::*;
use std::io::{stdout, BufWriter};
use std::path::{absolute, PathBuf};

use clap::Parser;
use color_eyre::eyre::Result;
use colored::Colorize;
use serde::Deserialize;
use serde_json::to_string_pretty;

use license_fetcher::build_script::generate_package_list_with_licenses_without_env_calls;
use license_fetcher::get_package_list_macro;
use license_fetcher::PackageList;

#[derive(Deserialize)]
struct CargoToml {
    package: CargoPackage,
}

#[derive(Deserialize)]
struct CargoPackage {
    name: String,
}

/// CLI for printing license information of rust cargo projects to the terminal.
///
/// Cargo needs to be installed and be in the PATH.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Optional path to manifest dir (where Cargo.toml and Cargo.lock are). Defaults to current dir.
    manifest_dir_path: Option<PathBuf>,

    /// Output as yaml.
    #[arg(short, long)]
    yaml: bool,

    /// Output as json.
    #[arg(short, long)]
    json: bool,

    /// Outputs only a short overview.
    #[arg(short, long)]
    short: bool,

    /// Outputs license information regarding this software and it's dependencies.
    #[arg(short, long)]
    license: bool,
}

fn print_short_license_info(package_list: PackageList) -> Result<()> {
    let mut license_map: HashMap<String, Vec<String>> = HashMap::new();
    for pck in package_list.iter() {
        if let Some(license) = pck.license_identifier.clone() {
            if !license_map.contains_key(&license) {
                license_map.insert(license, vec![pck.name.clone()]);
            } else {
                license_map
                    .get_mut(&license)
                    .unwrap()
                    .push(pck.name.clone());
            }
        }
    }
    let mut stdout_buffered = BufWriter::new(stdout());
    for (license, packages) in license_map {
        write!(stdout_buffered, "{}: ", license.green())?;
        for pck in packages.iter().take(packages.len() - 1) {
            write!(stdout_buffered, "{}, ", pck)?;
        }
        write!(stdout_buffered, "{}\n", packages.last().unwrap())?;
    }
    stdout_buffered.flush()?;

    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    if cli.license {
        let packages = get_package_list_macro!()?;
        println!("{}", packages);
        return Ok(());
    }

    let manifest_dir = match cli.manifest_dir_path {
        Some(path) => {
            if !path.try_exists()? {
                panic!("{}", "Error: Path does not exist!".red());
            }
            let absolute_path = absolute(path)?;
            if !absolute_path.is_dir() {
                absolute_path
                    .parent()
                    .unwrap_or_else(|| {
                        panic!("{}", "Error: Cannot find parent of path.".red());
                    })
                    .to_owned()
            } else {
                absolute_path
            }
        }
        None => current_dir()?,
    };

    assert!(manifest_dir.is_dir());

    let cargo_toml_path = read_dir(manifest_dir.clone())?
        .into_iter()
        .filter_map(|enry| enry.ok())
        .filter(|entry| entry.file_type().map_or(false, |ft| ft.is_file()))
        .filter(|entry| entry.file_name().to_string_lossy() == "Cargo.toml")
        .next()
        .expect(&format!(
            "{}",
            "Error: Failed finding Cargo.toml file in dir.".red()
        ))
        .path();

    let cargo_toml: CargoToml = toml::from_str(&read_to_string(cargo_toml_path)?)?;
    let name = cargo_toml.package.name;

    let package_list = generate_package_list_with_licenses_without_env_calls(
        None,
        manifest_dir.as_os_str().to_owned(),
        name,
    );

    if cli.yaml {
        println!("{}", serde_yml::to_string(&package_list)?)
    } else if cli.json {
        println!("{}", to_string_pretty(&package_list)?)
    } else {
        if cli.short {
            print_short_license_info(package_list)?;
        } else {
            println!("{}", package_list);
        }
    }

    Ok(())
}
