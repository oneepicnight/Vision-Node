use tracing::Level;

pub fn init_test_tracing() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // Keep tracing setup minimal and avoid requiring optional features in tracing-subscriber
        let _ = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .with_test_writer()
            .try_init();
    });
}
