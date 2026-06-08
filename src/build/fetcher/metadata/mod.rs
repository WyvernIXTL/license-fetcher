// Copyright Adam McKellar 2025, 2026
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use exn::Result;
use std::thread::scope;

use crate::{
    Package,
    build::fetcher::{
        error::{ErrorJoin, IE},
        metadata::{
            exec_metadata::exec_cargo_metadata_and_parse_result,
            exec_tree::exec_cargo_tree_and_parse_output,
        },
    },
};

use crate::build::config::MetadataConfig;

mod exec;
mod exec_metadata;
mod exec_tree;
mod json_parsing;

pub(super) fn package_list_impl(
    config: &MetadataConfig,
) -> Result<(String, impl Iterator<Item = Package> + '_), IE> {
    scope(|scope| {
        let cargo_metadata_thread_handle =
            scope.spawn(|| exec_cargo_metadata_and_parse_result(config));

        let cargo_tree_thread_handle = scope.spawn(|| exec_cargo_tree_and_parse_output(config));

        let cargo_metadata_thread_result = cargo_metadata_thread_handle.join().map_err(|e| {
            IE::new(format!(
                "the `cargo metadata` thread should complete without panic | panic message: '{e:?}'"
            ))
        })?;
        let cargo_tree_thread_result = cargo_tree_thread_handle.join().map_err(|e| {
            IE::new(format!(
                "the `cargo tree` thread should complete without panic | panic message: '{e:?}'"
            ))
        })?;

        match (cargo_metadata_thread_result, cargo_tree_thread_result) {
            (Err(cargo_metadata_err), Err(cargo_tree_err)) => {
                let mut err_join = ErrorJoin::new(IE::new(
                    "`cargo metadata` and `cargo tree` should execute successfully and their outputs should parse correctly",
                ));
                err_join.join(cargo_metadata_err);
                err_join.join(cargo_tree_err);
                Err(err_join.err())
            }
            (Err(cargo_metadata_err), _) => Err(cargo_metadata_err),
            (_, Err(cargo_tree_err)) => Err(cargo_tree_err),
            (Ok((root_package_name, packages_iter)), Ok(used_package_names)) => Ok((
                root_package_name,
                packages_iter.filter(move |p| used_package_names.contains(&p.name)),
            )),
        }
    })
}
