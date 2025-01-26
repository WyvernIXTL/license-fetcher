use std::env::current_dir;
use std::fs::{read_dir, read_to_string};
use std::path::{absolute, PathBuf};

use clap::Parser;
use color_eyre::eyre::Result;
use colored::Colorize;
use serde::Deserialize;

use license_fetcher::build_script::generate_package_list_with_licenses_without_env_calls;

#[derive(Deserialize)]
struct CargoToml {
    package: CargoPackage,
}

#[derive(Deserialize)]
struct CargoPackage {
    name: String,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Optional path to manifest dir (where Cargo.toml and Cargo.lock are).
    manifest_dir_path: Option<PathBuf>,

    /// Output as yaml.
    #[arg(short, long)]
    yaml: bool,

    /// Output as json.
    #[arg(short, long)]
    json: bool,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

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
    println!("{}", package_list);

    Ok(())
}
