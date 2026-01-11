#![allow(dead_code)]
//! Reputation System - Adversarial Resilience for Vision Network
//!
//! **Phase 4: Adversarial Resilience & Reputation System**
//!
//! This module implements trust-based peer management to defend against
//! malicious actors, protocol violations, spam, and routing failures.
//!
//! **Key Features:**
//! - Misbehavior scoring with weighted penalties
//! - Trust level classification (Trusted → Banned)
//! - Temporal bans with automatic expiry
//! - Forgiveness through reputation decay
//! - Integration with routing decisions
//!
//! **Trust Levels:**
//! - Trusted: reputation >= 80.0, no recent issues
//! - Normal: reputation 40.0-79.9
//! - Probation: reputation 20.0-39.9
//! - Graylisted: misbehavior >= 30.0 (temporary ban)
//! - Banned: misbehavior >= 80.0 (long-term ban)
//!
//! **Decay Mechanism:**
//! Reputation improves by +5.0 per hour for non-banned peers
//! Misbehavior score decays by -5.0 per hour
//! Encourages redemption for previously problematic peers

use crate::p2p::peer_store::{PeerTrustLevel, VisionPeer};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

// ============================================================================
// MISBEHAVIOR TYPES & SCORING
// ============================================================================

/// Categories of peer misbehavior with associated penalties
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MisbehaviorKind {
    /// Sent invalid block (signature, PoW, structure issues)
    InvalidBlock,
    /// Sent invalid transaction (signature, balance, format issues)
    InvalidTransaction,
    /// Protocol violation (malformed messages, wrong handshake)
    ProtocolViolation,
    /// Spam behavior (duplicate invs, excessive messages)
    Spam,
    /// Relay failure (message didn't reach destination)
    RelayFailure,
    /// Connection flooding (excessive reconnects)
    ConnectionFlood,
}

impl MisbehaviorKind {
    /// Get the misbehavior score penalty for this kind
    pub fn penalty(&self) -> f32 {
        match self {
            Self::InvalidBlock => 25.0,
            Self::InvalidTransaction => 10.0,
            Self::ProtocolViolation => 20.0,
            Self::Spam => 15.0,
            Self::RelayFailure => 5.0,
            Self::ConnectionFlood => 30.0,
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::InvalidBlock => "Invalid block",
            Self::InvalidTransaction => "Invalid transaction",
            Self::ProtocolViolation => "Protocol violation",
            Self::Spam => "Spam behavior",
            Self::RelayFailure => "Relay failure",
            Self::ConnectionFlood => "Connection flooding",
        }
    }
}

// ============================================================================
// REPUTATION CONFIGURATION
// ============================================================================

/// Configuration for reputation system thresholds and behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationConfig {
    /// Graylist threshold (temporary ban)
    pub graylist_threshold: f32,

    /// Ban threshold (long-term ban)
    pub ban_threshold: f32,

    /// Graylist duration in seconds (default: 1 hour)
    pub graylist_duration_secs: i64,

    /// Ban duration in seconds (default: 24 hours)
    pub ban_duration_secs: i64,

    /// Reputation decay per hour (forgiveness rate)
    pub decay_per_hour: f32,

    /// Minimum reputation (floor)
    pub min_reputation: f32,

    /// Maximum reputation (ceiling)
    pub max_reputation: f32,

    /// Probation threshold (reputation below this = probation)
    pub probation_threshold: f32,

    /// Trusted threshold (reputation above this = trusted)
    pub trusted_threshold: f32,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            graylist_threshold: 30.0,
            ban_threshold: 80.0,
            graylist_duration_secs: 3600, // 1 hour
            ban_duration_secs: 86400,     // 24 hours
            decay_per_hour: 5.0,
            min_reputation: 0.0,
            max_reputation: 100.0,
            probation_threshold: 40.0,
            trusted_threshold: 80.0,
        }
    }
}

// ============================================================================
// REPUTATION SYSTEM OPERATIONS
// ============================================================================

/// Apply misbehavior penalty to a peer and update trust level
pub fn apply_misbehavior(peer: &mut VisionPeer, kind: MisbehaviorKind, config: &ReputationConfig) {
    let penalty = kind.penalty();
    peer.misbehavior_score += penalty;
    peer.reputation = (peer.reputation - penalty).max(config.min_reputation);

    // Update counters
    match kind {
        MisbehaviorKind::InvalidBlock | MisbehaviorKind::InvalidTransaction => {
            peer.total_invalid_msgs += 1;
        }
        MisbehaviorKind::ProtocolViolation => {
            peer.total_protocol_violations += 1;
        }
        MisbehaviorKind::Spam => {
            peer.total_spam_events += 1;
        }
        _ => {}
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Apply bans if thresholds exceeded
    if peer.misbehavior_score >= config.ban_threshold {
        peer.trust_level = PeerTrustLevel::Banned;
        peer.banned_until = Some(now + config.ban_duration_secs);
        warn!(
            "[reputation] Peer {} BANNED: {} (misbehavior: {:.1}, reputation: {:.1})",
            peer.node_tag,
            kind.description(),
            peer.misbehavior_score,
            peer.reputation
        );
        // Log to event store
        crate::api::routing_api::log_misbehavior_event(
            &peer.node_tag,
            kind.description(),
            peer.misbehavior_score,
            "banned",
        );
    } else if peer.misbehavior_score >= config.graylist_threshold {
        peer.trust_level = PeerTrustLevel::Graylisted;
        peer.graylisted_until = Some(now + config.graylist_duration_secs);
        warn!(
            "[reputation] Peer {} GRAYLISTED: {} (misbehavior: {:.1}, reputation: {:.1})",
            peer.node_tag,
            kind.description(),
            peer.misbehavior_score,
            peer.reputation
        );
        // Log to event store
        crate::api::routing_api::log_misbehavior_event(
            &peer.node_tag,
            kind.description(),
            peer.misbehavior_score,
            "graylisted",
        );
    } else {
        // Update trust level based on reputation
        update_trust_level(peer, config);
        debug!(
            "[reputation] Peer {} misbehavior: {} +{:.1} (total: {:.1}, reputation: {:.1})",
            peer.node_tag,
            kind.description(),
            penalty,
            peer.misbehavior_score,
            peer.reputation
        );
        // Log to event store (warning level)
        let trust_str = match peer.trust_level {
            PeerTrustLevel::Trusted => "trusted",
            PeerTrustLevel::Normal => "normal",
            PeerTrustLevel::Probation => "probation",
            PeerTrustLevel::Graylisted => "graylisted",
            PeerTrustLevel::Banned => "banned",
        };
        crate::api::routing_api::log_misbehavior_event(
            &peer.node_tag,
            kind.description(),
            peer.misbehavior_score,
            trust_str,
        );
    }
}

/// Update peer trust level based on current reputation score
pub fn update_trust_level(peer: &mut VisionPeer, config: &ReputationConfig) {
    // Don't change trust level if banned or graylisted
    if matches!(
        peer.trust_level,
        PeerTrustLevel::Banned | PeerTrustLevel::Graylisted
    ) {
        return;
    }

    peer.trust_level = if peer.reputation >= config.trusted_threshold {
        PeerTrustLevel::Trusted
    } else if peer.reputation >= config.probation_threshold {
        PeerTrustLevel::Normal
    } else {
        PeerTrustLevel::Probation
    };
}

/// Apply reputation decay (forgiveness mechanism)
pub fn decay_reputation(peer: &mut VisionPeer, config: &ReputationConfig, hours_elapsed: f32) {
    // Don't decay for banned peers
    if peer.trust_level == PeerTrustLevel::Banned {
        return;
    }

    let decay_amount = config.decay_per_hour * hours_elapsed;

    // Decay misbehavior score
    let old_misbehavior = peer.misbehavior_score;
    peer.misbehavior_score = (peer.misbehavior_score - decay_amount).max(0.0);

    // Improve reputation
    let old_reputation = peer.reputation;
    peer.reputation = (peer.reputation + decay_amount).min(config.max_reputation);

    if old_misbehavior != peer.misbehavior_score || old_reputation != peer.reputation {
        debug!(
            "[reputation] Decay applied: {} misbehavior {:.1} → {:.1}, reputation {:.1} → {:.1}",
            peer.node_tag, old_misbehavior, peer.misbehavior_score, old_reputation, peer.reputation
        );
    }

    // Update trust level after decay
    update_trust_level(peer, config);
}

/// Check and clear expired bans/graylists
pub fn check_ban_expiry(peer: &mut VisionPeer, config: &ReputationConfig) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mut changed = false;

    // Check graylist expiry
    if let Some(until) = peer.graylisted_until {
        if now >= until {
            peer.graylisted_until = None;
            info!(
                "[reputation] Peer {} graylist expired (misbehavior: {:.1})",
                peer.node_tag, peer.misbehavior_score
            );
            update_trust_level(peer, config);
            changed = true;

            // Log to event store
            let trust_str = match peer.trust_level {
                PeerTrustLevel::Trusted => "trusted",
                PeerTrustLevel::Normal => "normal",
                PeerTrustLevel::Probation => "probation",
                _ => "unknown",
            };
            crate::api::routing_api::log_ban_expiry_event(&peer.node_tag, trust_str);
        }
    }

    // Check ban expiry
    if let Some(until) = peer.banned_until {
        if now >= until {
            peer.banned_until = None;
            info!(
                "[reputation] Peer {} ban expired (misbehavior: {:.1})",
                peer.node_tag, peer.misbehavior_score
            );
            update_trust_level(peer, config);
            changed = true;

            // Log to event store
            let trust_str = match peer.trust_level {
                PeerTrustLevel::Trusted => "trusted",
                PeerTrustLevel::Normal => "normal",
                PeerTrustLevel::Probation => "probation",
                _ => "unknown",
            };
            crate::api::routing_api::log_ban_expiry_event(&peer.node_tag, trust_str);
        }
    }

    changed
}

/// Get reputation factor for routing score (0.1 to 1.2)
pub fn reputation_factor(peer: &VisionPeer) -> f32 {
    match peer.trust_level {
        PeerTrustLevel::Trusted => 1.2,    // 20% bonus
        PeerTrustLevel::Normal => 1.0,     // No adjustment
        PeerTrustLevel::Probation => 0.6,  // 40% penalty
        PeerTrustLevel::Graylisted => 0.1, // 90% penalty
        PeerTrustLevel::Banned => 0.0,     // Cannot route
    }
}

/// Check if peer should be excluded from routing
pub fn is_excluded_from_routing(peer: &VisionPeer) -> bool {
    matches!(
        peer.trust_level,
        PeerTrustLevel::Banned | PeerTrustLevel::Graylisted
    )
}

// ============================================================================
// ROUTE LEARNING & EFFECTIVENESS
// ============================================================================

/// Mark successful route delivery and update effectiveness metrics
pub fn mark_route_success(peer: &mut VisionPeer, delivery_time_ms: u32) {
    peer.route_uses += 1;
    peer.route_successes += 1;

    // Update average delivery time (EMA with alpha=0.2)
    if let Some(avg) = peer.avg_delivery_ms {
        peer.avg_delivery_ms = Some((avg as f32 * 0.8 + delivery_time_ms as f32 * 0.2) as u32);
    } else {
        peer.avg_delivery_ms = Some(delivery_time_ms);
    }

    debug!(
        "[route_learning] Peer {} route success: {} uses, {}/{} success rate, avg {}ms",
        peer.node_tag,
        peer.route_uses,
        peer.route_successes,
        peer.route_uses,
        peer.avg_delivery_ms.unwrap_or(0)
    );
}

/// Mark failed route delivery and update effectiveness metrics
pub fn mark_route_failure(peer: &mut VisionPeer) {
    peer.route_uses += 1;
    peer.route_failures += 1;

    debug!(
        "[route_learning] Peer {} route failure: {} uses, {}/{} success rate",
        peer.node_tag, peer.route_uses, peer.route_successes, peer.route_uses
    );
}

/// Calculate route success rate (0.0 to 1.0)
pub fn route_success_rate(peer: &VisionPeer) -> f32 {
    if peer.route_uses == 0 {
        return 0.5; // Neutral for untested peers
    }
    peer.route_successes as f32 / peer.route_uses as f32
}

/// Get route performance score for routing decisions (0-20 points)
pub fn route_performance_score(peer: &VisionPeer) -> f32 {
    if peer.route_uses == 0 {
        return 0.0; // No data yet
    }

    let success_rate = route_success_rate(peer);
    let base_score = success_rate * 20.0;

    // Delivery speed bonus: faster = better (max +5 points)
    let delivery_bonus = if let Some(avg_ms) = peer.avg_delivery_ms {
        match avg_ms {
            0..=50 => 5.0,
            51..=100 => 4.0,
            101..=200 => 3.0,
            201..=500 => 2.0,
            501..=1000 => 1.0,
            _ => 0.0,
        }
    } else {
        0.0
    };

    base_score + delivery_bonus
}

// ============================================================================
// BACKGROUND MAINTENANCE TASK
// ============================================================================

/// Start reputation maintenance background task
///
/// This task runs periodically to:
/// 1. Decay misbehavior scores (forgiveness)
/// 2. Improve reputation over time for good peers
/// 3. Check and expire temporary bans/graylists
/// 4. Update trust levels based on current scores
///
/// **Interval:** Every 1 hour (3600 seconds)
pub async fn start_reputation_maintenance(db: sled::Db) {
    use crate::p2p::peer_store::PeerStore;
    use tokio::time::{sleep, Duration};

    info!("[reputation] Starting reputation maintenance task (decay every 1 hour)");

    let mut last_run = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    loop {
        sleep(Duration::from_secs(3600)).await; // Run every hour

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let hours_elapsed = ((now - last_run) as f32) / 3600.0;

        if let Ok(peer_store) = PeerStore::new(&db) {
            let config = ReputationConfig::default();
            let mut peers = peer_store.all();
            let mut updated_count = 0;
            let mut expired_bans = 0;

            for peer in &mut peers {
                let old_trust = peer.trust_level;
                let old_reputation = peer.reputation;
                let old_misbehavior = peer.misbehavior_score;

                // Decay misbehavior and improve reputation
                decay_reputation(peer, &config, hours_elapsed);

                // Check and expire bans/graylists
                if check_ban_expiry(peer, &config) {
                    expired_bans += 1;
                }

                // Update peer if anything changed
                if peer.trust_level != old_trust
                    || (peer.reputation - old_reputation).abs() > 0.01
                    || (peer.misbehavior_score - old_misbehavior).abs() > 0.01
                {
                    let _ = peer_store.upsert(peer.clone());
                    updated_count += 1;
                }
            }

            if updated_count > 0 {
                info!(
                    "[reputation] Maintenance complete: {} peers updated, {} bans expired",
                    updated_count, expired_bans
                );

                // Log cluster balance event
                let classified = peer_store.classify_peers_for_routing(None);
                let mut inner_count = 0;
                let mut middle_count = 0;
                let mut outer_count = 0;

                for classified_peer in classified {
                    match classified_peer.ring {
                        crate::p2p::peer_store::PeerRing::Inner => inner_count += 1,
                        crate::p2p::peer_store::PeerRing::Middle => middle_count += 1,
                        crate::p2p::peer_store::PeerRing::Outer => outer_count += 1,
                    }
                }

                crate::api::routing_api::log_cluster_balance_event(
                    inner_count,
                    middle_count,
                    outer_count,
                );
            }
        } else {
            warn!("[reputation] Failed to open peer store for maintenance");
        }

        last_run = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_misbehavior_penalties() {
        assert_eq!(MisbehaviorKind::InvalidBlock.penalty(), 25.0);
        assert_eq!(MisbehaviorKind::InvalidTransaction.penalty(), 10.0);
        assert_eq!(MisbehaviorKind::ProtocolViolation.penalty(), 20.0);
        assert_eq!(MisbehaviorKind::Spam.penalty(), 15.0);
        assert_eq!(MisbehaviorKind::RelayFailure.penalty(), 5.0);
        assert_eq!(MisbehaviorKind::ConnectionFlood.penalty(), 30.0);
    }

    #[test]
    fn test_reputation_thresholds() {
        let config = ReputationConfig::default();
        assert_eq!(config.graylist_threshold, 30.0);
        assert_eq!(config.ban_threshold, 80.0);
        assert_eq!(config.trusted_threshold, 80.0);
        assert_eq!(config.probation_threshold, 40.0);
    }

    #[test]
    fn test_reputation_factor() {
        let mut peer = VisionPeer::new(
            "test".to_string(),
            "TEST".to_string(),
            "pubkey".to_string(),
            "vision://test".to_string(),
            None,
            "constellation".to_string(),
        );

        peer.trust_level = PeerTrustLevel::Trusted;
        assert_eq!(reputation_factor(&peer), 1.2);

        peer.trust_level = PeerTrustLevel::Normal;
        assert_eq!(reputation_factor(&peer), 1.0);

        peer.trust_level = PeerTrustLevel::Probation;
        assert_eq!(reputation_factor(&peer), 0.6);

        peer.trust_level = PeerTrustLevel::Graylisted;
        assert_eq!(reputation_factor(&peer), 0.1);

        peer.trust_level = PeerTrustLevel::Banned;
        assert_eq!(reputation_factor(&peer), 0.0);
    }
}
