//! Graceful shutdown coordinator
//!
//! Handles SIGTERM/SIGINT signals and coordinates clean shutdown
//! of all node components with database flush guarantees.
#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, Ordering};
use tokio::signal;
use tracing::{error, info, warn};

/// Global shutdown flag
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Check if shutdown has been requested
pub fn is_shutdown_requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::Relaxed)
}

/// Request shutdown (can be called from anywhere)
pub fn request_shutdown() {
    SHUTDOWN_REQUESTED.store(true, Ordering::Relaxed);
}

/// Shutdown coordinator that handles graceful termination
pub struct ShutdownCoordinator {
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

impl ShutdownCoordinator {
    pub fn new() -> Self {
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(10);
        Self { shutdown_tx }
    }

    /// Get a receiver for shutdown notifications
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Start listening for shutdown signals (CTRL+C, SIGTERM)
    pub async fn wait_for_signal(self) {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install CTRL+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                info!("🛑 Received CTRL+C, initiating graceful shutdown...");
            }
            _ = terminate => {
                info!("🛑 Received SIGTERM, initiating graceful shutdown...");
            }
        }

        // Set global shutdown flag
        request_shutdown();

        // Broadcast shutdown to all subscribers
        if let Err(e) = self.shutdown_tx.send(()) {
            warn!("Failed to broadcast shutdown signal: {}", e);
        }

        // Give components time to finish current operations
        info!("⏳ Waiting 5 seconds for components to shut down...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // Flush database
        info!("💾 Flushing database to disk...");
        if let Err(e) = Self::flush_database() {
            error!("Failed to flush database: {}", e);
        }

        info!("✅ Graceful shutdown complete");
    }

    /// Flush the global CHAIN database to disk
    fn flush_database() -> Result<(), String> {
        let chain = crate::CHAIN.lock();
        chain
            .db
            .flush()
            .map_err(|e| format!("Database flush failed: {}", e))?;

        info!("   Database flushed successfully");
        Ok(())
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to wrap a task with shutdown awareness
pub async fn run_until_shutdown<F, Fut>(
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    task: F,
) where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    tokio::select! {
        _ = task() => {
            info!("Task completed normally");
        }
        _ = shutdown_rx.recv() => {
            info!("Task interrupted by shutdown signal");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shutdown_request() {
        assert!(!is_shutdown_requested());
        request_shutdown();
        assert!(is_shutdown_requested());

        // Reset for other tests
        SHUTDOWN_REQUESTED.store(false, Ordering::Relaxed);
    }

    #[tokio::test]
    async fn test_shutdown_coordinator_subscribe() {
        let coordinator = ShutdownCoordinator::new();
        let mut rx = coordinator.subscribe();

        // Should not have received anything yet
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_run_until_shutdown() {
        let coordinator = ShutdownCoordinator::new();
        let rx = coordinator.subscribe();

        // Task that completes immediately
        let task_completed = Arc::new(AtomicBool::new(false));
        let task_flag = task_completed.clone();

        run_until_shutdown(rx, || async move {
            task_flag.store(true, Ordering::Relaxed);
        })
        .await;

        assert!(task_completed.load(Ordering::Relaxed));
    }
}
