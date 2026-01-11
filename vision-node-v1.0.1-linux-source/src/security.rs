//! Security Hardening Module
//! 
//! Implements security features for wallet operations:
//! - Rate limiting for send operations
//! - Transaction amount limits
//! - Audit trail for all transactions
//! - Security event logging

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use once_cell::sync::Lazy;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc, Timelike};

use crate::market::engine::QuoteAsset;

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per time window
    pub max_requests: u32,
    /// Time window in seconds
    pub window_secs: u64,
    /// Cooldown period after limit exceeded (seconds)
    pub cooldown_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 10,       // 10 sends per window
            window_secs: 60,        // 1 minute window
            cooldown_secs: 300,     // 5 minute cooldown
        }
    }
}

/// Amount limit configuration per asset
#[derive(Debug, Clone)]
pub struct AmountLimitConfig {
    /// Maximum single transaction amount (in base units)
    pub max_single_tx: f64,
    /// Maximum daily transaction volume (in base units)
    pub max_daily_volume: f64,
    /// Daily limit reset time (UTC hour)
    pub daily_reset_hour: u8,
}

impl AmountLimitConfig {
    pub fn btc_default() -> Self {
        Self {
            max_single_tx: 1.0,      // 1 BTC per transaction
            max_daily_volume: 10.0,  // 10 BTC per day
            daily_reset_hour: 0,     // Midnight UTC
        }
    }
    
    pub fn bch_default() -> Self {
        Self {
            max_single_tx: 10.0,     // 10 BCH per transaction
            max_daily_volume: 100.0, // 100 BCH per day
            daily_reset_hour: 0,
        }
    }
    
    pub fn doge_default() -> Self {
        Self {
            max_single_tx: 10000.0,     // 10k DOGE per transaction
            max_daily_volume: 100000.0, // 100k DOGE per day
            daily_reset_hour: 0,
        }
    }
}

/// Rate limiter tracking per user
#[derive(Debug, Clone)]
struct RateLimitState {
    requests: Vec<u64>,
    cooldown_until: Option<u64>,
}

/// Daily volume tracking per user and asset
#[derive(Debug, Clone)]
struct VolumeTracker {
    volume: f64,
    last_reset: DateTime<Utc>,
}

/// Audit trail entry types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    SendInitiated,
    SendCompleted,
    SendFailed,
    RateLimitExceeded,
    AmountLimitExceeded,
    SuspiciousActivity,
}

/// Audit trail entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub user_id: String,
    pub event_type: AuditEventType,
    pub asset: String,
    pub amount: Option<f64>,
    pub to_address: Option<String>,
    pub txid: Option<String>,
    pub ip_address: Option<String>,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl AuditEntry {
    pub fn new(user_id: &str, event_type: AuditEventType) -> Self {
        let now = Utc::now();
        let id = format!("audit_{}_{}", user_id, now.timestamp_nanos_opt().unwrap_or(0));
        
        Self {
            id,
            timestamp: now,
            user_id: user_id.to_string(),
            event_type,
            asset: String::new(),
            amount: None,
            to_address: None,
            txid: None,
            ip_address: None,
            error_message: None,
            metadata: HashMap::new(),
        }
    }
    
    pub fn with_asset(mut self, asset: QuoteAsset) -> Self {
        self.asset = asset.as_str().to_string();
        self
    }
    
    pub fn with_amount(mut self, amount: f64) -> Self {
        self.amount = Some(amount);
        self
    }
    
    pub fn with_address(mut self, address: &str) -> Self {
        self.to_address = Some(address.to_string());
        self
    }
    
    pub fn with_txid(mut self, txid: &str) -> Self {
        self.txid = Some(txid.to_string());
        self
    }
    
    pub fn with_error(mut self, error: &str) -> Self {
        self.error_message = Some(error.to_string());
        self
    }
    
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// Global security state
static RATE_LIMIT_STATE: Lazy<Arc<Mutex<HashMap<String, RateLimitState>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

static VOLUME_TRACKER: Lazy<Arc<Mutex<HashMap<(String, QuoteAsset), VolumeTracker>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

static AUDIT_TRAIL: Lazy<Arc<Mutex<Vec<AuditEntry>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

/// Security manager
pub struct SecurityManager;

impl SecurityManager {
    /// Check rate limit for user
    pub fn check_rate_limit(user_id: &str, config: &RateLimitConfig) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut state = RATE_LIMIT_STATE.lock()
            .map_err(|e| anyhow!("Failed to lock rate limit state: {}", e))?;
        
        let user_state = state.entry(user_id.to_string())
            .or_insert_with(|| RateLimitState {
                requests: Vec::new(),
                cooldown_until: None,
            });
        
        // Check cooldown
        if let Some(cooldown_until) = user_state.cooldown_until {
            if now < cooldown_until {
                let remaining = cooldown_until - now;
                return Err(anyhow!(
                    "Rate limit cooldown active. {} seconds remaining",
                    remaining
                ));
            } else {
                user_state.cooldown_until = None;
                user_state.requests.clear();
            }
        }
        
        // Remove requests outside window
        let window_start = now.saturating_sub(config.window_secs);
        user_state.requests.retain(|&req_time| req_time > window_start);
        
        // Check if limit exceeded
        if user_state.requests.len() >= config.max_requests as usize {
            user_state.cooldown_until = Some(now + config.cooldown_secs);
            
            // Log audit event
            Self::log_audit(
                AuditEntry::new(user_id, AuditEventType::RateLimitExceeded)
                    .with_metadata("max_requests", &config.max_requests.to_string())
                    .with_metadata("window_secs", &config.window_secs.to_string())
            );
            
            tracing::warn!(
                "Rate limit exceeded for user {}: {} requests in {} seconds",
                user_id, user_state.requests.len(), config.window_secs
            );
            
            return Err(anyhow!(
                "Rate limit exceeded: {} requests in {} seconds. Cooldown: {} seconds",
                config.max_requests,
                config.window_secs,
                config.cooldown_secs
            ));
        }
        
        // Record request
        user_state.requests.push(now);
        
        Ok(())
    }
    
    /// Check amount limits
    pub fn check_amount_limit(
        user_id: &str,
        asset: QuoteAsset,
        amount: f64,
        config: &AmountLimitConfig,
    ) -> Result<()> {
        // Check single transaction limit
        if amount > config.max_single_tx {
            Self::log_audit(
                AuditEntry::new(user_id, AuditEventType::AmountLimitExceeded)
                    .with_asset(asset)
                    .with_amount(amount)
                    .with_metadata("limit_type", "single_tx")
                    .with_metadata("max_amount", &config.max_single_tx.to_string())
            );
            
            return Err(anyhow!(
                "Transaction amount {} exceeds single transaction limit of {}",
                amount,
                config.max_single_tx
            ));
        }
        
        // Check daily volume limit
        let now = Utc::now();
        let mut tracker = VOLUME_TRACKER.lock()
            .map_err(|e| anyhow!("Failed to lock volume tracker: {}", e))?;
        
        let key = (user_id.to_string(), asset);
        let volume_state = tracker.entry(key.clone())
            .or_insert_with(|| VolumeTracker {
                volume: 0.0,
                last_reset: now,
            });
        
        // Reset daily volume if needed
        let reset_hour = config.daily_reset_hour as u32;
        let should_reset = now.time().hour() == reset_hour && 
                          volume_state.last_reset.date_naive() < now.date_naive();
        
        if should_reset {
            volume_state.volume = 0.0;
            volume_state.last_reset = now;
        }
        
        // Check if adding this amount would exceed daily limit
        let new_volume = volume_state.volume + amount;
        if new_volume > config.max_daily_volume {
            Self::log_audit(
                AuditEntry::new(user_id, AuditEventType::AmountLimitExceeded)
                    .with_asset(asset)
                    .with_amount(amount)
                    .with_metadata("limit_type", "daily_volume")
                    .with_metadata("current_volume", &volume_state.volume.to_string())
                    .with_metadata("max_daily", &config.max_daily_volume.to_string())
            );
            
            return Err(anyhow!(
                "Daily volume limit exceeded: current {} + {} = {} exceeds limit of {}",
                volume_state.volume,
                amount,
                new_volume,
                config.max_daily_volume
            ));
        }
        
        // Update volume
        volume_state.volume = new_volume;
        
        Ok(())
    }
    
    /// Log audit entry
    pub fn log_audit(entry: AuditEntry) {
        if let Ok(mut audit) = AUDIT_TRAIL.lock() {
            tracing::info!(
                "🔒 AUDIT: user={}, event={:?}, asset={}, amount={:?}",
                entry.user_id,
                entry.event_type,
                entry.asset,
                entry.amount
            );
            
            audit.push(entry);
            
            // Keep last 10000 entries
            if audit.len() > 10000 {
                audit.drain(0..1000);
            }
        }
    }
    
    /// Get audit trail for user
    pub fn get_user_audit_trail(user_id: &str, limit: usize) -> Vec<AuditEntry> {
        AUDIT_TRAIL.lock()
            .ok()
            .map(|audit| {
                let mut entries: Vec<_> = audit.iter()
                    .filter(|e| e.user_id == user_id)
                    .cloned()
                    .collect();
                entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                entries.into_iter().take(limit).collect()
            })
            .unwrap_or_default()
    }
    
    /// Get all audit entries (admin only)
    pub fn get_all_audit_trail(limit: usize) -> Vec<AuditEntry> {
        AUDIT_TRAIL.lock()
            .ok()
            .map(|audit| {
                let mut entries: Vec<_> = audit.iter().cloned().collect();
                entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                entries.into_iter().take(limit).collect()
            })
            .unwrap_or_default()
    }
    
    /// Get rate limit status for user
    pub fn get_rate_limit_status(user_id: &str, config: &RateLimitConfig) -> (u32, Option<u64>) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let state = RATE_LIMIT_STATE.lock().ok();
        if let Some(state) = state {
            if let Some(user_state) = state.get(user_id) {
                // Check cooldown
                if let Some(cooldown_until) = user_state.cooldown_until {
                    if now < cooldown_until {
                        return (0, Some(cooldown_until - now));
                    }
                }
                
                // Count requests in current window
                let window_start = now.saturating_sub(config.window_secs);
                let count = user_state.requests.iter()
                    .filter(|&&req_time| req_time > window_start)
                    .count() as u32;
                
                return (config.max_requests - count, None);
            }
        }
        
        (config.max_requests, None)
    }
    
    /// Get volume status for user and asset
    pub fn get_volume_status(
        user_id: &str,
        asset: QuoteAsset,
        config: &AmountLimitConfig,
    ) -> (f64, f64) {
        let tracker = VOLUME_TRACKER.lock().ok();
        if let Some(tracker) = tracker {
            let key = (user_id.to_string(), asset);
            if let Some(volume_state) = tracker.get(&key) {
                let remaining = config.max_daily_volume - volume_state.volume;
                return (volume_state.volume, remaining.max(0.0));
            }
        }
        
        (0.0, config.max_daily_volume)
    }
    
    /// Reset rate limit for user (admin function)
    pub fn reset_rate_limit(user_id: &str) -> Result<()> {
        let mut state = RATE_LIMIT_STATE.lock()
            .map_err(|e| anyhow!("Failed to lock rate limit state: {}", e))?;
        
        state.remove(user_id);
        tracing::info!("Reset rate limit for user: {}", user_id);
        
        Ok(())
    }
    
    /// Clear old audit entries
    pub fn cleanup_old_audits(days: i64) {
        if let Ok(mut audit) = AUDIT_TRAIL.lock() {
            let cutoff = Utc::now() - chrono::Duration::days(days);
            let before = audit.len();
            audit.retain(|e| e.timestamp > cutoff);
            let after = audit.len();
            
            if before != after {
                tracing::info!("Cleaned up {} old audit entries", before - after);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_enforcement() {
        let config = RateLimitConfig {
            max_requests: 3,
            window_secs: 60,
            cooldown_secs: 10,
        };
        
        let user_id = "test_user_rate";
        
        // First 3 requests should pass
        assert!(SecurityManager::check_rate_limit(user_id, &config).is_ok());
        assert!(SecurityManager::check_rate_limit(user_id, &config).is_ok());
        assert!(SecurityManager::check_rate_limit(user_id, &config).is_ok());
        
        // 4th request should fail
        assert!(SecurityManager::check_rate_limit(user_id, &config).is_err());
    }

    #[test]
    fn test_amount_limit_single_tx() {
        let config = AmountLimitConfig::btc_default();
        let result = SecurityManager::check_amount_limit(
            "test_user_amount",
            QuoteAsset::Btc,
            2.0, // Exceeds 1 BTC limit
            &config,
        );
        
        assert!(result.is_err());
    }

    #[test]
    fn test_audit_logging() {
        let entry = AuditEntry::new("test_user", AuditEventType::SendInitiated)
            .with_asset(QuoteAsset::Btc)
            .with_amount(0.5)
            .with_address("bc1q...");
        
        SecurityManager::log_audit(entry);
        
        let entries = SecurityManager::get_user_audit_trail("test_user", 10);
        assert!(!entries.is_empty());
    }
}
