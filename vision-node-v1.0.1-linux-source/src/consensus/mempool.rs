// Mempool: In-memory transaction pool for unconfirmed transactions
// Phase 2 Feature #3: Transaction Relay & Mempool Management

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
};
use crate::blockchain::transaction::{Transaction, TxId};

/// Thread-safe mempool for managing unconfirmed transactions
#[derive(Clone, Default)]
pub struct Mempool {
    inner: Arc<Mutex<Pool>>,
}

#[derive(Default)]
struct Pool {
    /// Transaction storage by ID
    by_id: HashMap<TxId, Transaction>,
    /// Insertion order (FIFO for now; TODO: fee-based priority)
    order: VecDeque<TxId>,
}

impl Mempool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a transaction into the mempool
    /// Returns true if inserted, false if duplicate
    pub fn insert(&self, tx: Transaction) -> bool {
        let mut p = self.inner.lock().unwrap();
        let id = tx.txid();
        
        if p.by_id.contains_key(&id) {
            return false; // Already in mempool
        }
        
        p.order.push_back(id.clone());
        p.by_id.insert(id, tx);
        true
    }

    /// Check if transaction exists in mempool
    pub fn has(&self, id: &TxId) -> bool {
        self.inner.lock().unwrap().by_id.contains_key(id)
    }

    /// Get transaction by ID
    pub fn get(&self, id: &TxId) -> Option<Transaction> {
        self.inner.lock().unwrap().by_id.get(id).cloned()
    }

    /// List transaction IDs (for /mempool endpoint)
    pub fn list_ids(&self, max: usize) -> Vec<TxId> {
        let p = self.inner.lock().unwrap();
        p.order.iter().take(max).cloned().collect()
    }

    /// Get all transactions (for debugging/UI)
    pub fn list_transactions(&self, max: usize) -> Vec<Transaction> {
        let p = self.inner.lock().unwrap();
        p.order.iter()
            .take(max)
            .filter_map(|id| p.by_id.get(id).cloned())
            .collect()
    }

    /// Select transactions for block template
    /// Naive FIFO selection; TODO: replace with fee-based sorting
    pub fn select_for_block(&self, max_count: usize, max_weight: u64) -> Vec<Transaction> {
        let p = self.inner.lock().unwrap();
        let mut selected = Vec::new();
        let mut weight = 0u64;

        for id in p.order.iter() {
            if let Some(tx) = p.by_id.get(id) {
                let tx_weight = tx.estimated_weight();
                
                // Skip if would exceed weight limit
                if weight + tx_weight > max_weight {
                    continue;
                }
                
                selected.push(tx.clone());
                weight += tx_weight;
                
                // Stop if reached max count
                if selected.len() >= max_count {
                    break;
                }
            }
        }

        selected
    }

    /// Remove confirmed transactions after block integration
    pub fn remove_many(&self, ids: &HashSet<TxId>) {
        let mut p = self.inner.lock().unwrap();
        
        // Remove from order queue
        p.order.retain(|id| !ids.contains(id));
        
        // Remove from storage
        for id in ids {
            p.by_id.remove(id);
        }
    }

    /// Remove single transaction
    pub fn remove(&self, id: &TxId) {
        let mut p = self.inner.lock().unwrap();
        p.order.retain(|tid| tid != id);
        p.by_id.remove(id);
    }

    /// Get current mempool size
    pub fn size(&self) -> usize {
        self.inner.lock().unwrap().by_id.len()
    }

    /// Clear entire mempool (for testing/reorg)
    pub fn clear(&self) {
        let mut p = self.inner.lock().unwrap();
        p.by_id.clear();
        p.order.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_tx(id: &str) -> Transaction {
        // Create minimal transaction for testing
        Transaction {
            from: format!("addr_{}", id),
            to: format!("dest_{}", id),
            amount: 1000,
            fee: 100,
            nonce: 0,
            signature: String::new(),
            timestamp: 0,
        }
    }

    #[test]
    fn test_insert_and_get() {
        let mempool = Mempool::new();
        let tx = dummy_tx("test1");
        let txid = tx.txid();

        assert!(mempool.insert(tx.clone()));
        assert!(!mempool.insert(tx.clone())); // duplicate
        assert!(mempool.has(&txid));
        assert_eq!(mempool.get(&txid).unwrap().from, "addr_test1");
    }

    #[test]
    fn test_remove_many() {
        let mempool = Mempool::new();
        let tx1 = dummy_tx("1");
        let tx2 = dummy_tx("2");
        let tx3 = dummy_tx("3");

        mempool.insert(tx1.clone());
        mempool.insert(tx2.clone());
        mempool.insert(tx3.clone());

        assert_eq!(mempool.size(), 3);

        let mut to_remove = HashSet::new();
        to_remove.insert(tx1.txid());
        to_remove.insert(tx3.txid());

        mempool.remove_many(&to_remove);

        assert_eq!(mempool.size(), 1);
        assert!(mempool.has(&tx2.txid()));
        assert!(!mempool.has(&tx1.txid()));
    }

    #[test]
    fn test_select_for_block() {
        let mempool = Mempool::new();
        
        // Add 10 transactions
        for i in 0..10 {
            mempool.insert(dummy_tx(&i.to_string()));
        }

        // Select first 5
        let selected = mempool.select_for_block(5, u64::MAX);
        assert_eq!(selected.len(), 5);
    }
}
