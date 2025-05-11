// Copyright Adam McKellar 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::env::VarError;
use std::{env::var_os, ffi::OsStr, path::PathBuf};

use error_stack::{Result, ResultExt};

use crate::build::error::CEnvVar;

use super::{ConfigBuildError, ConfigBuilder};

struct MetadataEnv {
    manifest_dir: PathBuf,
    cargo_path: PathBuf,
}

impl MetadataEnv {
    fn new() -> Result<Self, VarError> {
        Ok(Self {
            manifest_dir: path_buf_from_env("CARGO_MANIFEST_DIR")?,
            cargo_path: path_buf_from_env("CARGO")?,
        })
    }
}

fn path_buf_from_env(env: impl AsRef<OsStr>) -> Result<PathBuf, VarError> {
    let env_value = var_os(&env)
        .ok_or_else(|| VarError::NotPresent)
        .attach_printable_lazy(|| CEnvVar::from(env))?;

    Ok(PathBuf::from(env_value))
}

impl ConfigBuilder {
    /// Adds needed values from environment variables to builder.
    ///
    /// This method is meant to be used from a build script (`build.rs`)!
    /// The environment variables used are set by cargo during build.
    pub fn with_build_env(mut self) -> Self {
        match MetadataEnv::new().change_context(ConfigBuildError::FailedFromEnvVars) {
            Ok(meta) => {
                self = self
                    .manifest_dir(meta.manifest_dir)
                    .cargo_path(meta.cargo_path);
            }
            Err(e) => {
                self.error.join(e);
            }
        }

        self
    }

    /// New builder with needed values being filled in from environment variables.
    ///
    /// This constructor is meant to be used from a build script (`build.rs`)!
    /// The environment variables used are set by cargo during build.
    pub fn from_build_env() -> Self {
        ConfigBuilder::default().with_build_env()
    }
}

#[cfg(test)]
mod test {
    use crate::build::debug::setup_test;

    use super::*;

    #[test]
    fn test_config_from_env() -> Result<(), ConfigBuildError> {
        setup_test();
        let conf = ConfigBuilder::from_build_env().build()?;
        assert_eq!(
            conf.metadata_config.manifest_dir,
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        );
        assert_eq!(
            conf.metadata_config.cargo_path,
            PathBuf::from(env!("CARGO"))
        );

        Ok(())
    }
}
