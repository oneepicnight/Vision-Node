// Atomic Swaps - Cross-chain trading without exchanges
// Implements Hash Time Locked Contracts (HTLC) for trustless swaps

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc, Duration};

use crate::market::engine::QuoteAsset;

// ============================================================================
// ATOMIC SWAP TYPES
// ============================================================================

/// Atomic swap status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SwapStatus {
    /// Swap initiated, waiting for counterparty
    Initiated,
    /// Counterparty accepted, creating HTLCs
    Accepted,
    /// HTLCs created on both chains
    Locked,
    /// Initiator claimed on chain B
    ClaimedByInitiator,
    /// Counterparty claimed on chain A
    ClaimedByCounterparty,
    /// Swap completed successfully
    Completed,
    /// Swap refunded (timeout)
    Refunded,
    /// Swap failed or cancelled
    Failed,
}

/// Atomic swap between two assets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicSwap {
    /// Unique swap ID
    pub swap_id: String,
    /// Initiator user ID
    pub initiator: String,
    /// Counterparty user ID
    pub counterparty: Option<String>,
    /// Asset being sent by initiator
    pub send_asset: QuoteAsset,
    /// Amount being sent (satoshis)
    pub send_amount: u64,
    /// Asset being received by initiator
    pub receive_asset: QuoteAsset,
    /// Amount being received (satoshis)
    pub receive_amount: u64,
    /// Secret hash (SHA256)
    pub secret_hash: String,
    /// Secret preimage (only known to initiator initially)
    pub secret: Option<String>,
    /// Initiator's HTLC contract address/txid
    pub initiator_htlc_txid: Option<String>,
    /// Counterparty's HTLC contract address/txid
    pub counterparty_htlc_txid: Option<String>,
    /// Timelock for initiator (block height)
    pub initiator_timelock: u32,
    /// Timelock for counterparty (must be earlier)
    pub counterparty_timelock: u32,
    /// Initiator's refund address
    pub initiator_refund_address: String,
    /// Counterparty's refund address
    pub counterparty_refund_address: Option<String>,
    /// Swap status
    pub status: SwapStatus,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Expiry timestamp
    pub expires_at: DateTime<Utc>,
}

impl AtomicSwap {
    pub fn new(
        initiator: String,
        send_asset: QuoteAsset,
        send_amount: u64,
        receive_asset: QuoteAsset,
        receive_amount: u64,
        refund_address: String,
        timeout_hours: i64,
    ) -> Result<Self> {
        if send_asset == receive_asset {
            return Err(anyhow!("Cannot swap same asset"));
        }
        
        if send_asset == QuoteAsset::Land || receive_asset == QuoteAsset::Land {
            return Err(anyhow!("LAND swaps not yet supported"));
        }
        
        let secret = Self::generate_secret();
        let secret_hash = Self::hash_secret(&secret);
        let now = Utc::now();
        let expires_at = now + Duration::hours(timeout_hours);
        
        // Timelocks: counterparty must be shorter to give initiator time to claim
        let initiator_timelock = (now + Duration::hours(timeout_hours)).timestamp() as u32;
        let counterparty_timelock = (now + Duration::hours(timeout_hours - 2)).timestamp() as u32;
        
        Ok(Self {
            swap_id: uuid::Uuid::new_v4().to_string(),
            initiator,
            counterparty: None,
            send_asset,
            send_amount,
            receive_asset,
            receive_amount,
            secret_hash,
            secret: Some(secret),
            initiator_htlc_txid: None,
            counterparty_htlc_txid: None,
            initiator_timelock,
            counterparty_timelock,
            initiator_refund_address: refund_address,
            counterparty_refund_address: None,
            status: SwapStatus::Initiated,
            created_at: now,
            updated_at: now,
            expires_at,
        })
    }
    
    fn generate_secret() -> String {
        use ring::rand::SecureRandom;
        let rng = ring::rand::SystemRandom::new();
        let mut secret = [0u8; 32];
        rng.fill(&mut secret).expect("Failed to generate secret");
        hex::encode(secret)
    }
    
    /// Hash secret using SHA256 (standard for atomic swaps)
    /// Cross-chain compatibility: Bitcoin, Ethereum use SHA256 for HTLCs
    /// CRITICAL: Use swap::htlc_hash_lock_hex() instead of BLAKE3
    fn hash_secret(secret: &str) -> String {
        let bytes = hex::decode(secret).expect("Invalid secret");
        crate::swap::htlc_hash_lock_hex(&bytes)
    }
    
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
    
    /// Verify secret matches hash
    pub fn verify_secret(&self, secret: &str) -> bool {
        Self::hash_secret(secret) == self.secret_hash
    }
}

// ============================================================================
// HTLC (Hash Time Locked Contract)
// ============================================================================

/// HTLC script for atomic swaps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtlcScript {
    /// Secret hash (SHA256)
    pub hash: String,
    /// Recipient address (can claim with secret)
    pub recipient: String,
    /// Refund address (can claim after timelock)
    pub refund: String,
    /// Timelock (block height or timestamp)
    pub timelock: u32,
    /// Asset type
    pub asset: QuoteAsset,
    /// Amount locked
    pub amount: u64,
}

impl HtlcScript {
    pub fn new(
        hash: String,
        recipient: String,
        refund: String,
        timelock: u32,
        asset: QuoteAsset,
        amount: u64,
    ) -> Self {
        Self {
            hash,
            recipient,
            refund,
            timelock,
            asset,
            amount,
        }
    }
    
    /// Generate HTLC script (Bitcoin Script)
    pub fn to_script(&self) -> String {
        // Simplified Bitcoin Script for HTLC
        // OP_IF
        //   OP_SHA256 <hash> OP_EQUALVERIFY <recipient_pubkey> OP_CHECKSIG
        // OP_ELSE
        //   <timelock> OP_CHECKLOCKTIMEVERIFY OP_DROP <refund_pubkey> OP_CHECKSIG
        // OP_ENDIF
        
        format!(
            "IF SHA256 {} EQUALVERIFY {} CHECKSIG ELSE {} CHECKLOCKTIMEVERIFY DROP {} CHECKSIG ENDIF",
            self.hash,
            self.recipient,
            self.timelock,
            self.refund
        )
    }
    
    /// Create HTLC transaction
    pub async fn create_htlc_tx(&self, funding_txid: &str) -> Result<String> {
        // In production, this would:
        // 1. Create transaction with HTLC script as output
        // 2. Sign and broadcast to blockchain
        // 3. Return transaction ID
        
        let htlc_txid = format!(
            "htlc_{}_{}_{}",
            self.asset.as_str(),
            &self.hash[0..8],
            Utc::now().timestamp()
        );
        
        tracing::info!(
            "🔒 Created HTLC: {} for {} {} (timelock: {})",
            htlc_txid,
            self.amount,
            self.asset.as_str(),
            self.timelock
        );
        
        Ok(htlc_txid)
    }
    
    /// Claim HTLC with secret
    pub async fn claim_htlc(&self, secret: &str, htlc_txid: &str) -> Result<String> {
        // Verify secret
        let hash = AtomicSwap::hash_secret(secret);
        if hash != self.hash {
            return Err(anyhow!("Invalid secret"));
        }
        
        // In production, this would:
        // 1. Create transaction spending HTLC output
        // 2. Include secret in witness/scriptSig
        // 3. Sign and broadcast
        
        let claim_txid = format!(
            "claim_{}_{}",
            htlc_txid,
            Utc::now().timestamp()
        );
        
        tracing::info!(
            "✅ Claimed HTLC: {} → {}",
            htlc_txid,
            claim_txid
        );
        
        Ok(claim_txid)
    }
    
    /// Refund HTLC after timelock
    pub async fn refund_htlc(&self, htlc_txid: &str, current_height: u32) -> Result<String> {
        // Check timelock
        if current_height < self.timelock {
            return Err(anyhow!("Timelock not yet expired"));
        }
        
        // In production, this would:
        // 1. Create transaction spending HTLC output
        // 2. Wait for timelock to expire
        // 3. Sign and broadcast refund
        
        let refund_txid = format!(
            "refund_{}_{}",
            htlc_txid,
            Utc::now().timestamp()
        );
        
        tracing::info!(
            "🔄 Refunded HTLC: {} → {}",
            htlc_txid,
            refund_txid
        );
        
        Ok(refund_txid)
    }
}

// ============================================================================
// ATOMIC SWAP MANAGER
// ============================================================================

pub static SWAPS: Lazy<Arc<Mutex<HashMap<String, AtomicSwap>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub struct AtomicSwapManager;

impl AtomicSwapManager {
    /// Initiate a new atomic swap
    pub fn initiate_swap(
        user_id: &str,
        send_asset: QuoteAsset,
        send_amount: u64,
        receive_asset: QuoteAsset,
        receive_amount: u64,
        refund_address: String,
        timeout_hours: i64,
    ) -> Result<AtomicSwap> {
        let swap = AtomicSwap::new(
            user_id.to_string(),
            send_asset,
            send_amount,
            receive_asset,
            receive_amount,
            refund_address,
            timeout_hours,
        )?;
        
        let swap_id = swap.swap_id.clone();
        
        let mut swaps = SWAPS.lock()
            .map_err(|e| anyhow!("Failed to lock swaps: {}", e))?;
        
        swaps.insert(swap_id.clone(), swap.clone());
        
        tracing::info!(
            "🔄 Initiated swap: {} {} → {} {}",
            send_amount,
            send_asset.as_str(),
            receive_amount,
            receive_asset.as_str()
        );
        
        Ok(swap)
    }
    
    /// Accept a swap offer
    pub async fn accept_swap(
        swap_id: &str,
        counterparty: &str,
        refund_address: String,
    ) -> Result<AtomicSwap> {
        let mut swaps = SWAPS.lock()
            .map_err(|e| anyhow!("Failed to lock swaps: {}", e))?;
        
        let swap = swaps.get_mut(swap_id)
            .ok_or_else(|| anyhow!("Swap not found"))?;
        
        if swap.status != SwapStatus::Initiated {
            return Err(anyhow!("Swap already accepted or completed"));
        }
        
        if swap.is_expired() {
            swap.status = SwapStatus::Failed;
            return Err(anyhow!("Swap expired"));
        }
        
        swap.counterparty = Some(counterparty.to_string());
        swap.counterparty_refund_address = Some(refund_address.clone());
        swap.status = SwapStatus::Accepted;
        swap.updated_at = Utc::now();
        
        tracing::info!("✅ Swap accepted by {}: {}", counterparty, swap_id);
        
        Ok(swap.clone())
    }
    
    /// Create HTLC contracts for both parties
    pub async fn lock_swap(swap_id: &str) -> Result<(String, String)> {
        let mut swaps = SWAPS.lock()
            .map_err(|e| anyhow!("Failed to lock swaps: {}", e))?;
        
        let swap = swaps.get_mut(swap_id)
            .ok_or_else(|| anyhow!("Swap not found"))?;
        
        if swap.status != SwapStatus::Accepted {
            return Err(anyhow!("Swap not accepted"));
        }
        
        let counterparty = swap.counterparty.as_ref()
            .ok_or_else(|| anyhow!("No counterparty"))?;
        
        let counterparty_refund = swap.counterparty_refund_address.as_ref()
            .ok_or_else(|| anyhow!("No counterparty refund address"))?;
        
        // Create initiator's HTLC (send_asset)
        let initiator_htlc = HtlcScript::new(
            swap.secret_hash.clone(),
            counterparty.clone(),
            swap.initiator_refund_address.clone(),
            swap.initiator_timelock,
            swap.send_asset,
            swap.send_amount,
        );
        
        let initiator_htlc_txid = initiator_htlc.create_htlc_tx("funding_tx").await?;
        
        // Create counterparty's HTLC (receive_asset)
        let counterparty_htlc = HtlcScript::new(
            swap.secret_hash.clone(),
            swap.initiator.clone(),
            counterparty_refund.clone(),
            swap.counterparty_timelock,
            swap.receive_asset,
            swap.receive_amount,
        );
        
        let counterparty_htlc_txid = counterparty_htlc.create_htlc_tx("funding_tx").await?;
        
        swap.initiator_htlc_txid = Some(initiator_htlc_txid.clone());
        swap.counterparty_htlc_txid = Some(counterparty_htlc_txid.clone());
        swap.status = SwapStatus::Locked;
        swap.updated_at = Utc::now();
        
        tracing::info!("🔒 Swap locked: {}", swap_id);
        
        Ok((initiator_htlc_txid, counterparty_htlc_txid))
    }
    
    /// Claim counterparty's HTLC (initiator claims receive_asset)
    pub async fn claim_initiator(swap_id: &str) -> Result<String> {
        let mut swaps = SWAPS.lock()
            .map_err(|e| anyhow!("Failed to lock swaps: {}", e))?;
        
        let swap = swaps.get_mut(swap_id)
            .ok_or_else(|| anyhow!("Swap not found"))?;
        
        if swap.status != SwapStatus::Locked {
            return Err(anyhow!("Swap not locked"));
        }
        
        let secret = swap.secret.as_ref()
            .ok_or_else(|| anyhow!("No secret"))?;
        
        let htlc_txid = swap.counterparty_htlc_txid.as_ref()
            .ok_or_else(|| anyhow!("No counterparty HTLC"))?;
        
        let counterparty_refund = swap.counterparty_refund_address.as_ref()
            .ok_or_else(|| anyhow!("No counterparty refund address"))?;
        
        // Create HTLC script
        let htlc = HtlcScript::new(
            swap.secret_hash.clone(),
            swap.initiator.clone(),
            counterparty_refund.clone(),
            swap.counterparty_timelock,
            swap.receive_asset,
            swap.receive_amount,
        );
        
        // Claim with secret
        let claim_txid = htlc.claim_htlc(secret, htlc_txid).await?;
        
        swap.status = SwapStatus::ClaimedByInitiator;
        swap.updated_at = Utc::now();
        
        tracing::info!("✅ Initiator claimed: {}", swap_id);
        
        Ok(claim_txid)
    }
    
    /// Claim initiator's HTLC (counterparty claims send_asset using revealed secret)
    pub async fn claim_counterparty(swap_id: &str, secret: &str) -> Result<String> {
        let mut swaps = SWAPS.lock()
            .map_err(|e| anyhow!("Failed to lock swaps: {}", e))?;
        
        let swap = swaps.get_mut(swap_id)
            .ok_or_else(|| anyhow!("Swap not found"))?;
        
        if swap.status != SwapStatus::ClaimedByInitiator {
            return Err(anyhow!("Initiator has not claimed yet"));
        }
        
        // Verify secret
        if !swap.verify_secret(secret) {
            return Err(anyhow!("Invalid secret"));
        }
        
        let htlc_txid = swap.initiator_htlc_txid.as_ref()
            .ok_or_else(|| anyhow!("No initiator HTLC"))?;
        
        let counterparty = swap.counterparty.as_ref()
            .ok_or_else(|| anyhow!("No counterparty"))?;
        
        // Create HTLC script
        let htlc = HtlcScript::new(
            swap.secret_hash.clone(),
            counterparty.clone(),
            swap.initiator_refund_address.clone(),
            swap.initiator_timelock,
            swap.send_asset,
            swap.send_amount,
        );
        
        // Claim with secret
        let claim_txid = htlc.claim_htlc(secret, htlc_txid).await?;
        
        swap.status = SwapStatus::Completed;
        swap.updated_at = Utc::now();
        
        tracing::info!("✅ Swap completed: {}", swap_id);
        
        Ok(claim_txid)
    }
    
    /// Refund swap after timeout
    pub async fn refund_swap(swap_id: &str, current_height: u32) -> Result<(String, String)> {
        let mut swaps = SWAPS.lock()
            .map_err(|e| anyhow!("Failed to lock swaps: {}", e))?;
        
        let swap = swaps.get_mut(swap_id)
            .ok_or_else(|| anyhow!("Swap not found"))?;
        
        if swap.status != SwapStatus::Locked {
            return Err(anyhow!("Swap not in locked state"));
        }
        
        let initiator_htlc_txid = swap.initiator_htlc_txid.as_ref()
            .ok_or_else(|| anyhow!("No initiator HTLC"))?;
        
        let counterparty_htlc_txid = swap.counterparty_htlc_txid.as_ref()
            .ok_or_else(|| anyhow!("No counterparty HTLC"))?;
        
        let counterparty = swap.counterparty.as_ref()
            .ok_or_else(|| anyhow!("No counterparty"))?;
        
        let counterparty_refund = swap.counterparty_refund_address.as_ref()
            .ok_or_else(|| anyhow!("No counterparty refund address"))?;
        
        // Refund initiator's HTLC
        let initiator_htlc = HtlcScript::new(
            swap.secret_hash.clone(),
            counterparty.clone(),
            swap.initiator_refund_address.clone(),
            swap.initiator_timelock,
            swap.send_asset,
            swap.send_amount,
        );
        
        let initiator_refund_txid = initiator_htlc.refund_htlc(initiator_htlc_txid, current_height).await?;
        
        // Refund counterparty's HTLC
        let counterparty_htlc = HtlcScript::new(
            swap.secret_hash.clone(),
            swap.initiator.clone(),
            counterparty_refund.clone(),
            swap.counterparty_timelock,
            swap.receive_asset,
            swap.receive_amount,
        );
        
        let counterparty_refund_txid = counterparty_htlc.refund_htlc(counterparty_htlc_txid, current_height).await?;
        
        swap.status = SwapStatus::Refunded;
        swap.updated_at = Utc::now();
        
        tracing::info!("🔄 Swap refunded: {}", swap_id);
        
        Ok((initiator_refund_txid, counterparty_refund_txid))
    }
    
    /// Get swap by ID
    pub fn get_swap(swap_id: &str) -> Result<AtomicSwap> {
        let swaps = SWAPS.lock()
            .map_err(|e| anyhow!("Failed to lock swaps: {}", e))?;
        
        swaps.get(swap_id)
            .cloned()
            .ok_or_else(|| anyhow!("Swap not found"))
    }
    
    /// Get all swaps for user
    pub fn get_user_swaps(user_id: &str) -> Result<Vec<AtomicSwap>> {
        let swaps = SWAPS.lock()
            .map_err(|e| anyhow!("Failed to lock swaps: {}", e))?;
        
        let result: Vec<_> = swaps.values()
            .filter(|swap| {
                swap.initiator == user_id ||
                swap.counterparty.as_ref().map_or(false, |cp| cp == user_id)
            })
            .cloned()
            .collect();
        
        Ok(result)
    }
    
    /// Get active swap offers (initiated but not accepted)
    pub fn get_active_offers(
        send_asset: Option<QuoteAsset>,
        receive_asset: Option<QuoteAsset>,
    ) -> Result<Vec<AtomicSwap>> {
        let swaps = SWAPS.lock()
            .map_err(|e| anyhow!("Failed to lock swaps: {}", e))?;
        
        let result: Vec<_> = swaps.values()
            .filter(|swap| {
                swap.status == SwapStatus::Initiated &&
                !swap.is_expired() &&
                send_asset.map_or(true, |a| swap.send_asset == a) &&
                receive_asset.map_or(true, |a| swap.receive_asset == a)
            })
            .cloned()
            .collect();
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_creation() {
        let swap = AtomicSwap::new(
            "user1".to_string(),
            QuoteAsset::Btc,
            1_000_000,
            QuoteAsset::Bch,
            10_000_000,
            "refund_addr".to_string(),
            24,
        ).unwrap();
        
        assert_eq!(swap.send_amount, 1_000_000);
        assert_eq!(swap.receive_amount, 10_000_000);
        assert_eq!(swap.status, SwapStatus::Initiated);
        assert!(swap.secret.is_some());
    }
    
    #[test]
    fn test_secret_verification() {
        let swap = AtomicSwap::new(
            "user1".to_string(),
            QuoteAsset::Btc,
            1_000_000,
            QuoteAsset::Doge,
            100_000_000,
            "refund_addr".to_string(),
            24,
        ).unwrap();
        
        let secret = swap.secret.as_ref().unwrap();
        assert!(swap.verify_secret(secret));
        assert!(!swap.verify_secret("wrong_secret"));
    }
    
    #[test]
    fn test_htlc_script() {
        let htlc = HtlcScript::new(
            "hash123".to_string(),
            "recipient_addr".to_string(),
            "refund_addr".to_string(),
            500000,
            QuoteAsset::Btc,
            1_000_000,
        );
        
        let script = htlc.to_script();
        assert!(script.contains("hash123"));
        assert!(script.contains("500000"));
    }
}
