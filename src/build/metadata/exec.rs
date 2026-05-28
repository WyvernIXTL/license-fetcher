// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    ffi::{OsStr, OsString},
    path::Path,
    process::{Command, Output},
};

use exn::{bail, ensure, Result, ResultExt};

use crate::build::{
    config::{CargoDirective, MetadataConfig},
    error::{ErrorJoin, EK, IE},
};

fn exec_cargo_single<P, S, I>(
    cargo: P,
    cargo_directive: CargoDirective,
    manifest_dir: P,
    features_opt: Option<&OsString>,
    arguments: I,
) -> Result<Output, IE>
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

    let output = command.output().or_raise(|| {
        IE::new(format!(
            "cargo executable should execute at all | directive: \"{cargo_directive}\""
        ))
        .with_path(cargo.as_ref())
        .with_kind(EK::CargoFailedExecution)
    })?;

    if output.status.success() {
        Ok(output)
    } else {
        bail!(IE::new(format!("cargo should execute with status success (0) | directive: \"{cargo_directive}\" | stderr: \"{}\"", String::from_utf8_lossy(&output.stderr))))
    }
}

pub(super) fn exec_cargo<I, S>(config: &MetadataConfig, arguments: &I) -> Result<Output, IE>
where
    I: IntoIterator<Item = S> + Clone,
    S: AsRef<OsStr> + Clone,
{
    ensure!(
        !config.cargo_directives.is_empty(),
        IE::new("cargo directives in config passed to `exec_cargo` should contain at least one directive")
    );

    let mut err_join = ErrorJoin::new(IE::new("cargo should at least succeed with one directive"));

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
            Err(e) => err_join.join(e),
        }
    }

    Err(err_join.err())
}
