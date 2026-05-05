// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    error::Error,
    ffi::{OsStr, OsString},
    path::Path,
    process::{Command, Output},
};

use error_stack::{Report, Result, ResultExt};

use crate::build::config::{CargoDirective, MetadataConfig};

/// Error that occurs when `cargo` does not execute or returns itself an error.
#[derive(Debug, Clone, Copy, displaydoc::Display)]
pub enum ExecCargoError {
    /// `cargo` executed, but returned an error
    ExecutionWithError,
    /// failed to execute `cargo`
    FailedToExecute,
}

impl Error for ExecCargoError {}

fn exec_cargo_single<P, S, I>(
    cargo: P,
    cargo_directive: CargoDirective,
    manifest_dir: P,
    features_opt: Option<&OsString>,
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

    if cargo_directive != CargoDirective::Default {
        let cargo_directive: &'static str = (cargo_directive).into();
        command.arg(cargo_directive);
    }

    let output = command
        .output()
        .change_context(ExecCargoError::FailedToExecute)
        .attach_printable_lazy(|| format!("cargo directive: {cargo_directive}"))?;

    if output.status.success() {
        Ok(output)
    } else {
        Err(Report::new(ExecCargoError::ExecutionWithError)
            .attach_printable(format!("cargo directive: {cargo_directive}"))
            .attach_printable(String::from_utf8_lossy(&output.stderr).into_owned()))
    }
}

pub fn exec_cargo<I, S>(config: &MetadataConfig, arguments: &I) -> Result<Output, ExecCargoError>
where
    I: IntoIterator<Item = S> + Clone,
    S: AsRef<OsStr> + Clone,
{
    debug_assert!(
        !config.cargo_directives.is_empty(),
        "cargo directives in config passed to `exec_cargo` should not have been empty"
    );

    let mut err: Option<Report<ExecCargoError>> = None;

    for directive in config.cargo_directives.iter() {
        let result_single = exec_cargo_single(
            &config.cargo_path,
            *directive,
            &config.manifest_dir,
            config.enabled_features.as_ref(),
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

    debug_assert!(
        err.is_some(),
        "cargo execution failed, but the combined error is None"
    );

    Err(err.unwrap_or_else(|| Report::new(ExecCargoError::ExecutionWithError)))
}
