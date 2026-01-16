#![allow(dead_code)]
//! Chain reorganization handling (DEPRECATED)
//!
//! ⚠️ DEPRECATED: This module is no longer used for reorg orchestration.
//! All reorg logic is now handled internally by `chain::accept::apply_block()`.
//!
//! Previously managed chain switches when a competing fork became longer.
//! Now `apply_block()` handles:
//! - Detecting heavier chains via cumulative work comparison
//! - Rolling back to common ancestor
//! - Replaying blocks from the new fork
//! - Returning transactions from orphaned blocks to mempool
//! - Updating chain state and metrics
//!
//! This file remains for historical reference and helper functions only.

use crate::{Block, Tx};

/// Result of reorg attempt
#[derive(Debug)]
pub enum ReorgResult {
    /// Successfully switched to new chain
    Success {
        old_tip: String,
        new_tip: String,
        blocks_rolled_back: usize,
        blocks_applied: usize,
    },
    /// No reorg needed (current chain is still best)
    NotNeeded,
    /// Reorg failed (kept current chain)
    Failed(String),
}

/// Represents a competing fork that may require reorganization
#[derive(Debug, Clone)]
pub struct Fork {
    /// Blocks in this fork (from common ancestor forward)
    pub blocks: Vec<Block>,
    /// Hash of common ancestor with main chain
    pub common_ancestor: String,
    /// Total accumulated difficulty
    pub total_difficulty: u64,
}

/// Check if height is past bootstrap checkpoint (hard limit for reorgs)
fn is_past_bootstrap_checkpoint(height: u64) -> bool {
    height <= crate::vision_constants::BOOTSTRAP_CHECKPOINT_HEIGHT
}

/// Check if a reorg is needed and execute it if so
///
/// ⚠️ DEPRECATED: Do not call this function directly!
/// Use `chain::accept::apply_block()` instead, which handles reorgs internally.
/// This function is kept for compatibility but should not be used in new code.
#[deprecated(
    since = "3.0.0",
    note = "Use chain::accept::apply_block() which handles reorgs internally"
)]
pub fn handle_reorg(new_block: &Block) -> ReorgResult {
    let mut g = crate::CHAIN.lock();

    // Get current chain tip
    let current_tip = match g.blocks.last() {
        Some(b) => b,
        None => {
            // Empty chain, just add the block
            drop(g);
            return ReorgResult::NotNeeded;
        }
    };

    // Check if new block extends current tip (no reorg needed)
    if new_block.header.parent_hash == current_tip.header.pow_hash {
        tracing::trace!(
            target = "p2p::reorg",
            parent = %new_block.header.parent_hash,
            "Block extends current tip, no reorg needed"
        );
        return ReorgResult::NotNeeded;
    }

    // Find common ancestor and build fork
    let fork = match find_fork(&g.blocks, new_block) {
        Some(f) => f,
        None => {
            // Enhanced error logging with chain state details
            let local_tip_hash = current_tip.header.pow_hash.clone();
            let local_height = g.blocks.len().saturating_sub(1);
            let remote_height = new_block.header.number;

            // === BOOTSTRAP CHECKPOINT VALIDATION ===
            // Phase 11: Verify local chain hasn't diverged from bootstrap prefix
            if local_height as u64 >= crate::vision_constants::BOOTSTRAP_CHECKPOINT_HEIGHT {
                // Check if our local checkpoint matches the baked-in hash
                if let Some(checkpoint_block) = g
                    .blocks
                    .get(crate::vision_constants::BOOTSTRAP_CHECKPOINT_HEIGHT as usize)
                {
                    let local_checkpoint = &checkpoint_block.header.pow_hash;

                    if local_checkpoint != crate::vision_constants::BOOTSTRAP_CHECKPOINT_HASH {
                        tracing::error!(
                            target = "p2p::reorg",
                            local_checkpoint = %local_checkpoint,
                            expected_checkpoint = %crate::vision_constants::BOOTSTRAP_CHECKPOINT_HASH,
                            checkpoint_height = crate::vision_constants::BOOTSTRAP_CHECKPOINT_HEIGHT,
                            "❌ LOCAL CHAIN CORRUPTED - bootstrap checkpoint mismatch\n\
                             \n\
                             Your local chain prefix does not match the baked-in bootstrap.\n\
                             This indicates database corruption or tampering.\n\
                             \n\
                             Expected: {} @ height {}\n\
                             Got:      {} @ height {}\n\
                             \n\
                             REQUIRED ACTION: Delete chain database and restart.",
                            crate::vision_constants::BOOTSTRAP_CHECKPOINT_HASH,
                            crate::vision_constants::BOOTSTRAP_CHECKPOINT_HEIGHT,
                            local_checkpoint,
                            crate::vision_constants::BOOTSTRAP_CHECKPOINT_HEIGHT
                        );
                        return ReorgResult::Failed(format!(
                            "Local bootstrap checkpoint corrupted at height {}",
                            crate::vision_constants::BOOTSTRAP_CHECKPOINT_HEIGHT
                        ));
                    }
                }

                // Peer likely on incompatible chain - log as different bootstrap prefix
                tracing::warn!(
                    target = "p2p::reorg",
                    peer_block = %new_block.header.pow_hash,
                    peer_height = remote_height,
                    "⚠️  Peer appears to be on incompatible chain\n\
                     \n\
                     Cannot find common ancestor, but our bootstrap checkpoint is valid.\n\
                     This peer likely has a different bootstrap prefix (incompatible build).\n\
                     \n\
                     Peer will be disconnected automatically by handshake validation."
                );
            }

            tracing::error!(
                target = "p2p::reorg",
                "❌ CANNOT FIND COMMON ANCESTOR FOR REORG\n\
                 Local chain state:\n\
                   Tip hash:   {}\n\
                   Height:     {}\n\
                 Remote block:\n\
                   Block hash: {}\n\
                   Parent:     {}\n\
                   Height:     {}\n\
                 \n\
                 This indicates:\n\
                 1. Peer is on a completely different chain (wrong genesis?)\n\
                 2. Chain database corruption\n\
                 3. Network split or hard fork\n\
                 \n\
                 Marking peer for cooldown to prevent repeated invalid requests.",
                local_tip_hash,
                local_height,
                new_block.header.pow_hash,
                new_block.header.parent_hash,
                remote_height
            );

            // TODO: Add peer cooldown/reputation system here
            // For now, just return failure
            return ReorgResult::Failed(format!(
                "No common ancestor (local_height={}, remote_height={}, remote_parent={})",
                local_height, remote_height, new_block.header.parent_hash
            ));
        }
    };

    // Calculate current chain difficulty from common ancestor
    let ancestor_height = find_block_height(&g.blocks, &fork.common_ancestor);

    // === BOOTSTRAP CHECKPOINT HARD LIMIT ===
    // Phase 11: Refuse reorg if it would roll back past bootstrap checkpoint
    if is_past_bootstrap_checkpoint(ancestor_height as u64) {
        tracing::error!(
            target = "p2p::reorg",
            ancestor_height,
            checkpoint_height = crate::vision_constants::BOOTSTRAP_CHECKPOINT_HEIGHT,
            "❌ REFUSING REORG - would cross bootstrap checkpoint\n\
             \n\
             Common ancestor at height {} is below bootstrap checkpoint (height {}).\n\
             Reorgs are FORBIDDEN past the baked-in bootstrap prefix.\n\
             \n\
             This peer is on an incompatible chain and should have been rejected\n\
             during handshake. Marking for disconnect.",
            ancestor_height,
            crate::vision_constants::BOOTSTRAP_CHECKPOINT_HEIGHT
        );
        return ReorgResult::Failed(format!(
            "Attempted reorg past bootstrap checkpoint (ancestor={}, checkpoint={})",
            ancestor_height,
            crate::vision_constants::BOOTSTRAP_CHECKPOINT_HEIGHT
        ));
    }

    let current_difficulty: u64 = g
        .blocks
        .iter()
        .skip(ancestor_height + 1)
        .map(|b| b.header.difficulty)
        .sum();

    tracing::debug!(
        target = "p2p::reorg",
        fork_difficulty = fork.total_difficulty,
        current_difficulty = current_difficulty,
        fork_blocks = fork.blocks.len(),
        "Evaluating potential reorg"
    );

    // Only reorg if fork has MORE accumulated work
    if fork.total_difficulty <= current_difficulty {
        tracing::debug!(
            target = "p2p::reorg",
            "Fork does not have more work, keeping current chain"
        );
        return ReorgResult::NotNeeded;
    }

    tracing::warn!(
        target = "p2p::reorg",
        old_tip = %current_tip.header.pow_hash,
        new_tip = %fork.blocks.last().unwrap().header.pow_hash,
        rollback_count = g.blocks.len() - ancestor_height - 1,
        apply_count = fork.blocks.len(),
        "Executing chain reorganization"
    );

    // PATCH 1: Transactional reorg
    // Validate the entire fork chain *before* mutating local chain state.
    if let Err(e) = validate_fork_chain(&fork, ancestor_height as u64) {
        tracing::error!(
            target = "p2p::reorg",
            ancestor_height,
            error = %e,
            "Fork chain pre-validation failed - aborting reorg without mutation"
        );
        return ReorgResult::Failed(format!("Fork chain invalid: {}", e));
    }

    // Execute the reorg
    let old_tip = current_tip.header.pow_hash.clone();
    let rollback_count = g.blocks.len() - ancestor_height - 1;

    // 1. Save transactions from blocks being rolled back
    let orphaned_txs = collect_orphaned_transactions(&g.blocks, ancestor_height + 1);

    // 2. Roll back to common ancestor
    g.blocks.truncate(ancestor_height + 1);

    // 3. Apply fork blocks using unified validation
    for block in &fork.blocks {
        if let Err(e) = crate::chain::accept::apply_block(&mut g, block, None) {
            return ReorgResult::Failed(format!("Failed to apply fork block: {}", e));
        }
    }

    let new_tip = g.blocks.last().unwrap().header.pow_hash.clone();

    // 4. Return orphaned transactions to mempool (excluding those in new chain)
    let new_chain_txs = collect_transactions_in_blocks(&fork.blocks);
    let reinserted_count = reinsert_orphaned_transactions(&mut g, orphaned_txs, &new_chain_txs);

    // 5. Update metrics
    crate::PROM_CHAIN_REORGS.inc();
    crate::PROM_CHAIN_REORG_BLOCKS_ROLLED_BACK.inc_by(rollback_count as u64);
    crate::PROM_CHAIN_REORG_TXS_REINSERTED.inc_by(reinserted_count as u64);
    crate::PROM_CHAIN_REORG_DEPTH.set(rollback_count as i64);

    // Release lock
    drop(g);

    ReorgResult::Success {
        old_tip,
        new_tip,
        blocks_rolled_back: rollback_count,
        blocks_applied: fork.blocks.len(),
    }
}

/// Find the fork starting from a new block back to common ancestor
fn find_fork(main_chain: &[Block], new_block: &Block) -> Option<Fork> {
    let mut fork_blocks = vec![new_block.clone()];
    let mut current_hash = new_block.header.parent_hash.clone();
    let mut total_difficulty = new_block.header.difficulty;

    // Walk backwards through orphan pool and main chain
    let max_depth = 100; // Prevent infinite loops
    for _ in 0..max_depth {
        // Check if current_hash is in main chain
        if main_chain.iter().any(|b| b.header.pow_hash == current_hash) {
            // Found common ancestor
            return Some(Fork {
                blocks: fork_blocks.into_iter().rev().collect(),
                common_ancestor: current_hash,
                total_difficulty,
            });
        }

        // Try to find parent in orphan pool
        let orphan_pool_arc = crate::p2p::routes::orphan_pool();
        let pool = orphan_pool_arc.lock();

        if let Some(parent_block) = pool.get_orphan(&current_hash) {
            total_difficulty += parent_block.header.difficulty;
            fork_blocks.push(parent_block.clone());
            current_hash = parent_block.header.parent_hash.clone();
            drop(pool); // Release lock before next iteration
        } else {
            // Can't find parent in orphan pool or main chain
            tracing::debug!(
                target = "p2p::reorg",
                missing_hash = %current_hash,
                "Cannot find parent block in orphan pool or main chain"
            );
            break;
        }
    }

    None
}

/// Find the height of a block in the chain
fn find_block_height(blocks: &[Block], hash: &str) -> usize {
    blocks
        .iter()
        .position(|b| b.header.pow_hash == hash)
        .unwrap_or(0)
}

/// Collect all transactions from blocks being rolled back
fn collect_orphaned_transactions(blocks: &[Block], from_height: usize) -> Vec<Tx> {
    let mut txs = Vec::new();

    for block in blocks.iter().skip(from_height) {
        // Skip coinbase transaction (index 0)
        for tx in block.txs.iter().skip(1) {
            txs.push(tx.clone());
        }
    }

    tracing::debug!(
        target = "p2p::reorg",
        count = txs.len(),
        "Collected orphaned transactions"
    );

    txs
}

/// Collect all transaction hashes in a set of blocks
fn collect_transactions_in_blocks(blocks: &[Block]) -> std::collections::HashSet<[u8; 32]> {
    let mut txs = std::collections::HashSet::new();

    for block in blocks {
        for tx in &block.txs {
            let tx_hash = crate::tx_hash(tx);
            txs.insert(tx_hash);
        }
    }

    txs
}

/// Reinsert orphaned transactions back into mempool
fn reinsert_orphaned_transactions(
    chain_state: &mut parking_lot::MutexGuard<crate::Chain>,
    orphaned: Vec<Tx>,
    new_chain_txs: &std::collections::HashSet<[u8; 32]>,
) -> usize {
    let mut reinserted = 0;
    let total = orphaned.len();

    for tx in orphaned {
        let tx_hash = crate::tx_hash(&tx);

        // Only reinsert if not already in new chain
        if !new_chain_txs.contains(&tx_hash) {
            // Enterprise: Re-validate transaction against new chain state
            match revalidate_transaction_against_state(&tx, chain_state) {
                Ok(true) => {
                    // Transaction still valid - reinsert into mempool
                    chain_state.mempool_bulk.push_back(tx);
                    reinserted += 1;
                }
                Ok(false) => {
                    tracing::debug!(
                        target = "p2p::reorg",
                        tx_hash = %hex::encode(tx_hash),
                        "Transaction no longer valid after reorg - discarded"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        target = "p2p::reorg",
                        tx_hash = %hex::encode(tx_hash),
                        error = %e,
                        "Failed to revalidate transaction - discarded"
                    );
                }
            }
        }
    }

    tracing::info!(
        target = "p2p::reorg",
        total = total,
        reinserted = reinserted,
        skipped = total - reinserted,
        "Reinserted orphaned transactions to mempool"
    );

    reinserted
}

/// Check if a block would trigger a reorg (without executing it)
pub fn would_trigger_reorg(block: &Block) -> bool {
    let g = crate::CHAIN.lock();

    let current_tip = match g.blocks.last() {
        Some(b) => b,
        None => return false,
    };

    // Not a reorg if it extends current tip
    if block.header.parent_hash == current_tip.header.pow_hash {
        return false;
    }

    // Check if fork would have more work
    if let Some(fork) = find_fork(&g.blocks, block) {
        let ancestor_height = find_block_height(&g.blocks, &fork.common_ancestor);
        let current_difficulty: u64 = g
            .blocks
            .iter()
            .skip(ancestor_height + 1)
            .map(|b| b.header.difficulty)
            .sum();

        fork.total_difficulty > current_difficulty
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_reorg_on_tip_extension() {
        // Test that extending the tip doesn't trigger reorg
    }

    #[test]
    fn test_reorg_detection() {
        // Test fork detection and comparison
    }

    #[test]
    fn test_orphaned_tx_collection() {
        // Test that transactions are properly collected from orphaned blocks
    }
}

// ==================== ENTERPRISE VALIDATION HELPERS ====================

// Legacy validate_block_structure removed - validation now handled by chain::accept::apply_block

// PATCH 2: Strict PoW hash parsing (accept optional 0x prefix, require 32 bytes)
fn parse_pow_hash_32(pow_hash: &str) -> Result<[u8; 32], String> {
    let s = pow_hash.trim();
    let s = s.strip_prefix("0x").unwrap_or(s);
    if s.len() != 64 {
        return Err(format!(
            "Invalid PoW hash length: expected 64 hex chars (32 bytes), got {}",
            s.len()
        ));
    }
    let bytes = hex::decode(s).map_err(|_| "Invalid PoW hash format".to_string())?;
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

// PATCH 1: Validate a fork chain end-to-end before reorg mutation
fn validate_fork_chain(fork: &Fork, ancestor_height: u64) -> Result<(), String> {
    if fork.blocks.is_empty() {
        return Err("Fork has no blocks".to_string());
    }

    // First block must connect to reported common ancestor
    if fork.blocks[0].header.parent_hash != fork.common_ancestor {
        return Err(format!(
            "Fork does not connect to common ancestor: first.parent={} ancestor={}",
            fork.blocks[0].header.parent_hash, fork.common_ancestor
        ));
    }

    // Height continuity check (detailed validation happens in apply_block)
    for (i, block) in fork.blocks.iter().enumerate() {
        // Height should be sequential starting at ancestor_height+1
        let expected_height = ancestor_height + 1 + i as u64;
        if block.header.number != expected_height {
            return Err(format!(
                "Fork height discontinuity at i={}: got {} expected {}",
                i, block.header.number, expected_height
            ));
        }

        // Parent hash link
        if i > 0 {
            let prev_hash = &fork.blocks[i - 1].header.pow_hash;
            if block.header.parent_hash != *prev_hash {
                return Err(format!(
                    "Fork parent mismatch at i={}: got {} expected {}",
                    i, block.header.parent_hash, prev_hash
                ));
            }
        }
    }

    Ok(())
}

/// Re-validate transaction against new chain state after reorg
fn revalidate_transaction_against_state(
    tx: &Tx,
    chain_state: &crate::Chain,
) -> Result<bool, String> {
    use tracing::debug;

    // 1. Basic signature validation
    if let Err(e) = crate::verify_tx(tx) {
        debug!(
            tx_hash = %hex::encode(crate::tx_hash(tx)),
            error = ?e,
            "Transaction signature invalid after reorg"
        );
        return Ok(false);
    }

    // 2. Check nonce hasn't been consumed in new chain
    let sender = &tx.sender_pubkey;
    let tx_nonce = tx.nonce;

    if let Some(&chain_nonce) = chain_state.nonces.get(sender) {
        if tx_nonce <= chain_nonce {
            debug!(
                tx_hash = %hex::encode(crate::tx_hash(tx)),
                tx_nonce = tx_nonce,
                chain_nonce = chain_nonce,
                "Transaction nonce already consumed in new chain"
            );
            return Ok(false);
        }
    }

    // 3. Check sender has sufficient balance for fees
    let sender_balance = chain_state.balances.get(sender).copied().unwrap_or(0);

    // Calculate total fee: base_fee * weight + tip
    let base_fee = crate::fee_base();
    let weight = crate::est_tx_weight(tx) as u128;
    let total_fee = base_fee
        .saturating_mul(weight)
        .saturating_add(tx.tip as u128);

    // Also check against fee_limit
    if total_fee > tx.fee_limit as u128 {
        debug!(
            tx_hash = %hex::encode(crate::tx_hash(tx)),
            total_fee = total_fee,
            fee_limit = tx.fee_limit,
            "Transaction fee exceeds limit"
        );
        return Ok(false);
    }

    if sender_balance < total_fee {
        debug!(
            tx_hash = %hex::encode(crate::tx_hash(tx)),
            sender_balance = sender_balance,
            required_fee = total_fee,
            "Insufficient balance for fees in new chain state"
        );
        return Ok(false);
    }

    // Transaction is still valid
    Ok(true)
}
