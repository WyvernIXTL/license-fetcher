// Copyright Adam McKellar 2025
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{fs::File, path::PathBuf, sync::Once};

use simplelog::{CombinedLogger, Config, WriteLogger};

static SETUP_ONCE: Once = Once::new();

pub(crate) fn setup_logger() {
    let log_file_path = PathBuf::from(
        std::env::var_os("CARGO_TARGET_DIR ")
            .expect("'CARGO_TARGET_DIR' should be set for the initialization of logger"),
    )
    .join("license-fetcher.log");

    let log_file = File::create(log_file_path).expect("log file should be creatable");

    CombinedLogger::init(vec![
        simplelog::TermLogger::new(
            log::LevelFilter::Debug,
            Config::default(),
            simplelog::TerminalMode::Mixed,
            simplelog::ColorChoice::Auto,
        ),
        WriteLogger::new(log::LevelFilter::Debug, Config::default(), log_file),
    ])
    .expect("logger should initialize");
}

pub(crate) fn setup_test() {
    SETUP_ONCE.call_once(|| {
        setup_logger();
    });
}
