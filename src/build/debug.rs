use std::sync::Once;

static LOGGER_ONCE: Once = Once::new();

pub(crate) fn install_logger_build_env() {
    LOGGER_ONCE.call_once(|| {
        picolog::PicoLogger::new(log::LevelFilter::Debug).init();
    });
}

#[cfg(test)]
pub(crate) fn install_logger_test_env() {
    LOGGER_ONCE.call_once(|| {
        picolog::PicoLogger::new(log::LevelFilter::Trace).init();
    });
}

#[cfg(test)]
pub(crate) fn test_setup() {
    install_logger_test_env();
}
