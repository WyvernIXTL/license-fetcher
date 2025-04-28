use std::env::VarError;
use std::{
    env::{var, var_os},
    ffi::OsStr,
    path::PathBuf,
};

use error_stack::{Result, ResultExt};

use super::ConfigBuilder;

impl ConfigBuilder {
    /// New builder with needed values being filled in from environment variables.
    ///
    /// This constructor is meant to be used from a build script (`build.rs`)!
    /// The environment variables used are set by cargo during build.
    pub fn from_build_env() -> Result<Self, VarError> {
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

fn path_buf_from_env(env: impl AsRef<OsStr>) -> Result<PathBuf, VarError> {
    let env_value = var_os(&env)
        .ok_or_else(|| VarError::NotPresent)
        .attach_printable_lazy(|| {
            format!("Environment Variable: '{}'", env.as_ref().to_string_lossy())
        })?;

    Ok(PathBuf::from(env_value))
}

fn string_from_env<K>(env: K) -> Result<String, VarError>
where
    K: AsRef<OsStr>,
{
    Ok(var(&env).attach_printable_lazy(|| {
        format!("Environment Variable: '{}'", env.as_ref().to_string_lossy())
    })?)
}

#[cfg(test)]
mod test {
    use crate::build::debug::setup_test;

    use super::*;

    #[test]
    fn test_config_from_env() -> Result<(), VarError> {
        setup_test();
        let conf = ConfigBuilder::from_build_env()?.build();
        assert_eq!(conf.package_name, env!("CARGO_PKG_NAME"));
        assert_eq!(conf.manifest_dir, PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        assert_eq!(conf.cargo_path, PathBuf::from(env!("CARGO")));

        Ok(())
    }
}
