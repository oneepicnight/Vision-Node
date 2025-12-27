// Lightning Network Integration for Vision Node
// Implements BOLT (Basis of Lightning Technology) protocol for instant payments

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc, Duration};

use crate::market::engine::QuoteAsset;

// ============================================================================
// LIGHTNING CHANNEL MANAGEMENT
// ============================================================================

/// Lightning Network channel state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChannelState {
    /// Channel is being opened (funding tx pending)
    Opening,
    /// Channel is active and can process payments
    Active,
    /// Channel is being closed cooperatively
    Closing,
    /// Channel was force-closed (non-cooperative)
    ForceClosed,
    /// Channel is fully closed
    Closed,
}

/// Lightning channel between two nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightningChannel {
    /// Unique channel ID (derived from funding tx)
    pub channel_id: String,
    /// Asset type (BTC, BCH, DOGE)
    pub asset: QuoteAsset,
    /// Local node ID
    pub local_node_id: String,
    /// Remote node ID
    pub remote_node_id: String,
    /// Channel capacity in satoshis
    pub capacity: u64,
    /// Local balance in satoshis
    pub local_balance: u64,
    /// Remote balance in satoshis
    pub remote_balance: u64,
    /// Funding transaction ID
    pub funding_txid: String,
    /// Funding output index
    pub funding_output_index: u32,
    /// Channel state
    pub state: ChannelState,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Channel reserve amount (min balance)
    pub reserve_satoshis: u64,
    /// Commitment fee rate
    pub fee_rate_per_kw: u64,
}

impl LightningChannel {
    pub fn new(
        asset: QuoteAsset,
        local_node_id: String,
        remote_node_id: String,
        capacity: u64,
        funding_txid: String,
        funding_output_index: u32,
    ) -> Self {
        let reserve = capacity / 100; // 1% reserve
        Self {
            channel_id: format!("{}:{}", funding_txid, funding_output_index),
            asset,
            local_node_id,
            remote_node_id,
            capacity,
            local_balance: capacity,
            remote_balance: 0,
            funding_txid,
            funding_output_index,
            state: ChannelState::Opening,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            reserve_satoshis: reserve,
            fee_rate_per_kw: 1000, // 1 sat/vbyte default
        }
    }
    
    /// Check if channel can send amount
    pub fn can_send(&self, amount: u64) -> bool {
        self.state == ChannelState::Active &&
        self.local_balance >= amount + self.reserve_satoshis
    }
    
    /// Check if channel can receive amount
    pub fn can_receive(&self, amount: u64) -> bool {
        self.state == ChannelState::Active &&
        self.remote_balance >= amount + self.reserve_satoshis
    }
}

// ============================================================================
// LIGHTNING INVOICES (BOLT-11)
// ============================================================================

/// Lightning invoice (BOLT-11)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightningInvoice {
    /// Payment hash (32 bytes)
    pub payment_hash: String,
    /// Payment preimage (secret, 32 bytes)
    pub payment_preimage: Option<String>,
    /// Amount in satoshis
    pub amount_sat: u64,
    /// Asset type
    pub asset: QuoteAsset,
    /// Invoice description
    pub description: String,
    /// Payee node ID
    pub payee: String,
    /// Expiry time (seconds from creation)
    pub expiry: u64,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Payment status
    pub status: PaymentStatus,
    /// BOLT-11 encoded string
    pub bolt11: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentStatus {
    Pending,
    InFlight,
    Succeeded,
    Failed,
    Expired,
}

impl LightningInvoice {
    pub fn new(
        amount_sat: u64,
        asset: QuoteAsset,
        description: String,
        payee: String,
        expiry: u64,
    ) -> Self {
        let payment_preimage = Self::generate_preimage();
        let payment_hash = Self::hash_preimage(&payment_preimage);
        let bolt11 = Self::encode_bolt11(&payment_hash, amount_sat, &description, &payee, expiry);
        
        Self {
            payment_hash,
            payment_preimage: Some(payment_preimage),
            amount_sat,
            asset,
            description,
            payee,
            expiry,
            created_at: Utc::now(),
            status: PaymentStatus::Pending,
            bolt11,
        }
    }
    
    fn generate_preimage() -> String {
        use ring::rand::SecureRandom;
        let rng = ring::rand::SystemRandom::new();
        let mut preimage = [0u8; 32];
        rng.fill(&mut preimage).expect("Failed to generate preimage");
        hex::encode(preimage)
    }
    
    fn hash_preimage(preimage: &str) -> String {
        let bytes = hex::decode(preimage).expect("Invalid preimage");
        let hash = blake3::hash(&bytes);
        hex::encode(hash.as_bytes())
    }
    
    fn encode_bolt11(hash: &str, amount: u64, desc: &str, payee: &str, expiry: u64) -> String {
        // Simplified BOLT-11 encoding (production would use proper bech32)
        format!("lnbc{}u1p{}d{}", amount / 1000, &hash[0..20], expiry)
    }
    
    pub fn is_expired(&self) -> bool {
        let expiry_time = self.created_at + Duration::seconds(self.expiry as i64);
        Utc::now() > expiry_time
    }
}

// ============================================================================
// PAYMENT ROUTING (BOLT-07)
// ============================================================================

/// Route hop in Lightning payment path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteHop {
    pub node_id: String,
    pub short_channel_id: String,
    pub fee_msat: u64,
    pub cltv_expiry_delta: u16,
}

/// Complete payment route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRoute {
    pub hops: Vec<RouteHop>,
    pub total_amount_msat: u64,
    pub total_fee_msat: u64,
    pub total_delay: u16,
}

/// Network graph for routing
pub struct NetworkGraph {
    nodes: HashMap<String, NodeInfo>,
    channels: HashMap<String, ChannelInfo>,
}

#[derive(Debug, Clone)]
struct NodeInfo {
    node_id: String,
    channels: Vec<String>,
}

#[derive(Debug, Clone)]
struct ChannelInfo {
    channel_id: String,
    node1: String,
    node2: String,
    capacity: u64,
    fee_base_msat: u64,
    fee_rate_millionths: u64,
}

impl NetworkGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            channels: HashMap::new(),
        }
    }
    
    pub fn add_node(&mut self, node_id: String) {
        self.nodes.entry(node_id.clone()).or_insert(NodeInfo {
            node_id,
            channels: Vec::new(),
        });
    }
    
    pub fn add_channel(&mut self, channel: ChannelInfo) {
        let channel_id = channel.channel_id.clone();
        
        // Add to node1's channels
        if let Some(node) = self.nodes.get_mut(&channel.node1) {
            node.channels.push(channel_id.clone());
        }
        
        // Add to node2's channels
        if let Some(node) = self.nodes.get_mut(&channel.node2) {
            node.channels.push(channel_id.clone());
        }
        
        self.channels.insert(channel_id, channel);
    }
    
    /// Find route using Dijkstra's algorithm
    pub fn find_route(
        &self,
        source: &str,
        destination: &str,
        amount_msat: u64,
    ) -> Result<PaymentRoute> {
        // Simplified routing - production would use A* or better
        if source == destination {
            return Err(anyhow!("Source and destination are the same"));
        }
        
        // For now, return a single-hop route if direct channel exists
        for channel in self.channels.values() {
            if (channel.node1 == source && channel.node2 == destination) ||
               (channel.node2 == source && channel.node1 == destination) {
                let fee = channel.fee_base_msat + 
                          (amount_msat * channel.fee_rate_millionths) / 1_000_000;
                
                return Ok(PaymentRoute {
                    hops: vec![RouteHop {
                        node_id: destination.to_string(),
                        short_channel_id: channel.channel_id.clone(),
                        fee_msat: fee,
                        cltv_expiry_delta: 40, // Standard 40 blocks
                    }],
                    total_amount_msat: amount_msat + fee,
                    total_fee_msat: fee,
                    total_delay: 40,
                });
            }
        }
        
        Err(anyhow!("No route found"))
    }
}

// ============================================================================
// HTLC (Hash Time Locked Contracts)
// ============================================================================

/// HTLC for conditional payments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Htlc {
    /// HTLC ID
    pub id: String,
    /// Payment hash
    pub payment_hash: String,
    /// Amount in millisatoshis
    pub amount_msat: u64,
    /// Expiry block height
    pub cltv_expiry: u32,
    /// Is this an outgoing HTLC?
    pub outgoing: bool,
    /// Channel ID
    pub channel_id: String,
    /// Status
    pub status: HtlcStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HtlcStatus {
    Pending,
    Fulfilled,
    Failed,
    TimedOut,
}

// ============================================================================
// LIGHTNING MANAGER
// ============================================================================

/// Global state
pub static CHANNELS: Lazy<Arc<Mutex<HashMap<String, LightningChannel>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub static INVOICES: Lazy<Arc<Mutex<HashMap<String, LightningInvoice>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub static NETWORK_GRAPH: Lazy<Arc<Mutex<NetworkGraph>>> = 
    Lazy::new(|| Arc::new(Mutex::new(NetworkGraph::new())));

pub struct LightningManager;

impl LightningManager {
    /// Open a new Lightning channel
    pub async fn open_channel(
        user_id: &str,
        asset: QuoteAsset,
        remote_node_id: String,
        capacity: u64,
    ) -> Result<LightningChannel> {
        if asset == QuoteAsset::Land {
            return Err(anyhow!("Lightning channels not supported for LAND"));
        }
        
        // In production, this would:
        // 1. Create funding transaction
        // 2. Exchange signatures with peer
        // 3. Broadcast funding tx
        // 4. Wait for confirmations
        
        let funding_txid = format!("funding_{}_{}", user_id, Utc::now().timestamp());
        let channel = LightningChannel::new(
            asset,
            user_id.to_string(),
            remote_node_id,
            capacity,
            funding_txid,
            0,
        );
        
        let channel_id = channel.channel_id.clone();
        
        let mut channels = CHANNELS.lock()
            .map_err(|e| anyhow!("Failed to lock channels: {}", e))?;
        
        channels.insert(channel_id.clone(), channel.clone());
        
        tracing::info!("⚡ Opened Lightning channel: {}", channel_id);
        
        Ok(channel)
    }
    
    /// Activate channel after funding tx confirms
    pub fn activate_channel(channel_id: &str) -> Result<()> {
        let mut channels = CHANNELS.lock()
            .map_err(|e| anyhow!("Failed to lock channels: {}", e))?;
        
        let channel = channels.get_mut(channel_id)
            .ok_or_else(|| anyhow!("Channel not found"))?;
        
        if channel.state != ChannelState::Opening {
            return Err(anyhow!("Channel not in opening state"));
        }
        
        channel.state = ChannelState::Active;
        channel.updated_at = Utc::now();
        
        tracing::info!("⚡ Activated Lightning channel: {}", channel_id);
        
        Ok(())
    }
    
    /// Create a Lightning invoice
    pub fn create_invoice(
        user_id: &str,
        amount_sat: u64,
        asset: QuoteAsset,
        description: String,
        expiry_seconds: u64,
    ) -> Result<LightningInvoice> {
        let invoice = LightningInvoice::new(
            amount_sat,
            asset,
            description,
            user_id.to_string(),
            expiry_seconds,
        );
        
        let payment_hash = invoice.payment_hash.clone();
        
        let mut invoices = INVOICES.lock()
            .map_err(|e| anyhow!("Failed to lock invoices: {}", e))?;
        
        invoices.insert(payment_hash.clone(), invoice.clone());
        
        tracing::info!("⚡ Created invoice: {} for {} sat", payment_hash, amount_sat);
        
        Ok(invoice)
    }
    
    /// Pay a Lightning invoice
    pub async fn pay_invoice(
        user_id: &str,
        bolt11: &str,
    ) -> Result<String> {
        // Decode BOLT-11 invoice
        let invoice = Self::decode_bolt11(bolt11)?;
        
        // Find route to destination
        let graph = NETWORK_GRAPH.lock()
            .map_err(|e| anyhow!("Failed to lock graph: {}", e))?;
        
        let route = graph.find_route(
            user_id,
            &invoice.payee,
            invoice.amount_sat * 1000, // Convert to msat
        )?;
        
        drop(graph);
        
        // Send payment through route
        let payment_hash = invoice.payment_hash.clone();
        Self::send_payment(user_id, route, &payment_hash).await?;
        
        tracing::info!("⚡ Payment sent: {}", payment_hash);
        
        Ok(payment_hash)
    }
    
    async fn send_payment(
        _user_id: &str,
        route: PaymentRoute,
        payment_hash: &str,
    ) -> Result<()> {
        // In production, this would:
        // 1. Create onion-routed packet
        // 2. Send update_add_htlc to first hop
        // 3. Wait for settlement or failure
        
        tracing::info!(
            "⚡ Sending payment via route: {} hops, {} msat total",
            route.hops.len(),
            route.total_amount_msat
        );
        
        // Simulate successful payment
        let mut invoices = INVOICES.lock()
            .map_err(|e| anyhow!("Failed to lock invoices: {}", e))?;
        
        if let Some(invoice) = invoices.get_mut(payment_hash) {
            invoice.status = PaymentStatus::Succeeded;
        }
        
        Ok(())
    }
    
    fn decode_bolt11(bolt11: &str) -> Result<LightningInvoice> {
        // Simplified decoding (production would use proper bech32 parser)
        if !bolt11.starts_with("lnbc") {
            return Err(anyhow!("Invalid BOLT-11 invoice"));
        }
        
        // For now, return mock invoice
        Ok(LightningInvoice {
            payment_hash: "mock_hash".to_string(),
            payment_preimage: None,
            amount_sat: 10000,
            asset: QuoteAsset::Btc,
            description: "Mock invoice".to_string(),
            payee: "mock_node".to_string(),
            expiry: 3600,
            created_at: Utc::now(),
            status: PaymentStatus::Pending,
            bolt11: bolt11.to_string(),
        })
    }
    
    /// Close a Lightning channel cooperatively
    pub async fn close_channel(channel_id: &str) -> Result<String> {
        let mut channels = CHANNELS.lock()
            .map_err(|e| anyhow!("Failed to lock channels: {}", e))?;
        
        let channel = channels.get_mut(channel_id)
            .ok_or_else(|| anyhow!("Channel not found"))?;
        
        if channel.state != ChannelState::Active {
            return Err(anyhow!("Channel not active"));
        }
        
        channel.state = ChannelState::Closing;
        channel.updated_at = Utc::now();
        
        // In production, this would create and broadcast closing tx
        let closing_txid = format!("closing_{}_{}", channel_id, Utc::now().timestamp());
        
        tracing::info!("⚡ Closing Lightning channel: {}", channel_id);
        
        Ok(closing_txid)
    }
    
    /// Get all channels for user
    pub fn get_channels(user_id: &str, asset: Option<QuoteAsset>) -> Result<Vec<LightningChannel>> {
        let channels = CHANNELS.lock()
            .map_err(|e| anyhow!("Failed to lock channels: {}", e))?;
        
        let result: Vec<_> = channels.values()
            .filter(|ch| {
                ch.local_node_id == user_id &&
                asset.map_or(true, |a| ch.asset == a)
            })
            .cloned()
            .collect();
        
        Ok(result)
    }
    
    /// Get channel statistics
    pub fn get_stats(user_id: &str) -> Result<LightningStats> {
        let channels = CHANNELS.lock()
            .map_err(|e| anyhow!("Failed to lock channels: {}", e))?;
        
        let user_channels: Vec<_> = channels.values()
            .filter(|ch| ch.local_node_id == user_id)
            .collect();
        
        let active_channels = user_channels.iter()
            .filter(|ch| ch.state == ChannelState::Active)
            .count();
        
        let total_capacity: u64 = user_channels.iter()
            .filter(|ch| ch.state == ChannelState::Active)
            .map(|ch| ch.capacity)
            .sum();
        
        let local_balance: u64 = user_channels.iter()
            .filter(|ch| ch.state == ChannelState::Active)
            .map(|ch| ch.local_balance)
            .sum();
        
        let remote_balance: u64 = user_channels.iter()
            .filter(|ch| ch.state == ChannelState::Active)
            .map(|ch| ch.remote_balance)
            .sum();
        
        Ok(LightningStats {
            total_channels: user_channels.len(),
            active_channels,
            total_capacity,
            local_balance,
            remote_balance,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LightningStats {
    pub total_channels: usize,
    pub active_channels: usize,
    pub total_capacity: u64,
    pub local_balance: u64,
    pub remote_balance: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_channel_creation() {
        let channel = LightningManager::open_channel(
            "user1",
            QuoteAsset::Btc,
            "node2".to_string(),
            1_000_000,
        ).await.unwrap();
        
        assert_eq!(channel.capacity, 1_000_000);
        assert_eq!(channel.local_balance, 1_000_000);
        assert_eq!(channel.state, ChannelState::Opening);
    }
    
    #[test]
    fn test_invoice_creation() {
        let invoice = LightningManager::create_invoice(
            "user1",
            50_000,
            QuoteAsset::Btc,
            "Test payment".to_string(),
            3600,
        ).unwrap();
        
        assert_eq!(invoice.amount_sat, 50_000);
        assert!(!invoice.bolt11.is_empty());
        assert_eq!(invoice.status, PaymentStatus::Pending);
    }
    
    #[test]
    fn test_invoice_expiry() {
        let mut invoice = LightningInvoice::new(
            1000,
            QuoteAsset::Btc,
            "Test".to_string(),
            "node1".to_string(),
            0, // Expires immediately
        );
        
        // Wait a bit
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        assert!(invoice.is_expired());
    }
}
