use tracing_subscriber::{EnvFilter, fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logging() {
    tracing_subscriber::registry()
        .with(filter())
        .with(fmt_layer())
        .init();
}

pub fn fmt_layer<S>() -> Layer<S> {
    tracing_subscriber::fmt::layer()
        // Display source code file paths
        .with_file(true)
        // Display source code line numbers
        .with_line_number(true)
        // Don't display the event's target (module path)
        .with_target(false)
}

pub fn filter() -> EnvFilter {
    // Use the logging options from env variables
    EnvFilter::from_default_env()
        // Increase logging requirements for noisy dependencies
        .add_directive("hyper_util=info".parse().expect("directive was invalid"))
}
