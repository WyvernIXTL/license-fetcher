use std::{
    backtrace::Backtrace,
    env::{var, var_os},
    ffi::OsStr,
    path::PathBuf,
};

use log::error;
use snafu::{OptionExt, ResultExt, Snafu};

use super::ConfigBuilder;

impl ConfigBuilder {
    /// New builder with needed values being filled in from environment variables.
    ///
    /// This constructor is meant to be used from a build script (`build.rs`)!
    /// The environment variables used are set by cargo during build.
    pub fn from_env() -> Result<Self, ConfigBuilderEnvError> {
        let package_name = string_from_env("CARGO_PKG_NAME")?;
        let manifest_dir = path_buf_from_env("CARGO_MANIFEST_DIR")?;
        let cargo_path = path_buf_from_env("CARGO")?;

        Ok(ConfigBuilder::custom(
            package_name,
            manifest_dir,
            cargo_path,
        ))
    }
}

fn path_buf_from_env(env: impl AsRef<OsStr>) -> Result<PathBuf, ConfigBuilderEnvError> {
    let env_value = var_os(&env)
        .with_context(|| EnvVarNotPresentSnafu {
            env_variable: env.as_ref().to_string_lossy(),
        })
        .inspect_err(|e| error!("{}", e))?;

    Ok(PathBuf::from(env_value))
}

fn string_from_env<K>(env: K) -> Result<String, ConfigBuilderEnvError>
where
    K: AsRef<OsStr>,
{
    let env_value = var(&env)
        .with_context(|_| EnvVarSnafu {
            env_variable: env.as_ref().to_string_lossy(),
        })
        .inspect_err(|e| error!("{}", e))?;

    Ok(env_value)
}

/// Error that appears during failed build of config.
#[derive(Debug, Snafu)]
pub enum ConfigBuilderEnvError {
    /// Error that appears during execution of [ConfigBuilder::from_env()].
    ///
    /// This error might appear if this function is not called from a build script.
    /// Cargo sets during execution of the build script the needed environment variables.
    #[snafu(display(
        "Environment variable '{env_variable}' is not set. Was 'from_env()' not called from a build script ('build.rs')?"
    ))]
    EnvVarNotPresent {
        env_variable: String,
        backtrace: Backtrace,
    },
    /// Error that appears during execution of [ConfigBuilder::from_env()].
    #[snafu(display("Failure getting the environment variable '{env_variable}'."))]
    EnvVarError {
        source: std::env::VarError,
        env_variable: String,
        backtrace: Backtrace,
    },
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[snafu::report]
    fn test_config_from_env() -> Result<(), ConfigBuilderEnvError> {
        let conf = ConfigBuilder::from_env()?.build();
        assert_eq!(conf.package_name, env!("CARGO_PKG_NAME"));
        assert_eq!(conf.manifest_dir, PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        assert_eq!(conf.cargo_path, PathBuf::from(env!("CARGO")));

        Ok(())
    }
}
