// Performance optimization utilities for Vision Node
//
// Provides parallel processing for CPU-intensive operations like
// signature verification and block validation.
#![allow(dead_code)]

use crate::{verify_tx, Block, NodeError, Tx};
use rayon::prelude::*;

/// Verify multiple transaction signatures in parallel using Rayon
///
/// Returns Ok(()) if all signatures are valid, or the first error encountered.
/// Uses all available CPU cores for maximum throughput.
pub fn verify_transactions_parallel(txs: &[Tx]) -> Result<(), NodeError> {
    let _span =
        tracing::debug_span!("verify_transactions_parallel", tx_count = txs.len()).entered();

    // Use par_iter for parallel verification
    txs.par_iter().try_for_each(verify_tx)?;

    tracing::debug!("all signatures verified successfully");
    Ok(())
}

/// Verify block transactions in parallel
///
/// More efficient than sequential verification for blocks with many transactions.
/// Recommended for blocks with 10+ transactions.
pub fn verify_block_transactions_parallel(block: &Block) -> Result<(), NodeError> {
    let _span = tracing::debug_span!(
        "verify_block_transactions",
        block_height = block.header.number,
        tx_count = block.txs.len()
    )
    .entered();

    if block.txs.is_empty() {
        return Ok(());
    }

    // Parallel verification is only beneficial for larger transaction counts
    if block.txs.len() >= 10 {
        tracing::debug!("using parallel verification");
        verify_transactions_parallel(&block.txs)
    } else {
        tracing::debug!("using sequential verification (small block)");
        for tx in &block.txs {
            verify_tx(tx)?;
        }
        Ok(())
    }
}

/// Mining block template cache
///
/// Pre-computes block templates off the critical mining path to reduce
/// latency in block production.
pub mod mining_template {
    use crate::Block;
    use once_cell::sync::Lazy;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime};

    /// Cached block template with timestamp
    #[derive(Clone)]
    pub struct CachedTemplate {
        pub template: Block,
        pub created_at: SystemTime,
    }

    /// Global template cache with 500ms TTL
    static TEMPLATE_CACHE: Lazy<Arc<Mutex<Option<CachedTemplate>>>> =
        Lazy::new(|| Arc::new(Mutex::new(None)));

    /// Get cached template if still valid (< 500ms old)
    pub fn get_cached_template() -> Option<Block> {
        let cache = TEMPLATE_CACHE.lock().ok()?;
        let cached = cache.as_ref()?;

        let age = SystemTime::now()
            .duration_since(cached.created_at)
            .unwrap_or(Duration::from_secs(999));

        if age < Duration::from_millis(500) {
            // Phase 4: Record cache hit metric
            crate::PROM_VISION_CACHE_HITS.inc();
            tracing::debug!("mining template cache hit");
            Some(cached.template.clone())
        } else {
            // Phase 4: Record cache miss (expired)
            crate::PROM_VISION_CACHE_MISSES.inc();
            tracing::debug!("template expired, cache miss");
            None
        }
    }

    /// Update template cache
    pub fn set_cached_template(template: Block) {
        if let Ok(mut cache) = TEMPLATE_CACHE.lock() {
            *cache = Some(CachedTemplate {
                template,
                created_at: SystemTime::now(),
            });
            tracing::debug!("block template cached");
        }
    }

    /// Clear template cache (call after new block arrives)
    pub fn invalidate_cache() {
        if let Ok(mut cache) = TEMPLATE_CACHE.lock() {
            *cache = None;
            tracing::debug!("block template cache invalidated");
        }
    }
}

/// Batch processing utilities
pub mod batch {
    use std::collections::BTreeMap;

    /// Process items in parallel batches
    ///
    /// Splits work into chunks for parallel processing, useful for
    /// large datasets that need transformation.
    pub fn process_parallel<T, R, F>(items: &[T], f: F) -> Vec<R>
    where
        T: Sync,
        R: Send,
        F: Fn(&T) -> R + Sync + Send,
    {
        use rayon::prelude::*;
        items.par_iter().map(f).collect()
    }

    /// Group items by key efficiently
    pub fn group_by<T, K, F>(items: Vec<T>, key_fn: F) -> BTreeMap<K, Vec<T>>
    where
        K: Ord,
        F: Fn(&T) -> K,
    {
        let mut groups: BTreeMap<K, Vec<T>> = BTreeMap::new();
        for item in items {
            let key = key_fn(&item);
            groups.entry(key).or_default().push(item);
        }
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_verification_empty() {
        let txs: Vec<Tx> = vec![];
        let result = verify_transactions_parallel(&txs);
        assert!(result.is_ok());
    }

    #[test]
    fn test_template_cache() {
        use super::mining_template::*;

        // Initially empty
        assert!(get_cached_template().is_none());

        // Create a dummy block
        let block = crate::Block {
            header: crate::BlockHeader {
                parent_hash: "0".repeat(64),
                number: 100,
                timestamp: 1234567890,
                difficulty: 42,
                nonce: 0,
                pow_hash: "0".repeat(64),
                state_root: "0".repeat(64),
                tx_root: "0".repeat(64),
                receipts_root: "0".repeat(64),
                da_commitment: None,
                miner: "miner".to_string(),
                base_fee_per_gas: 0,
            },
            txs: vec![],
            weight: 1,
            agg_signature: None,
        };

        // Cache it
        set_cached_template(block.clone());

        // Should retrieve it
        let cached = get_cached_template();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().header.number, 100);

        // Invalidate
        invalidate_cache();
        assert!(get_cached_template().is_none());
    }

    #[test]
    fn test_batch_group_by() {
        use super::batch::*;

        let items = vec![
            ("alice", 100),
            ("bob", 200),
            ("alice", 50),
            ("carol", 75),
            ("bob", 300),
        ];

        let groups = group_by(items, |(name, _)| name.to_string());

        assert_eq!(groups.len(), 3);
        assert_eq!(groups.get("alice").unwrap().len(), 2);
        assert_eq!(groups.get("bob").unwrap().len(), 2);
        assert_eq!(groups.get("carol").unwrap().len(), 1);
    }
}
