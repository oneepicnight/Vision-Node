//! Network Health Monitoring and Automated Alerts
//!
//! Monitors P2P network health and triggers alerts when thresholds are breached.
//! Provides real-time health scores and actionable recommendations.

use serde::Serialize;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

use crate::p2p::peer_manager::{PeerManager, PeerState};

/// Health alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Health alert type
#[derive(Debug, Clone, Serialize)]
pub struct HealthAlert {
    pub severity: AlertSeverity,
    pub timestamp: u64,
    pub message: String,
    pub metric: String,
    pub current_value: f32,
    pub threshold: f32,
    pub recommendation: String,
}

/// Network health score (0-100)
#[derive(Debug, Clone, Serialize)]
pub struct HealthScore {
    pub overall: u8,      // 0-100
    pub connectivity: u8, // Peer connection health
    pub performance: u8,  // Latency and throughput
    pub stability: u8,    // Connection stability
    pub reputation: u8,   // Peer quality
}

/// Network health monitor
pub struct HealthMonitor {
    peer_manager: Arc<PeerManager>,
    alert_history: Arc<tokio::sync::RwLock<Vec<HealthAlert>>>,
    last_check: Arc<tokio::sync::RwLock<u64>>,
}

impl HealthMonitor {
    /// Create new health monitor
    pub fn new(peer_manager: Arc<PeerManager>) -> Self {
        Self {
            peer_manager,
            alert_history: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            last_check: Arc::new(tokio::sync::RwLock::new(0)),
        }
    }

    /// Run health check and generate alerts if needed
    pub async fn check_health(&self) -> Vec<HealthAlert> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        *self.last_check.write().await = now;

        let mut alerts = Vec::new();

        // Check peer connectivity
        alerts.extend(self.check_peer_connectivity().await);

        // Check network latency
        alerts.extend(self.check_network_latency().await);

        // Check peer quality
        alerts.extend(self.check_peer_quality().await);

        // Check sync status
        alerts.extend(self.check_sync_status().await);

        // Store alerts in history
        if !alerts.is_empty() {
            let mut history = self.alert_history.write().await;
            history.extend(alerts.clone());

            // Keep only last 100 alerts
            let len = history.len();
            if len > 100 {
                history.drain(0..len - 100);
            }

            // Log critical alerts
            for alert in &alerts {
                match alert.severity {
                    AlertSeverity::Critical => error!("[HEALTH] CRITICAL: {}", alert.message),
                    AlertSeverity::Warning => warn!("[HEALTH] WARNING: {}", alert.message),
                    AlertSeverity::Info => info!("[HEALTH] INFO: {}", alert.message),
                }
            }
        }

        alerts
    }

    /// Check peer connectivity health
    async fn check_peer_connectivity(&self) -> Vec<HealthAlert> {
        let mut alerts = Vec::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let peers = self.peer_manager.get_all_peers().await;
        let connected = peers
            .iter()
            .filter(|p| p.state == PeerState::Connected)
            .count();
        let total = peers.len();

        // ⭐ Mesh size target: 5-8 peers
        const TARGET_MIN_PEERS: usize = 5;
        const TARGET_MAX_PEERS: usize = 8;

        if connected == 0 {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Critical,
                timestamp: now,
                message: "NETWORK ISOLATED: No peers connected!".to_string(),
                metric: "connected_peers".to_string(),
                current_value: 0.0,
                threshold: TARGET_MIN_PEERS as f32,
                recommendation: "Check firewall, verify SEED_PEERS, check beacon connectivity"
                    .to_string(),
            });
        }
        // Warning: Below target minimum
        else if connected < TARGET_MIN_PEERS {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Warning,
                timestamp: now,
                message: format!("WARNING: low peer count = {} (target {}). Peer recovery will try to connect to more constellation nodes.", connected, TARGET_MIN_PEERS),
                metric: "connected_peers".to_string(),
                current_value: connected as f32,
                threshold: TARGET_MIN_PEERS as f32,
                recommendation: "Peer recovery active. Waiting for more connections.".to_string(),
            });
        }
        // Info: Within target range
        else if (TARGET_MIN_PEERS..=TARGET_MAX_PEERS).contains(&connected) {
            tracing::info!(
                "[P2P] Connected peers: {} (target {}-{})",
                connected,
                TARGET_MIN_PEERS,
                TARGET_MAX_PEERS
            );
        }
        // Info: Above target but acceptable
        else if connected > TARGET_MAX_PEERS && connected <= TARGET_MAX_PEERS + 3 {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Info,
                timestamp: now,
                message: format!(
                    "High peer count: {} connected (target {}-{})",
                    connected, TARGET_MIN_PEERS, TARGET_MAX_PEERS
                ),
                metric: "connected_peers".to_string(),
                current_value: connected as f32,
                threshold: 5.0,
                recommendation: "Peer count acceptable. Network is healthy.".to_string(),
            });
        }

        // Warning: No hot peers available
        let hot_count = peers
            .iter()
            .filter(|p| p.bucket == crate::p2p::PeerBucket::Hot)
            .count();
        if hot_count < 2 && connected > 2 {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Warning,
                timestamp: now,
                message: format!("Only {} hot peer(s) - low quality connections", hot_count),
                metric: "hot_peers".to_string(),
                current_value: hot_count as f32,
                threshold: 2.0,
                recommendation: "Wait for peers to stabilize, check network quality".to_string(),
            });
        }

        // Info: Many cold peers
        let cold_count = peers
            .iter()
            .filter(|p| p.bucket == crate::p2p::PeerBucket::Cold)
            .count();
        let cold_ratio = if total > 0 {
            cold_count as f32 / total as f32
        } else {
            0.0
        };
        if cold_ratio > 0.7 && total > 10 {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Info,
                timestamp: now,
                message: format!("High cold peer ratio: {:.0}%", cold_ratio * 100.0),
                metric: "cold_peer_ratio".to_string(),
                current_value: cold_ratio,
                threshold: 0.7,
                recommendation: "Consider pruning old peers from database".to_string(),
            });
        }

        alerts
    }

    /// Check network latency
    async fn check_network_latency(&self) -> Vec<HealthAlert> {
        let mut alerts = Vec::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let peers = self.peer_manager.connected_peers().await;
        let latencies: Vec<u32> = peers.iter().filter_map(|p| p.metrics.latency_ms).collect();

        if latencies.is_empty() {
            return alerts;
        }

        let avg_latency = latencies.iter().sum::<u32>() as f32 / latencies.len() as f32;
        let max_latency = *latencies.iter().max().unwrap();

        // Critical: Average latency > 1000ms
        if avg_latency > 1000.0 {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Critical,
                timestamp: now,
                message: format!("Very high average latency: {:.0}ms", avg_latency),
                metric: "avg_latency".to_string(),
                current_value: avg_latency,
                threshold: 1000.0,
                recommendation: "Check internet connection, consider geographic peer selection"
                    .to_string(),
            });
        }
        // Warning: Average latency > 500ms
        else if avg_latency > 500.0 {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Warning,
                timestamp: now,
                message: format!("High average latency: {:.0}ms", avg_latency),
                metric: "avg_latency".to_string(),
                current_value: avg_latency,
                threshold: 500.0,
                recommendation: "Network quality degraded, monitor for improvement".to_string(),
            });
        }

        // Warning: Max latency > 2000ms
        if max_latency > 2000 {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Warning,
                timestamp: now,
                message: format!("Slow peer detected: {}ms", max_latency),
                metric: "max_latency".to_string(),
                current_value: max_latency as f32,
                threshold: 2000.0,
                recommendation: "Consider disconnecting slow peers".to_string(),
            });
        }

        alerts
    }

    /// Check peer quality
    async fn check_peer_quality(&self) -> Vec<HealthAlert> {
        let mut alerts = Vec::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let peers = self.peer_manager.connected_peers().await;
        let scores: Vec<f32> = peers.iter().map(|p| p.metrics.score).collect();

        if scores.is_empty() {
            return alerts;
        }

        let avg_score = scores.iter().sum::<f32>() / scores.len() as f32;
        let low_score_count = scores.iter().filter(|&&s| s < 0.4).count();

        // Warning: Low average peer score
        if avg_score < 0.5 {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Warning,
                timestamp: now,
                message: format!("Low average peer quality: {:.2}", avg_score),
                metric: "avg_peer_score".to_string(),
                current_value: avg_score,
                threshold: 0.5,
                recommendation: "Many unreliable peers, wait for better connections".to_string(),
            });
        }

        // Info: Many low-quality peers
        if low_score_count > 3 {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Info,
                timestamp: now,
                message: format!("{} poor quality peers connected", low_score_count),
                metric: "low_quality_peers".to_string(),
                current_value: low_score_count as f32,
                threshold: 3.0,
                recommendation: "System will deprioritize these peers automatically".to_string(),
            });
        }

        alerts
    }

    /// Check sync status
    async fn check_sync_status(&self) -> Vec<HealthAlert> {
        let mut alerts = Vec::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Get local and network heights
        let local_height = {
            let chain = crate::CHAIN.lock();
            chain.blocks.len().saturating_sub(1) as u64
        };

        let peers = self.peer_manager.get_all_peers().await;
        let network_height = peers
            .iter()
            .filter_map(|p| p.height)
            .max()
            .unwrap_or(local_height);

        let blocks_behind = network_height.saturating_sub(local_height);

        // Critical: More than 100 blocks behind
        if blocks_behind > 100 {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Critical,
                timestamp: now,
                message: format!("Far behind network: {} blocks", blocks_behind),
                metric: "blocks_behind".to_string(),
                current_value: blocks_behind as f32,
                threshold: 100.0,
                recommendation: "Check sync process, ensure peers are responding".to_string(),
            });
        }
        // Warning: More than 10 blocks behind
        else if blocks_behind > 10 {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Warning,
                timestamp: now,
                message: format!("Syncing: {} blocks behind", blocks_behind),
                metric: "blocks_behind".to_string(),
                current_value: blocks_behind as f32,
                threshold: 10.0,
                recommendation: "Node is catching up, this is normal during sync".to_string(),
            });
        }

        alerts
    }

    /// Calculate overall health score
    pub async fn calculate_health_score(&self) -> HealthScore {
        let peers = self.peer_manager.get_all_peers().await;
        let connected = peers
            .iter()
            .filter(|p| p.state == PeerState::Connected)
            .count();

        // Connectivity score (0-100)
        let connectivity = if connected >= 10 {
            100
        } else if connected >= 5 {
            80
        } else if connected >= 2 {
            50
        } else if connected == 1 {
            25
        } else {
            0
        };

        // Performance score based on latency (0-100)
        let latencies: Vec<u32> = peers
            .iter()
            .filter(|p| p.state == PeerState::Connected)
            .filter_map(|p| p.metrics.latency_ms)
            .collect();

        let performance = if latencies.is_empty() {
            50 // Unknown
        } else {
            let avg = latencies.iter().sum::<u32>() as f32 / latencies.len() as f32;
            if avg < 100.0 {
                100
            } else if avg < 200.0 {
                80
            } else if avg < 500.0 {
                60
            } else if avg < 1000.0 {
                40
            } else {
                20
            }
        };

        // Stability score based on failure rates (0-100)
        let total_failures: u32 = peers.iter().map(|p| p.metrics.failure_count).sum();
        let total_attempts: u32 = peers
            .iter()
            .map(|p| p.metrics.success_count + p.metrics.failure_count)
            .sum();

        let stability = if total_attempts == 0 {
            50 // Unknown
        } else {
            let success_rate = (total_attempts - total_failures) as f32 / total_attempts as f32;
            (success_rate * 100.0) as u8
        };

        // Reputation score based on peer scores (0-100)
        let scores: Vec<f32> = peers.iter().map(|p| p.metrics.score).collect();
        let reputation = if scores.is_empty() {
            50 // Unknown
        } else {
            let avg_score = scores.iter().sum::<f32>() / scores.len() as f32;
            (avg_score * 100.0) as u8
        };

        // Overall score is weighted average
        let overall =
            (connectivity * 40 + performance * 30 + stability * 20 + reputation * 10) / 100;

        HealthScore {
            overall,
            connectivity,
            performance,
            stability,
            reputation,
        }
    }

    /// Get alert history
    pub async fn get_alert_history(&self, limit: usize) -> Vec<HealthAlert> {
        let history = self.alert_history.read().await;
        let start = history.len().saturating_sub(limit);
        history[start..].to_vec()
    }

    /// Start background monitoring task
    pub fn start_monitoring(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

                // Run health check every minute
                let _alerts = self.check_health().await;

                // Calculate and log health score
                let score = self.calculate_health_score().await;
                info!(
                    "[HEALTH] Score: {}% (connectivity: {}, performance: {}, stability: {}, reputation: {})",
                    score.overall, score.connectivity, score.performance, score.stability, score.reputation
                );
            }
        });
    }
}
