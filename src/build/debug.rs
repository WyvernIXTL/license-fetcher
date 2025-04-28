use std::sync::Once;

use error_stack::Report;

static SETUP_ONCE: Once = Once::new();

pub(crate) fn setup_logger() {
    use simplelog::Config;

    simplelog::TermLogger::new(
        log::LevelFilter::Debug,
        Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    );
}

pub(crate) fn setup_test() {
    SETUP_ONCE.call_once(|| {
        setup_logger();
        if supports_color::on_cached(supports_color::Stream::Stderr)
            .is_some_and(|level| level.has_basic)
        {
            Report::set_color_mode(error_stack::fmt::ColorMode::Color);
        }
    });
}
