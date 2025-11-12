use axum::Router;
pub mod cash;
pub mod cash_admin;
pub mod cash_store;
pub mod crypto_watch;
pub mod cursor;
pub mod fiat_stripe;
pub mod land;
pub mod oracle;
use std::sync::Arc;

pub fn router() -> Router {
    // open sled DB and share via Extension. Allow overriding path for tests/CI via VISION_DB_PATH
    // Use the store helper to obtain the sled Db so we don't open the same
    // path twice (which causes lock errors on Windows when tests also open it).
    let db = crate::market::cash_store::db_owned();
    let shared = Arc::new(db);
    // Spawn background crypto watchers, passing the shared DB so they don't reopen it.
    #[cfg(feature = "testhooks")]
    {
        use tokio::sync::Notify;
        let notify = std::sync::Arc::new(Notify::new());
        // testhooks: do not spawn watchers here; main spawns them to avoid dup tasks
        // mount testhooks routes
        use axum::{extract::State, routing::post, Json};
        #[derive(Clone)]
        struct TestHookState {
            notify: std::sync::Arc<Notify>,
        }
        let th = TestHookState {
            notify: notify.clone(),
        };
        let test_router = axum::Router::new()
            .route(
                "/__test/watcher_tick",
                post(move |State(st): State<TestHookState>| async move {
                    st.notify.notify_one();
                    Json(serde_json::json!({"ok": true}))
                }),
            )
            .with_state(th);
        Router::new()
            .merge(land::router(shared.clone()))
            .merge(cash::router())
            .merge(fiat_stripe::router())
            .merge(oracle::router())
            .merge(cash_admin::router())
            .merge(test_router)
            .layer(axum::extract::Extension(shared))
    }

    #[cfg(not(feature = "testhooks"))]
    {
        // default wiring: watchers are spawned from `main.rs` to avoid duplicate tasks

        Router::new()
            .merge(land::router(shared.clone()))
            .merge(cash::router())
            .merge(fiat_stripe::router())
            .merge(oracle::router())
            .merge(cash_admin::router())
            .layer(axum::extract::Extension(shared))
    }
}
