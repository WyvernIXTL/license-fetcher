// Copyright Adam McKellar 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    ffi::{OsStr, OsString},
    path::Path,
    process::{Command, Output},
};

use error_stack::{Report, Result, ResultExt};
use thiserror::Error;

use crate::build::config::{CargoDirective, MetadataConfig};

#[derive(Debug, Clone, Copy, Error)]
pub enum ExecCargoError {
    #[error("`cargo` did not execute successfully.")]
    FailedExecution,
    #[error("Failed to execute `cargo`.")]
    FailedToExecute,
}

fn exec_cargo_single<P, S, I>(
    cargo: P,
    cargo_directive: &CargoDirective,
    manifest_dir: P,
    features_opt: &Option<OsString>,
    arguments: I,
) -> Result<Output, ExecCargoError>
where
    P: AsRef<Path>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(cargo.as_ref());

    command.current_dir(manifest_dir.as_ref()).args(arguments);

    if let Some(features) = features_opt {
        command.arg("-F").arg(features);
    }

    if *cargo_directive != CargoDirective::Default {
        let cargo_directive: &'static str = (*cargo_directive).into();
        command.arg(cargo_directive);
    }

    let output = command
        .output()
        .change_context(ExecCargoError::FailedToExecute)
        .attach_printable_lazy(|| format!("cargo directive: {}", cargo_directive))?;

    if output.status.success() {
        Ok(output)
    } else {
        Err(Report::new(ExecCargoError::FailedExecution)
            .attach_printable(format!("cargo directive: {}", cargo_directive))
            .attach_printable(String::from_utf8_lossy(&output.stderr).into_owned()))
    }
}

pub fn exec_cargo<I, S>(config: &MetadataConfig, arguments: I) -> Result<Output, ExecCargoError>
where
    I: IntoIterator<Item = S> + Clone,
    S: AsRef<OsStr> + Clone,
{
    debug_assert!(!config.cargo_directives.is_empty());

    let mut err: Option<Report<ExecCargoError>> = None;

    for directive in config.cargo_directives.iter() {
        let result_single = exec_cargo_single(
            &config.cargo_path,
            directive,
            &config.manifest_dir,
            &config.enabled_features,
            arguments.clone(),
        );
        match result_single {
            Ok(_) => return result_single,
            Err(e) => match err.as_mut() {
                None => err = Some(e),
                Some(err) => err.extend_one(e),
            },
        }
    }

    Err(err.unwrap_or_else(|| Report::new(ExecCargoError::FailedExecution)))
}
