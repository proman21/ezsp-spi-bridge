use tracing::Level;
use tracing_subscriber::fmt;

pub fn setup_logging(level: Level) {
    fmt()
    .json()
    .with_timer(fmt::time())
    .with_max_level(level)
    .with_current_span(false)
    .with_span_list(false)
    .init()
}