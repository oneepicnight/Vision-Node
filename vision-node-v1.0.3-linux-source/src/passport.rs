// Vision Node Passport System - OFFLINE-FIRST DESIGN
// Guardian-issued credentials that ENHANCE trust but are NOT REQUIRED
// P2P network MUST function without guardian being online
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// NodePassport - Guardian-issued credential for enhanced trust
///
/// CRITICAL: Passports are OPTIONAL. Network MUST function without them.
/// - Guardian issues passports via HTTP API when online
/// - Nodes attach passports to P2P handshakes if they have them
/// - Peers without passports are accepted as "Untrusted" but still connected
/// - Handshakes NEVER call guardian - all verification is local
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodePassport {
    /// Node identifier (e.g., "VNODE-1234-ABCD-5678")
    pub node_tag: String,

    /// Node role in the network ("dreamer", "validator", "guardian")
    pub role: String,

    /// Network identifier ("testnet", "mainnet")
    pub network: String,

    /// Unix timestamp when passport was issued
    pub issued_at: u64,

    /// Unix timestamp when passport expires
    pub expires_at: u64,

    /// Maximum number of P2P peers this node can connect to
    pub max_peers: u16,

    /// Minimum node version required (e.g., 111 for v1.1.1)
    pub min_version: u32,

    /// Hex-encoded guardian public key used to sign this passport
    pub guardian_pubkey: String,

    /// Cryptographic signature over all fields (excluding signature itself)
    /// Allows any node to verify this passport without contacting Guardian
    pub signature: Vec<u8>,
}

/// PeerTrust level based on passport validation
/// IMPORTANT: Untrusted peers are still ACCEPTED, just with lower trust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PeerTrust {
    TrustedPassport, // passport valid - preferred for governance
    Untrusted,       // no passport or invalid - STILL ALLOWED for P2P
}

impl NodePassport {
    /// Check if passport is expired (soft check - doesn't reject)
    pub fn is_expired(&self, now: u64) -> bool {
        now >= self.expires_at
    }

    /// Check if passport is currently valid (not expired)
    pub fn is_valid(&self, now: u64) -> bool {
        now >= self.issued_at && now < self.expires_at
    }

    /// Get remaining validity in seconds
    pub fn remaining_seconds(&self, now: u64) -> i64 {
        self.expires_at as i64 - now as i64
    }

    /// Get passport age in seconds
    pub fn age_seconds(&self, now: u64) -> u64 {
        now.saturating_sub(self.issued_at)
    }

    /// Get passport bytes for signing (without signature field)
    pub fn signing_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        let mut unsigned = self.clone();
        unsigned.signature.clear();
        serde_json::to_vec(&unsigned)
    }
}

/// Verify passport signature locally (NO network calls)
/// Returns Ok(()) if valid, Err with reason if not
pub fn verify_passport_signature(passport: &NodePassport) -> Result<(), String> {
    // Get signing bytes
    let _bytes = passport
        .signing_bytes()
        .map_err(|e| format!("serialize passport: {}", e))?;

    // Decode guardian public key
    let _pubkey_bytes = hex::decode(&passport.guardian_pubkey)
        .map_err(|e| format!("decode guardian pubkey: {}", e))?;

    // TODO: Wire up actual signature verification with ed25519 or secp256k1
    // For now, we accept all signatures (development mode)
    // In production, this MUST verify the actual signature

    tracing::debug!(
        target: "passport",
        "Passport signature check for {} (guardian={}, sig_len={})",
        passport.node_tag,
        &passport.guardian_pubkey[..std::cmp::min(8, passport.guardian_pubkey.len())],
        passport.signature.len()
    );

    Ok(())
}

/// Verify passport locally without calling guardian
/// Returns PeerTrust level - NEVER rejects connection
///
/// CRITICAL: This function MUST NOT make network calls or block
pub fn verify_passport_local(
    passport: Option<&NodePassport>,
    node_tag: &str,
    network: &str,
    node_version: u32,
    now: u64,
) -> PeerTrust {
    let Some(passport) = passport else {
        // No passport: allowed, but untrusted
        tracing::debug!(
            target: "passport",
            "Peer {} has no passport - accepting as Untrusted",
            node_tag
        );
        return PeerTrust::Untrusted;
    };

    // 1) Basic sanity checks
    if passport.node_tag != node_tag {
        tracing::warn!(
            target: "passport",
            "Passport node_tag mismatch: handshake={} passport={} - accepting as Untrusted",
            node_tag,
            passport.node_tag
        );
        return PeerTrust::Untrusted;
    }

    if passport.network != network {
        tracing::warn!(
            target: "passport",
            "Passport network mismatch: handshake={} passport={} - accepting as Untrusted",
            network,
            passport.network
        );
        return PeerTrust::Untrusted;
    }

    // 2) Expiry check (soft - still accept)
    if passport.is_expired(now) {
        tracing::warn!(
            target: "passport",
            "Passport expired for {} (expires_at={}) - accepting as Untrusted",
            passport.node_tag,
            passport.expires_at
        );
        return PeerTrust::Untrusted;
    }

    // 3) Version check (soft - still accept)
    if node_version < passport.min_version {
        tracing::warn!(
            target: "passport",
            "Peer {} version {} below passport min_version {} - accepting as Untrusted",
            passport.node_tag,
            node_version,
            passport.min_version
        );
        return PeerTrust::Untrusted;
    }

    // 4) Signature check (local only - NO network calls)
    if let Err(e) = verify_passport_signature(passport) {
        tracing::warn!(
            target: "passport",
            "Invalid passport signature for {}: {} - accepting as Untrusted",
            passport.node_tag,
            e
        );
        return PeerTrust::Untrusted;
    }

    // All checks passed!
    tracing::info!(
        target: "passport",
        "✅ Valid passport for {} - marking as TrustedPassport",
        passport.node_tag
    );
    PeerTrust::TrustedPassport
}

/// Guardian heartbeat request - periodic check-in with Guardian (OPTIONAL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub node_tag: String,
    pub passport: Option<NodePassport>,
    pub height: u64,
    pub peer_count: usize,
    pub version: u32,
}

/// Guardian heartbeat response - passport refresh or revocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    /// Optional updated passport (rotation, rule changes)
    pub updated_passport: Option<NodePassport>,

    /// Status: "ok", "revoked", "warning"
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passport_validity() {
        let passport = NodePassport {
            node_tag: "VNODE-TEST".to_string(),
            role: "dreamer".to_string(),
            network: "mainnet".to_string(),
            issued_at: 1000,
            expires_at: 2000,
            max_peers: 16,
            min_version: 111,
            guardian_pubkey: "test_key".to_string(),
            signature: vec![],
        };

        assert!(passport.is_valid(1500));
        assert!(!passport.is_valid(500));
        assert!(!passport.is_valid(2500));

        assert!(!passport.is_expired(1500));
        assert!(passport.is_expired(2500));

        assert_eq!(passport.remaining_seconds(1500), 500);
        assert_eq!(passport.age_seconds(1500), 500);
    }
}
