// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{env::var_os, path::PathBuf};

use exn::{OptionExt, Result};

use crate::build::config::error::CEK;
use crate::build::config::Cie;

use super::ConfigBuilder;

struct MetadataEnv {
    manifest_dir: PathBuf,
    cargo_path: PathBuf,
}

impl MetadataEnv {
    fn new() -> Result<Self, Cie> {
        Ok(Self {
            manifest_dir: var_os("CARGO_MANIFEST_DIR")
                .ok_or_raise(|| {
                    Cie::new("within a build script the `CARGO_MANIFEST_DIR` env var should be set")
                        .with_kind(CEK::FailedFromEnvVars)
                })?
                .into(),
            cargo_path: var_os("CARGO")
                .ok_or_raise(|| {
                    Cie::new("within a build script the `CARGO` env var should be set")
                        .with_kind(CEK::FailedFromEnvVars)
                })?
                .into(),
        })
    }
}

impl ConfigBuilder {
    /// Adds needed values from environment variables to builder.
    ///
    /// This method is meant to be used from a build script (`build.rs`)!
    /// The environment variables used are set by cargo during build.
    #[must_use]
    pub fn with_build_env(mut self) -> Self {
        match MetadataEnv::new() {
            Ok(meta) => {
                self = self
                    .manifest_dir(meta.manifest_dir)
                    .cargo_path(meta.cargo_path);
            }
            Err(err) => {
                self.errors.push(err.raise(Cie::new("within a build script the `with_build_env` and `from_build_env` methods should succeed")));
            }
        }

        self
    }

    /// New builder with needed values being filled in from environment variables.
    ///
    /// This constructor is meant to be used from a build script (`build.rs`)!
    /// The environment variables used are set by cargo during build.
    #[must_use]
    pub fn from_build_env() -> Self {
        ConfigBuilder::default().with_build_env()
    }
}

/* -------------------------------------------------------------------------- */
/*                                 Unit Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod test {
    use crate::build::{config::error::ConfigBuilderError, debug::setup_test};

    use super::*;

    #[test]
    fn test_config_from_env() -> Result<(), ConfigBuilderError> {
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
