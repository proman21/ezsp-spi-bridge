use tracing_subscriber::fmt;

pub fn setup_logging() {
    fmt()
    .json()
    .with_timer(fmt::time())
    .init()
}