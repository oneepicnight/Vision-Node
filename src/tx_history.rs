//! Transaction History & Tracking Module
//!
//! Persistent storage and retrieval of transaction history for wallet operations.
//! Tracks sends, receives, confirmations, and transaction status.
#![allow(dead_code)]

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::market::engine::QuoteAsset;

/// Transaction type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TxType {
    /// Outgoing transaction (send)
    Send,
    /// Incoming transaction (receive/deposit)
    Receive,
    /// Internal transfer
    Internal,
}

/// Transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TxStatus {
    /// Transaction pending in mempool
    Pending,
    /// Transaction confirmed on blockchain
    Confirmed,
    /// Transaction failed
    Failed,
    /// Transaction replaced (RBF)
    Replaced,
}

/// A single transaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    /// Unique transaction ID
    pub id: String,
    /// User ID who owns this transaction
    pub user_id: String,
    /// Transaction hash on blockchain
    pub txid: String,
    /// Asset/currency
    pub asset: QuoteAsset,
    /// Transaction type
    pub tx_type: TxType,
    /// Current status
    pub status: TxStatus,
    /// Amount in base units (BTC, not satoshis)
    pub amount: f64,
    /// Fee paid (in base units)
    pub fee: f64,
    /// Sender address
    pub from_address: String,
    /// Recipient address
    pub to_address: String,
    /// Number of confirmations
    pub confirmations: u32,
    /// Block height (if confirmed)
    pub block_height: Option<u64>,
    /// Block hash (if confirmed)
    pub block_hash: Option<String>,
    /// Transaction timestamp
    pub timestamp: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Optional memo/note
    pub memo: Option<String>,
    /// Raw transaction hex (optional)
    pub raw_hex: Option<String>,
}

impl TransactionRecord {
    /// Create a new transaction record
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_id: &str,
        txid: &str,
        asset: QuoteAsset,
        tx_type: TxType,
        amount: f64,
        fee: f64,
        from_address: &str,
        to_address: &str,
    ) -> Self {
        let now = Utc::now();
        let id = format!(
            "{}_{}_{}_{}",
            user_id,
            asset.as_str(),
            txid,
            now.timestamp()
        );

        Self {
            id,
            user_id: user_id.to_string(),
            txid: txid.to_string(),
            asset,
            tx_type,
            status: TxStatus::Pending,
            amount,
            fee,
            from_address: from_address.to_string(),
            to_address: to_address.to_string(),
            confirmations: 0,
            block_height: None,
            block_hash: None,
            timestamp: now,
            updated_at: now,
            memo: None,
            raw_hex: None,
        }
    }

    /// Check if transaction is confirmed
    pub fn is_confirmed(&self) -> bool {
        self.status == TxStatus::Confirmed || self.confirmations >= 1
    }

    /// Get status description
    pub fn status_description(&self) -> &str {
        match self.status {
            TxStatus::Pending => "Pending in mempool",
            TxStatus::Confirmed => "Confirmed on blockchain",
            TxStatus::Failed => "Transaction failed",
            TxStatus::Replaced => "Replaced by another transaction",
        }
    }
}

/// In-memory transaction storage
/// Structure: user_id -> Vec<TransactionRecord>
static TX_HISTORY: Lazy<Arc<Mutex<HashMap<String, Vec<TransactionRecord>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Transaction history manager
pub struct TxHistoryManager;

impl TxHistoryManager {
    /// Add a new transaction to history
    pub fn add_transaction(tx: TransactionRecord) -> Result<()> {
        let mut storage = TX_HISTORY
            .lock()
            .map_err(|e| anyhow!("Failed to lock transaction storage: {}", e))?;

        let user_txs = storage.entry(tx.user_id.clone()).or_insert_with(Vec::new);

        // Check if transaction already exists
        if user_txs
            .iter()
            .any(|t| t.txid == tx.txid && t.asset == tx.asset)
        {
            tracing::debug!(
                "Transaction {} already exists for user {}",
                tx.txid,
                tx.user_id
            );
            return Ok(());
        }

        user_txs.push(tx.clone());
        tracing::info!(
            "📝 Added transaction {} to history for user {}",
            tx.txid,
            tx.user_id
        );

        // Send WebSocket notification for new transaction
        crate::ws_notifications::WsNotificationManager::notify_new_transaction(
            &tx.user_id,
            &tx.txid,
            tx.asset,
            tx.tx_type,
            tx.amount,
            &tx.from_address,
            &tx.to_address,
        );

        Ok(())
    }

    /// Get all transactions for a user
    pub fn get_user_transactions(user_id: &str) -> Vec<TransactionRecord> {
        TX_HISTORY
            .lock()
            .ok()
            .and_then(|storage| storage.get(user_id).cloned())
            .unwrap_or_default()
    }

    /// Get transactions for a user filtered by asset
    pub fn get_user_transactions_by_asset(
        user_id: &str,
        asset: QuoteAsset,
    ) -> Vec<TransactionRecord> {
        Self::get_user_transactions(user_id)
            .into_iter()
            .filter(|tx| tx.asset == asset)
            .collect()
    }

    /// Get transactions for a user filtered by status
    pub fn get_user_transactions_by_status(
        user_id: &str,
        status: TxStatus,
    ) -> Vec<TransactionRecord> {
        Self::get_user_transactions(user_id)
            .into_iter()
            .filter(|tx| tx.status == status)
            .collect()
    }

    /// Get transactions for a user filtered by type
    pub fn get_user_transactions_by_type(user_id: &str, tx_type: TxType) -> Vec<TransactionRecord> {
        Self::get_user_transactions(user_id)
            .into_iter()
            .filter(|tx| tx.tx_type == tx_type)
            .collect()
    }

    /// Get a specific transaction by txid
    pub fn get_transaction(
        user_id: &str,
        txid: &str,
        asset: QuoteAsset,
    ) -> Option<TransactionRecord> {
        Self::get_user_transactions(user_id)
            .into_iter()
            .find(|tx| tx.txid == txid && tx.asset == asset)
    }

    /// Update transaction status and confirmations
    pub fn update_transaction_status(
        user_id: &str,
        txid: &str,
        asset: QuoteAsset,
        status: TxStatus,
        confirmations: u32,
        block_height: Option<u64>,
        block_hash: Option<String>,
    ) -> Result<()> {
        let mut storage = TX_HISTORY
            .lock()
            .map_err(|e| anyhow!("Failed to lock transaction storage: {}", e))?;

        if let Some(user_txs) = storage.get_mut(user_id) {
            if let Some(tx) = user_txs
                .iter_mut()
                .find(|t| t.txid == txid && t.asset == asset)
            {
                tx.status = status;
                tx.confirmations = confirmations;
                tx.block_height = block_height;
                tx.block_hash = block_hash;
                tx.updated_at = Utc::now();

                tracing::debug!(
                    "Updated transaction {} status: {:?}, confirmations: {}",
                    txid,
                    status,
                    confirmations
                );
                return Ok(());
            }
        }

        Err(anyhow!(
            "Transaction {} not found for user {}",
            txid,
            user_id
        ))
    }

    /// Update transaction confirmations from blockchain data
    pub fn update_confirmations(
        user_id: &str,
        txid: &str,
        asset: QuoteAsset,
        confirmations: u32,
    ) -> Result<()> {
        let mut storage = TX_HISTORY
            .lock()
            .map_err(|e| anyhow!("Failed to lock transaction storage: {}", e))?;

        if let Some(user_txs) = storage.get_mut(user_id) {
            if let Some(tx) = user_txs
                .iter_mut()
                .find(|t| t.txid == txid && t.asset == asset)
            {
                if tx.confirmations != confirmations {
                    tx.confirmations = confirmations;
                    tx.updated_at = Utc::now();

                    // Update status to confirmed if we have confirmations
                    if confirmations >= 1 && tx.status == TxStatus::Pending {
                        tx.status = TxStatus::Confirmed;
                    }

                    tracing::debug!(
                        "Updated transaction {} confirmations: {}",
                        txid,
                        confirmations
                    );
                }
                return Ok(());
            }
        }

        Err(anyhow!(
            "Transaction {} not found for user {}",
            txid,
            user_id
        ))
    }

    /// Add memo to transaction
    pub fn add_memo(user_id: &str, txid: &str, asset: QuoteAsset, memo: String) -> Result<()> {
        let mut storage = TX_HISTORY
            .lock()
            .map_err(|e| anyhow!("Failed to lock transaction storage: {}", e))?;

        if let Some(user_txs) = storage.get_mut(user_id) {
            if let Some(tx) = user_txs
                .iter_mut()
                .find(|t| t.txid == txid && t.asset == asset)
            {
                tx.memo = Some(memo);
                tx.updated_at = Utc::now();
                return Ok(());
            }
        }

        Err(anyhow!(
            "Transaction {} not found for user {}",
            txid,
            user_id
        ))
    }

    /// Get transaction count for a user
    pub fn get_transaction_count(user_id: &str) -> usize {
        Self::get_user_transactions(user_id).len()
    }

    /// Get pending transactions for a user
    pub fn get_pending_transactions(user_id: &str) -> Vec<TransactionRecord> {
        Self::get_user_transactions_by_status(user_id, TxStatus::Pending)
    }

    /// Get recent transactions for a user (last N)
    pub fn get_recent_transactions(user_id: &str, limit: usize) -> Vec<TransactionRecord> {
        let mut txs = Self::get_user_transactions(user_id);
        txs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        txs.into_iter().take(limit).collect()
    }

    /// Background task to update confirmations for pending transactions
    pub async fn sync_pending_transactions() -> Result<usize> {
        let mut updated_count = 0;

        // Get snapshot of all pending transactions
        let pending_txs: Vec<(String, String, QuoteAsset)> = {
            let storage = TX_HISTORY
                .lock()
                .map_err(|e| anyhow!("Failed to lock transaction storage: {}", e))?;

            let mut pending = Vec::new();
            for (user_id, txs) in storage.iter() {
                for tx in txs {
                    if tx.status == TxStatus::Pending || tx.confirmations < 6 {
                        pending.push((user_id.clone(), tx.txid.clone(), tx.asset));
                    }
                }
            }
            pending
        };

        if pending_txs.is_empty() {
            return Ok(0);
        }

        tracing::debug!("🔄 Syncing {} pending transactions", pending_txs.len());

        // Check each pending transaction
        for (user_id, txid, asset) in pending_txs {
            match Self::fetch_transaction_status(&txid, asset).await {
                Ok((confirmations, block_height, block_hash)) => {
                    let status = if confirmations >= 1 {
                        TxStatus::Confirmed
                    } else {
                        TxStatus::Pending
                    };

                    if let Err(e) = Self::update_transaction_status(
                        &user_id,
                        &txid,
                        asset,
                        status,
                        confirmations,
                        block_height,
                        block_hash,
                    ) {
                        tracing::warn!("Failed to update transaction {}: {}", txid, e);
                    } else {
                        updated_count += 1;

                        // Send WebSocket notification
                        crate::ws_notifications::WsNotificationManager::notify_transaction_update(
                            &user_id,
                            &txid,
                            asset,
                            status,
                            confirmations,
                            block_height,
                        );
                    }
                }
                Err(e) => {
                    tracing::debug!("Could not fetch status for transaction {}: {}", txid, e);
                }
            }
        }

        Ok(updated_count)
    }

    /// Fetch transaction status from blockchain via RPC
    async fn fetch_transaction_status(
        txid: &str,
        asset: QuoteAsset,
    ) -> Result<(u32, Option<u64>, Option<String>)> {
        use crate::external_rpc::ExternalChain;

        let chain = match asset {
            QuoteAsset::Btc => ExternalChain::Btc,
            QuoteAsset::Bch => ExternalChain::Bch,
            QuoteAsset::Doge => ExternalChain::Doge,
            QuoteAsset::Land => return Err(anyhow!("LAND is not an external chain")),
        };

        // Get RPC client
        let client = {
            let clients = crate::EXTERNAL_RPC_CLIENTS.lock();
            clients
                .get(&chain)
                .ok_or_else(|| anyhow!("RPC client not available for {:?}", chain))?
                .clone()
        };

        // Call gettransaction RPC
        let result = client
            .call("gettransaction", serde_json::json!([txid, true]))
            .await?;

        let confirmations = result["confirmations"].as_u64().unwrap_or(0) as u32;

        let block_height = result["blockheight"].as_u64();

        let block_hash = result["blockhash"].as_str().map(|s| s.to_string());

        Ok((confirmations, block_height, block_hash))
    }

    /// Clear all transaction history (for testing)
    #[cfg(test)]
    pub fn clear_all() {
        let mut storage = TX_HISTORY.lock().unwrap();
        storage.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_transaction_record() {
        let tx = TransactionRecord::new(
            "test_user",
            "abc123",
            QuoteAsset::Btc,
            TxType::Send,
            0.5,
            0.0001,
            "sender_addr",
            "receiver_addr",
        );

        assert_eq!(tx.user_id, "test_user");
        assert_eq!(tx.txid, "abc123");
        assert_eq!(tx.asset, QuoteAsset::Btc);
        assert_eq!(tx.tx_type, TxType::Send);
        assert_eq!(tx.status, TxStatus::Pending);
        assert_eq!(tx.confirmations, 0);
        assert!(!tx.is_confirmed());
    }

    #[test]
    fn test_add_and_retrieve_transaction() {
        TxHistoryManager::clear_all();

        let tx = TransactionRecord::new(
            "test_user_2",
            "txid123",
            QuoteAsset::Btc,
            TxType::Receive,
            1.0,
            0.0,
            "sender",
            "receiver",
        );

        TxHistoryManager::add_transaction(tx.clone()).unwrap();

        let retrieved =
            TxHistoryManager::get_transaction("test_user_2", "txid123", QuoteAsset::Btc);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().amount, 1.0);
    }

    #[test]
    fn test_update_confirmations() {
        TxHistoryManager::clear_all();

        let tx = TransactionRecord::new(
            "test_user_3",
            "txid456",
            QuoteAsset::Bch,
            TxType::Send,
            0.5,
            0.0001,
            "from",
            "to",
        );

        TxHistoryManager::add_transaction(tx).unwrap();

        TxHistoryManager::update_confirmations("test_user_3", "txid456", QuoteAsset::Bch, 6)
            .unwrap();

        let updated =
            TxHistoryManager::get_transaction("test_user_3", "txid456", QuoteAsset::Bch).unwrap();
        assert_eq!(updated.confirmations, 6);
        assert_eq!(updated.status, TxStatus::Confirmed);
        assert!(updated.is_confirmed());
    }

    #[test]
    fn test_filter_by_type() {
        TxHistoryManager::clear_all();

        let tx1 = TransactionRecord::new(
            "user",
            "tx1",
            QuoteAsset::Btc,
            TxType::Send,
            1.0,
            0.0001,
            "a",
            "b",
        );
        let tx2 = TransactionRecord::new(
            "user",
            "tx2",
            QuoteAsset::Btc,
            TxType::Receive,
            2.0,
            0.0,
            "c",
            "d",
        );

        TxHistoryManager::add_transaction(tx1).unwrap();
        TxHistoryManager::add_transaction(tx2).unwrap();

        let sends = TxHistoryManager::get_user_transactions_by_type("user", TxType::Send);
        let receives = TxHistoryManager::get_user_transactions_by_type("user", TxType::Receive);

        assert_eq!(sends.len(), 1);
        assert_eq!(receives.len(), 1);
    }
}
