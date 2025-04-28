use std::sync::Once;

static LOGGER_ONCE: Once = Once::new();

pub(crate) fn setup_logger() {
    use simplelog::Config;

    LOGGER_ONCE.call_once(|| {
        simplelog::TermLogger::new(
            log::LevelFilter::Debug,
            Config::default(),
            simplelog::TerminalMode::Mixed,
            simplelog::ColorChoice::Auto,
        );
    });
}

pub(crate) fn setup_test() {
    setup_logger();
}
