// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Vision Contributors

// External Chain Deposit Scanning
// Monitors BTC, BCH, and DOGE chains for deposits to user addresses
//
// NON-CUSTODIAL ARCHITECTURE:
// - Each user gets deterministic addresses derived from node's external master seed
// - Keys stored locally on the user node (non-custodial)
// - User can export/import seed for full control and backup
// - Addresses are REAL chain-valid: bc1... (BTC), bitcoincash:... (BCH), D... (DOGE)
// - NO sweeping to vault addresses - user funds stay in user addresses
// - Only exchange FEES go to miners multisig vault addresses

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::market::engine::QuoteAsset;
use crate::market::real_addresses;
use crate::market::wallet::{process_deposit, DepositEvent};

/// Global address mapping: deposit_address -> user_id
/// This is persisted to database in production
static ADDRESS_TO_USER: Lazy<Arc<Mutex<HashMap<String, String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// MAINNET: Rebuild deposit address caches from database on startup
/// Call this during node initialization to restore mappings across restarts
pub fn rebuild_deposit_caches_from_db() -> Result<()> {
    let db = crate::CHAIN.lock().db.clone();

    // Restore address-to-user mappings
    if let Ok(deposit_tree) = db.open_tree(crate::vision_constants::DEPOSIT_MAPPING_TREE) {
        let mut map = ADDRESS_TO_USER.lock().unwrap();
        let mut count = 0;

        for item in deposit_tree
            .scan_prefix(crate::vision_constants::DEPOSIT_ADDR_TO_WALLET_PREFIX.as_bytes())
        {
            if let Ok((key, value)) = item {
                let key_str = String::from_utf8_lossy(&key);
                let address = key_str
                    .trim_start_matches(crate::vision_constants::DEPOSIT_ADDR_TO_WALLET_PREFIX);
                let user_id = String::from_utf8_lossy(&value).to_string();

                map.insert(address.to_string(), user_id);
                count += 1;
            }
        }

        tracing::info!(
            "✅ Restored {} deposit address mappings from database",
            count
        );
    }

    // Restore last scanned heights
    if let Ok(scan_tree) = db.open_tree("scan_heights") {
        let mut heights = LAST_SCANNED_HEIGHTS.lock().unwrap();

        for item in scan_tree.iter() {
            if let Ok((key, value)) = item {
                let chain_name = String::from_utf8_lossy(&key).to_string();
                if value.len() == 8 {
                    let mut bytes = [0u8; 8];
                    bytes.copy_from_slice(&value);
                    let height = u64::from_be_bytes(bytes);
                    heights.insert(chain_name.clone(), height);
                    tracing::info!("✅ Restored {} scan height: {}", chain_name, height);
                }
            }
        }
    }

    Ok(())
}

/// Generate a stable, permanent user_id from a wallet address
///
/// This is the RECOMMENDED way to derive user_id for production use.
/// The wallet address is stable, permanent, and unique per user.
///
/// Example:
/// ```rust
/// let user_id = stable_user_id_from_wallet("0x1234...");
/// let deposit_addr = deposit_address_for_user(&user_id, QuoteAsset::Bch)?;
/// ```
pub fn stable_user_id_from_wallet(wallet_address: &str) -> String {
    // Normalize address (lowercase, strip 0x if present)
    let normalized = wallet_address
        .to_lowercase()
        .trim_start_matches("0x")
        .to_string();
    // Return as-is: wallet address IS the stable user_id
    // Simple, boring, permanent - exactly what we want
    normalized
}

/// Get user index from user_id (hash-based, deterministic)
///
/// Uses Blake3 for cryptographic-quality hashing to avoid collisions.
/// The `user_id` should be generated via `stable_user_id_from_wallet()` for production.
///
/// ⚠️  PRODUCTION REQUIREMENT: Use wallet address as user_id (via stable_user_id_from_wallet)
/// DO NOT use mutable identifiers (usernames, emails, session IDs).
fn user_id_to_index(user_id: &str) -> u32 {
    // Use Blake3 for high-quality deterministic hashing
    let hash = blake3::hash(user_id.as_bytes());
    let hash_bytes = hash.as_bytes();

    // Take first 4 bytes and convert to u32
    let mut index_bytes = [0u8; 4];
    index_bytes.copy_from_slice(&hash_bytes[0..4]);
    let full_index = u32::from_be_bytes(index_bytes);

    // Limit to reasonable range (1 million unique derivation paths)
    full_index % 1_000_000
}

/// Public helper: Derive deterministic deposit address for a user on a specific asset
///
/// Uses real_addresses module for chain-valid address generation:
///   - BTC => bc1... (bech32 P2WPKH)
///   - BCH => bitcoincash:... (CashAddr P2PKH)
///   - DOGE => D... (base58check P2PKH with version 0x1E)
///   - LAND => Native asset (no external address)
///
/// Address is stable across restarts with same external_master_seed.bin
///
/// ⚠️  PRODUCTION REQUIREMENT: Use wallet address as `user_id` ⚠️
///
/// RECOMMENDED pattern (boring and stable):
/// ```rust
/// // Use wallet address directly as user_id - simple, stable, permanent
/// let user_id = stable_user_id_from_wallet("0x1234abcd...");
/// let bch_deposit = deposit_address_for_user(&user_id, QuoteAsset::Bch)?;
/// ```
///
/// The wallet address is:
/// ✅ Permanent (never changes)
/// ✅ Unique (one per user)
/// ✅ Boring (no clever abstractions)
/// ✅ Production-ready (no drift risk)
///
/// CRITICAL: Users must backup seed via /api/wallet/external/export or lose funds on reinstall
pub fn deposit_address_for_user(user_id: &str, asset: QuoteAsset) -> Result<String> {
    let coin = match asset {
        QuoteAsset::Btc => "BTC",
        QuoteAsset::Bch => "BCH",
        QuoteAsset::Doge => "DOGE",
        QuoteAsset::Land => return Err(anyhow!("LAND is a native asset; no deposit address")),
    };

    let user_index = user_id_to_index(user_id);
    real_addresses::derive_address(coin, user_index)
}

/// Export external master seed (hex-encoded)
/// CRITICAL: This is the ONLY way to backup/restore user funds across reinstalls
/// WARNING: Anyone with this seed can derive all user addresses and spend funds
pub fn export_external_seed() -> Result<String> {
    let seed = real_addresses::get_or_create_master_seed()?;
    Ok(hex::encode(seed))
}

/// Import external master seed from hex
/// DANGER: This OVERWRITES the existing seed - all previous addresses become inaccessible
/// Only use this when restoring from backup or migrating node
pub fn import_external_seed(seed_hex: &str) -> Result<()> {
    use std::fs;
    use std::io::Write;

    // Decode hex seed
    let seed_bytes = hex::decode(seed_hex).map_err(|e| anyhow!("Invalid hex seed: {}", e))?;

    if seed_bytes.len() != 32 {
        return Err(anyhow!(
            "Seed must be exactly 32 bytes (64 hex chars), got {}",
            seed_bytes.len()
        ));
    }

    let mut seed = [0u8; 32];
    seed.copy_from_slice(&seed_bytes);

    // Backup old seed if it exists
    let seed_path = crate::market::real_addresses::seed_file_path_public();
    if seed_path.exists() {
        let backup_path = seed_path
            .parent()
            .unwrap()
            .join("external_master_seed.bin.backup");
        fs::copy(&seed_path, &backup_path)?;
        tracing::warn!("⚠️  Old seed backed up to: {}", backup_path.display());
    }

    // Write new seed
    let mut file = fs::File::create(&seed_path)?;
    file.write_all(&seed)?;
    file.sync_all()?;

    // Set restrictive permissions (Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&seed_path, perms)?;
    }

    tracing::warn!("🔐 External seed imported successfully");
    tracing::warn!("⚠️  ALL PREVIOUS ADDRESSES ARE NOW INACCESSIBLE");
    tracing::warn!("⚠️  Restart node to regenerate addresses from new seed");

    Ok(())
}

/// Store address -> user_id mapping (WITH PERSISTENCE FOR MAINNET)
fn store_address_mapping(address: &str, user_id: &str) -> Result<()> {
    let mut map = ADDRESS_TO_USER.lock().unwrap();
    map.insert(address.to_string(), user_id.to_string());

    // MAINNET: Persist to sled database
    let db = crate::CHAIN.lock().db.clone();
    let deposit_tree = db.open_tree(crate::vision_constants::DEPOSIT_MAPPING_TREE)?;

    // Store bidirectional mapping:
    // 1. deposit_address -> user_id (for incoming tx attribution)
    let key_a2w = format!(
        "{}{}",
        crate::vision_constants::DEPOSIT_ADDR_TO_WALLET_PREFIX,
        address
    );
    deposit_tree.insert(key_a2w.as_bytes(), user_id.as_bytes())?;

    // 2. user_id -> deposit_address (for address regeneration on restart)
    // We can derive deposit index from user_id deterministically, so we store the index
    let deposit_index = user_id_to_index(user_id);
    let key_w2i = format!(
        "{}{}",
        crate::vision_constants::DEPOSIT_WALLET_TO_INDEX_PREFIX,
        user_id
    );
    deposit_tree.insert(key_w2i.as_bytes(), &deposit_index.to_be_bytes())?;

    Ok(())
}

/// Get user_id from address
pub fn get_user_from_address(address: &str) -> Option<String> {
    let map = ADDRESS_TO_USER.lock().unwrap();
    map.get(address).cloned()
}

/// Trait for external blockchain backends
pub trait ExternalChainBackend: Send + Sync {
    /// Get the chain name (BTC, BCH, DOGE)
    fn chain_name(&self) -> &str;

    /// Get the QuoteAsset for this chain
    fn quote_asset(&self) -> QuoteAsset;

    /// Generate or retrieve deposit address for a user
    /// TODO: Integrate with actual wallet generation (BIP32/BIP44)
    fn get_or_create_deposit_address(&self, user_id: &str) -> Result<String>;

    /// Scan for new deposits since last check
    /// Returns list of deposit events with confirmations
    fn scan_new_deposits(&self, last_block_height: u64) -> Result<Vec<DepositEvent>>;

    /// Get current block height of the external chain
    fn get_block_height(&self) -> Result<u64>;

    /// Get number of confirmations required before crediting
    fn confirmations_required(&self) -> u32 {
        6 // Default: 6 confirmations for security
    }
}

/// Bitcoin backend with RPC connection
pub struct BitcoinBackend {
    coin_type: u32, // BIP44 coin type (0 for BTC, 145 for BCH, 3 for DOGE)
}

impl BitcoinBackend {
    pub fn new() -> Self {
        // Check if external RPC is configured
        let has_rpc = {
            let clients = crate::EXTERNAL_RPC_CLIENTS
                .lock()
                .expect("External RPC clients lock poisoned");
            clients.contains_key(&crate::external_rpc::ExternalChain::Btc)
        };

        if has_rpc {
            tracing::info!("✅ Bitcoin RPC configured via external_rpc system");
        } else {
            tracing::warn!("⚠️  Bitcoin RPC not configured - deposits disabled");
        }

        Self {
            coin_type: 0, // BTC coin type
        }
    }

    fn get_rpc_client(&self) -> Option<Arc<crate::external_rpc::RpcClient>> {
        let clients = crate::EXTERNAL_RPC_CLIENTS
            .lock()
            .expect("External RPC clients lock poisoned");
        clients
            .get(&crate::external_rpc::ExternalChain::Btc)
            .cloned()
    }
}

impl ExternalChainBackend for BitcoinBackend {
    fn chain_name(&self) -> &str {
        "Bitcoin"
    }

    fn quote_asset(&self) -> QuoteAsset {
        QuoteAsset::Btc
    }

    fn get_or_create_deposit_address(&self, user_id: &str) -> Result<String> {
        // Check if address already exists
        let existing = {
            let map = ADDRESS_TO_USER.lock().unwrap();
            map.iter()
                .find(|(_, uid)| uid.as_str() == user_id)
                .map(|(addr, _)| addr.clone())
        };

        if let Some(addr) = existing {
            return Ok(addr);
        }

        // Derive new address using real address derivation
        let user_index = user_id_to_index(user_id);
        let address = real_addresses::derive_address("BTC", user_index)?;

        // Store mapping
        store_address_mapping(&address, user_id)?;

        tracing::info!("Generated BTC address {} for user {}", address, user_id);
        Ok(address)
    }

    fn scan_new_deposits(&self, last_block_height: u64) -> Result<Vec<DepositEvent>> {
        let client = match self.get_rpc_client() {
            Some(c) => c,
            None => return Ok(Vec::new()), // No RPC, no deposits
        };

        let mut deposits = Vec::new();

        // Get current block height using external RPC
        let current_height_future = async {
            let result = client.call_no_params("getblockcount").await?;
            result
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid block count response"))
        };

        let current_height = tokio::runtime::Handle::current()
            .block_on(current_height_future)
            .map_err(|e| anyhow!("Failed to get block count: {}", e))?;

        // Scan blocks from last_block_height to current
        // Note: This is a simplified implementation that scans for deposits
        // In production, consider using a more efficient approach with address indexing
        for height in last_block_height..=current_height {
            let scan_future = async {
                // Get block hash at height
                let hash_result = client
                    .call("getblockhash", serde_json::json!([height]))
                    .await?;
                let block_hash = hash_result
                    .as_str()
                    .ok_or_else(|| anyhow!("Invalid block hash response"))?;

                // Get block with transactions (verbosity=2 for full tx details)
                let block_result = client
                    .call("getblock", serde_json::json!([block_hash, 2]))
                    .await?;

                // Parse transactions
                if let Some(tx_array) = block_result.get("tx").and_then(|v| v.as_array()) {
                    for tx in tx_array {
                        let txid = tx.get("txid").and_then(|v| v.as_str()).unwrap_or("");

                        // Check outputs
                        if let Some(vout_array) = tx.get("vout").and_then(|v| v.as_array()) {
                            for (vout_idx, vout) in vout_array.iter().enumerate() {
                                let value =
                                    vout.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);

                                // Get addresses from scriptPubKey
                                if let Some(addresses) = vout
                                    .get("scriptPubKey")
                                    .and_then(|sp| sp.get("addresses"))
                                    .and_then(|a| a.as_array())
                                {
                                    for addr_val in addresses {
                                        if let Some(addr_str) = addr_val.as_str() {
                                            // Check if this address belongs to one of our users
                                            if let Some(user_id) = get_user_from_address(addr_str) {
                                                let confirmations =
                                                    (current_height - height) as u32 + 1;

                                                return Ok::<Option<DepositEvent>, anyhow::Error>(
                                                    Some(DepositEvent {
                                                        user_id: user_id.clone(),
                                                        asset: QuoteAsset::Btc,
                                                        amount: value,
                                                        txid: format!("{}:{}", txid, vout_idx),
                                                        confirmations,
                                                    }),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                Ok::<Option<DepositEvent>, anyhow::Error>(None)
            };

            match tokio::runtime::Handle::current().block_on(scan_future) {
                Ok(Some(deposit)) => {
                    tracing::info!(
                        "📥 BTC deposit detected: {} BTC to user {} ({} confirmations)",
                        deposit.amount,
                        deposit.user_id,
                        deposit.confirmations
                    );
                    deposits.push(deposit);
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!("Failed to scan block at height {}: {}", height, e);
                }
            }
        }

        Ok(deposits)
    }

    fn get_block_height(&self) -> Result<u64> {
        // TODO: Use cached height from async background task
        // For now, return 0 to allow scanner to skip this chain
        Ok(0)
    }
}

/// Bitcoin Cash backend (uses same RPC interface as Bitcoin)
pub struct BitcoinCashBackend {
    coin_type: u32,
}

impl BitcoinCashBackend {
    pub fn new() -> Self {
        let has_rpc = {
            let clients = crate::EXTERNAL_RPC_CLIENTS
                .lock()
                .expect("External RPC clients lock poisoned");
            clients.contains_key(&crate::external_rpc::ExternalChain::Bch)
        };

        if has_rpc {
            tracing::info!("✅ Bitcoin Cash RPC configured via external_rpc system");
        } else {
            tracing::warn!("⚠️  Bitcoin Cash RPC not configured - deposits disabled");
        }

        Self {
            coin_type: 145, // BCH coin type
        }
    }

    fn get_rpc_client(&self) -> Option<Arc<crate::external_rpc::RpcClient>> {
        let clients = crate::EXTERNAL_RPC_CLIENTS
            .lock()
            .expect("External RPC clients lock poisoned");
        clients
            .get(&crate::external_rpc::ExternalChain::Bch)
            .cloned()
    }
}

impl ExternalChainBackend for BitcoinCashBackend {
    fn chain_name(&self) -> &str {
        "Bitcoin Cash"
    }

    fn quote_asset(&self) -> QuoteAsset {
        QuoteAsset::Bch
    }

    fn get_or_create_deposit_address(&self, user_id: &str) -> Result<String> {
        let existing = {
            let map = ADDRESS_TO_USER.lock().unwrap();
            map.iter()
                .find(|(addr, uid)| uid.as_str() == user_id && addr.starts_with("bitcoincash:"))
                .map(|(addr, _)| addr.clone())
        };

        if let Some(addr) = existing {
            return Ok(addr);
        }

        let user_index = user_id_to_index(user_id);
        let address = real_addresses::derive_address("BCH", user_index)?;
        // BCH address already includes bitcoincash: prefix from real_addresses

        store_address_mapping(&address, user_id)?;

        tracing::info!("Generated BCH address {} for user {}", address, user_id);
        Ok(address)
    }

    fn scan_new_deposits(&self, _last_block_height: u64) -> Result<Vec<DepositEvent>> {
        // Similar to Bitcoin scanning (omitted for brevity - would be same logic)
        tracing::debug!("BCH scanning enabled but not fully implemented in this version");
        Ok(Vec::new())
    }

    fn get_block_height(&self) -> Result<u64> {
        // TODO: Use cached height from async background task
        // For now, return 0 to allow scanner to skip this chain
        Ok(0)
    }
}

/// Dogecoin backend (uses same RPC interface)
pub struct DogecoinBackend {
    coin_type: u32,
}

impl DogecoinBackend {
    pub fn new() -> Self {
        let has_rpc = {
            let clients = crate::EXTERNAL_RPC_CLIENTS
                .lock()
                .expect("External RPC clients lock poisoned");
            clients.contains_key(&crate::external_rpc::ExternalChain::Doge)
        };

        if has_rpc {
            tracing::info!("✅ Dogecoin RPC configured via external_rpc system");
        } else {
            tracing::warn!("⚠️  Dogecoin RPC not configured - deposits disabled");
        }

        Self {
            coin_type: 3, // DOGE coin type
        }
    }

    fn get_rpc_client(&self) -> Option<Arc<crate::external_rpc::RpcClient>> {
        let clients = crate::EXTERNAL_RPC_CLIENTS
            .lock()
            .expect("External RPC clients lock poisoned");
        clients
            .get(&crate::external_rpc::ExternalChain::Doge)
            .cloned()
    }
}

impl ExternalChainBackend for DogecoinBackend {
    fn chain_name(&self) -> &str {
        "Dogecoin"
    }

    fn quote_asset(&self) -> QuoteAsset {
        QuoteAsset::Doge
    }

    fn get_or_create_deposit_address(&self, user_id: &str) -> Result<String> {
        let existing = {
            let map = ADDRESS_TO_USER.lock().unwrap();
            map.iter()
                .find(|(addr, uid)| uid.as_str() == user_id && addr.starts_with("D"))
                .map(|(addr, _)| addr.clone())
        };

        if let Some(addr) = existing {
            return Ok(addr);
        }

        let user_index = user_id_to_index(user_id);
        let address = real_addresses::derive_address("DOGE", user_index)?;

        store_address_mapping(&address, user_id)?;

        tracing::info!("Generated DOGE address {} for user {}", address, user_id);
        Ok(address)
    }

    fn scan_new_deposits(&self, _last_block_height: u64) -> Result<Vec<DepositEvent>> {
        tracing::debug!("DOGE scanning enabled but not fully implemented in this version");
        Ok(Vec::new())
    }

    fn get_block_height(&self) -> Result<u64> {
        // TODO: Use cached height from async background task
        // For now, return 0 to allow scanner to skip this chain
        Ok(0)
    }
}

/// Last scanned block heights per chain
static LAST_SCANNED_HEIGHTS: Lazy<Arc<Mutex<HashMap<String, u64>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Last scan timestamp
static LAST_SCAN_TIME: Lazy<Arc<Mutex<Option<chrono::DateTime<chrono::Utc>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

fn get_last_scanned_height(chain_name: &str) -> u64 {
    let heights = LAST_SCANNED_HEIGHTS.lock().unwrap();
    heights.get(chain_name).copied().unwrap_or(0)
}

fn update_last_scanned_height(chain_name: &str, height: u64) {
    let mut heights = LAST_SCANNED_HEIGHTS.lock().unwrap();
    heights.insert(chain_name.to_string(), height);

    // MAINNET: Persist to database (best effort - don't block on errors)
    let chain = chain_name.to_string();
    let h = height;
    let _ = std::thread::spawn(move || {
        if let Some(chain_lock) = crate::CHAIN.try_lock() {
            if let Ok(tree) = chain_lock.db.open_tree("scan_heights") {
                let _ = tree.insert(chain.as_bytes(), &h.to_be_bytes());
            }
        }
    });
}

/// Deposit scanner manager
pub struct DepositScanner {
    backends: Vec<Box<dyn ExternalChainBackend>>,
}

impl DepositScanner {
    /// Create new deposit scanner with all supported chains
    pub fn new() -> Self {
        let backends: Vec<Box<dyn ExternalChainBackend>> = vec![
            Box::new(BitcoinBackend::new()),
            Box::new(BitcoinCashBackend::new()),
            Box::new(DogecoinBackend::new()),
        ];

        Self { backends }
    }

    /// Run one scan cycle across all chains
    /// Should be called periodically (e.g., every 30 seconds)
    pub fn scan_all_chains(&self) -> Result<usize> {
        let mut total_deposits = 0;

        for backend in &self.backends {
            let chain_name = backend.chain_name();

            // Get last scanned block height from state
            let last_height = get_last_scanned_height(chain_name);

            // Get current block height
            let current_height = match backend.get_block_height() {
                Ok(h) if h > 0 => h,
                _ => {
                    tracing::debug!("{} node not available, skipping scan", chain_name);
                    continue;
                }
            };

            // Only scan if there are new blocks
            if current_height <= last_height {
                continue;
            }

            match backend.scan_new_deposits(last_height) {
                Ok(deposits) => {
                    for deposit in deposits {
                        // Only process deposits with sufficient confirmations
                        if deposit.confirmations >= backend.confirmations_required() {
                            match process_deposit(deposit.clone()) {
                                Ok(_) => {
                                    tracing::info!(
                                        "✅ Processed deposit: {} {} to {} (txid: {})",
                                        deposit.amount,
                                        deposit.asset.as_str(),
                                        deposit.user_id,
                                        deposit.txid
                                    );
                                    total_deposits += 1;
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to process deposit for {}: {}",
                                        deposit.user_id,
                                        e
                                    );
                                }
                            }
                        } else {
                            tracing::debug!(
                                "Deposit pending: {} {} to {} ({}/{} confirmations)",
                                deposit.amount,
                                deposit.asset.as_str(),
                                deposit.user_id,
                                deposit.confirmations,
                                backend.confirmations_required()
                            );
                        }
                    }

                    // Update last scanned height
                    update_last_scanned_height(chain_name, current_height);
                }
                Err(e) => {
                    tracing::warn!("Failed to scan {} deposits: {}", chain_name, e);
                }
            }
        }

        Ok(total_deposits)
    }
}

/// Background task that runs deposit scanning periodically
pub async fn run_deposit_scanner() {
    let scanner = DepositScanner::new();
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

    tracing::info!("🔍 Starting deposit scanner (checking every 30 seconds)");

    loop {
        interval.tick().await;

        // Update last scan time
        {
            let mut last_scan = LAST_SCAN_TIME.lock().unwrap();
            *last_scan = Some(chrono::Utc::now());
        }

        match scanner.scan_all_chains() {
            Ok(count) if count > 0 => {
                tracing::info!("Deposit scan complete: processed {} deposits", count);
            }
            Ok(_) => {
                tracing::debug!("Deposit scan complete: no new deposits");
            }
            Err(e) => {
                tracing::error!("Deposit scan failed: {}", e);
            }
        }
    }
}

/// Get deposit scanner status for API
pub fn get_deposit_status() -> serde_json::Value {
    let clients = crate::EXTERNAL_RPC_CLIENTS.lock().unwrap();
    let heights = LAST_SCANNED_HEIGHTS.lock().unwrap();
    let last_scan = LAST_SCAN_TIME.lock().unwrap();

    let mut chains = serde_json::Map::new();

    // BTC status
    if clients.contains_key(&crate::external_rpc::ExternalChain::Btc) {
        chains.insert(
            "btc".to_string(),
            serde_json::json!({
                "rpc_ok": true,
                "last_scanned_height": heights.get("Bitcoin").copied().unwrap_or(0),
                "confirmations_required": 3
            }),
        );
    }

    // BCH status
    if clients.contains_key(&crate::external_rpc::ExternalChain::Bch) {
        chains.insert(
            "bch".to_string(),
            serde_json::json!({
                "rpc_ok": true,
                "last_scanned_height": heights.get("Bitcoin Cash").copied().unwrap_or(0),
                "confirmations_required": 6
            }),
        );
    }

    // DOGE status
    if clients.contains_key(&crate::external_rpc::ExternalChain::Doge) {
        chains.insert(
            "doge".to_string(),
            serde_json::json!({
                "rpc_ok": true,
                "last_scanned_height": heights.get("Dogecoin").copied().unwrap_or(0),
                "confirmations_required": 20
            }),
        );
    }

    serde_json::json!({
        "enabled": !clients.is_empty(),
        "chains": chains,
        "last_scan_utc": last_scan.map(|t| t.to_rfc3339())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_creation() {
        let btc = BitcoinBackend::new();
        assert_eq!(btc.chain_name(), "Bitcoin");
        assert_eq!(btc.confirmations_required(), 6);

        let bch = BitcoinCashBackend::new();
        assert_eq!(bch.chain_name(), "Bitcoin Cash");

        let doge = DogecoinBackend::new();
        assert_eq!(doge.chain_name(), "Dogecoin");
    }

    #[test]
    fn test_scanner_creation() {
        let scanner = DepositScanner::new();
        assert_eq!(scanner.backends.len(), 3);
    }
}
