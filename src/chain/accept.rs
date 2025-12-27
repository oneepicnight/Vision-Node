//! Block acceptance module - single-track validation for all blocks
//!
//! All blocks (from P2P, local mining, sync, etc.) MUST go through this module.
//! This ensures consistent VisionX PoW validation with consensus params.

use crate::*;
use std::collections::BTreeMap;
use tracing::info;

/// Apply and validate a block through the unified acceptance pipeline.
///
/// This is the ONLY function that should add blocks to the chain.
/// All paths (P2P, local mining, sync) must use this.
pub fn apply_block(g: &mut Chain, blk: &Block) -> Result<(), String> {
    let _span = tracing::info_span!("block_validation", block_hash = %blk.header.pow_hash, height = blk.header.number).entered();
    let validation_start = std::time::Instant::now();

    // dedupe
    if g.seen_blocks.contains(&blk.header.pow_hash) {
        return Err(format!("duplicate block: {}", blk.header.pow_hash));
    }

    // ⚠️ FORK-CRITICAL: Verify PoW using VisionX with hardcoded consensus params
    // ALL nodes must use identical params or chain will fork!
    // This MUST match the params used by miners when computing digest.
    let params = consensus_params_to_visionx(&VISIONX_CONSENSUS_PARAMS);
    let target = pow::u256_from_difficulty(blk.header.difficulty);

    // Get parent hash as bytes32
    // Strict validation: reject blocks with invalid parent_hash
    let parent_hash_str =
        if blk.header.parent_hash.starts_with("0x") || blk.header.parent_hash.starts_with("0X") {
            &blk.header.parent_hash[2..]
        } else {
            &blk.header.parent_hash
        };

    let parent_hash32 = hex::decode(parent_hash_str).map_err(|e| {
        format!(
            "Invalid parent_hash '{}' in block {}: {} - rejecting corrupted block",
            blk.header.parent_hash, blk.header.number, e
        )
    })?;

    if parent_hash32.len() != 32 {
        return Err(format!(
            "Invalid parent_hash '{}' in block {}: decoded to {} bytes, expected 32",
            blk.header.parent_hash,
            blk.header.number,
            parent_hash32.len()
        ));
    }

    let mut parent_hash_array = [0u8; 32];
    parent_hash_array.copy_from_slice(&parent_hash32);

    // Compute epoch
    let epoch = blk.header.number / (params.epoch_blocks as u64);

    // Create PoW message (stable binary encoding with strict validation)
    let msg = pow_message_bytes(&blk.header).map_err(|e| {
        format!(
            "Block validation failed: {} [from peer providing block {}]",
            e, blk.header.pow_hash
        )
    })?;

    // Compute digest (MUST be identical to what miner computed)
    let (dataset, mask) =
        pow::visionx::VisionXDataset::get_cached(&params, &parent_hash_array, epoch);
    let digest = pow::visionx::visionx_hash(&params, &dataset, mask, &msg, blk.header.nonce);

    // Verify digest meets target and matches block's pow_hash
    let digest_hex = hex::encode(digest);
    let block_pow_hash = if blk.header.pow_hash.starts_with("0x") {
        &blk.header.pow_hash[2..]
    } else {
        &blk.header.pow_hash
    };

    if !pow::u256_leq(&digest, &target) {
        return Err(
            errors::NodeError::Consensus(errors::ConsensusError::InvalidPoW(format!(
                "block {} digest {} does not meet target difficulty {}",
                blk.header.number, digest_hex, blk.header.difficulty
            )))
            .to_string(),
        );
    }

    if digest_hex != block_pow_hash {
        return Err(
            errors::NodeError::Consensus(errors::ConsensusError::InvalidPoW(format!(
                "block {} pow_hash mismatch: computed {}, block has {}",
                blk.header.number, digest_hex, block_pow_hash
            )))
            .to_string(),
        );
    }

    // Insert into side_blocks (we'll decide whether to reorg)
    g.side_blocks
        .insert(blk.header.pow_hash.clone(), blk.clone());
    PROM_VISION_SIDE_BLOCKS.set(g.side_blocks.len() as i64);

    // compute cumulative work for this block
    let parent_cum = g
        .cumulative_work
        .get(&blk.header.parent_hash)
        .cloned()
        .unwrap_or(0);
    let my_cum = parent_cum.saturating_add(block_work(blk.header.difficulty));
    g.cumulative_work
        .insert(blk.header.pow_hash.clone(), my_cum);

    // find heaviest tip among current main tip and side blocks
    let mut heaviest_hash = g.blocks.last().unwrap().header.pow_hash.clone();
    let mut heaviest_work = *g.cumulative_work.get(&heaviest_hash).unwrap_or(&0);
    for (hsh, w) in g.cumulative_work.iter() {
        if *w > heaviest_work {
            heaviest_work = *w;
            heaviest_hash = hsh.clone();
        }
    }

    // if heaviest is current tip, we're done (no reorg)
    let current_tip_hash = g.blocks.last().unwrap().header.pow_hash.clone();
    if heaviest_hash == current_tip_hash {
        // if block extends current tip, append it
        if blk.header.parent_hash == current_tip_hash {
            // execute and append
            let mut balances = g.balances.clone();
            let mut nonces = g.nonces.clone();
            let mut gm = g.gamemaster.clone();
            let miner_key = acct_key("miner");
            balances.entry(miner_key.clone()).or_insert(0);
            nonces.entry(miner_key.clone()).or_insert(0);
            let mut exec_results: BTreeMap<String, Result<(), String>> = BTreeMap::new();
            for tx in &blk.txs {
                let h = hex::encode(tx_hash(tx));
                let res = execute_tx_with_nonce_and_fees(
                    tx,
                    &mut balances,
                    &mut nonces,
                    &miner_key,
                    &mut gm,
                    &g.legacy_manager,
                );
                exec_results.insert(h, res);
            }
            let new_state_root = compute_state_root(&balances, &gm);
            if new_state_root != blk.header.state_root {
                return Err(format!(
                    "state_root mismatch: expected {}, got {}",
                    blk.header.state_root, new_state_root
                ));
            }
            let tip = g.blocks.last().unwrap();
            let tx_root = if blk.txs.is_empty() {
                tip.header.tx_root.clone()
            } else {
                tx_root_placeholder(&blk.txs)
            };
            if tx_root != blk.header.tx_root {
                return Err("tx_root mismatch".into());
            }
            // Accept with atomic state update
            // Phase 3: Use atomic transaction for state persistence

            // Phase 4: Trace atomic transaction performance
            let atomic_span =
                tracing::info_span!("atomic_state_update", accounts = balances.len()).entered();
            let atomic_start = std::time::Instant::now();
            let atomic_result = db_transactions::atomic_state_update(&g.db, &balances, &nonces);
            PROM_VISION_ATOMIC_TXS.inc();
            if atomic_result.is_err() {
                PROM_VISION_ATOMIC_FAILURES.inc();
            }
            tracing::info!(
                duration_us = atomic_start.elapsed().as_micros(),
                success = atomic_result.is_ok(),
                "atomic transaction complete"
            );
            drop(atomic_span);

            match atomic_result {
                Ok(_) => {
                    g.balances = balances;
                    g.nonces = nonces;
                    g.gamemaster = gm;

                    // Write receipts
                    for (txh, res) in exec_results.iter() {
                        let r = Receipt {
                            ok: res.is_ok(),
                            error: res.clone().err(),
                            height: blk.header.number,
                            block_hash: blk.header.pow_hash.clone(),
                        };
                        let key = format!("{}{}", RCPT_PREFIX, txh);
                        let _ = g.db.insert(key.as_bytes(), serde_json::to_vec(&r).unwrap());
                    }

                    persist_block_only(&g.db, blk.header.number, blk);
                }
                Err(e) => {
                    tracing::error!(block = %blk.header.pow_hash, error = %e, "atomic state update failed, falling back to traditional persist");
                    // Fallback to traditional persistence
                    g.balances = balances;
                    g.nonces = nonces;
                    g.gamemaster = gm;
                    for (txh, res) in exec_results.iter() {
                        let r = Receipt {
                            ok: res.is_ok(),
                            error: res.clone().err(),
                            height: blk.header.number,
                            block_hash: blk.header.pow_hash.clone(),
                        };
                        let key = format!("{}{}", RCPT_PREFIX, txh);
                        let _ = g.db.insert(key.as_bytes(), serde_json::to_vec(&r).unwrap());
                    }
                    persist_state(&g.db, &g.balances, &g.nonces, &g.gamemaster);
                    persist_block_only(&g.db, blk.header.number, blk);
                }
            }
            // update last-seen block weight metric
            PROM_VISION_BLOCK_WEIGHT_LAST.set(blk.weight as i64);
            let _ = g.db.flush();

            // Track height change before accepting reorg block
            let old_height = g.current_height();
            g.blocks.push(blk.clone());
            let new_height = g.current_height();
            g.log_height_change(old_height, new_height, "reorg_accept_block");

            g.seen_blocks.insert(blk.header.pow_hash.clone());
            info!(block = %blk.header.pow_hash, height = blk.header.number, "accepted block");
            // snapshot periodically (env-driven cadence)
            if g.limits.snapshot_every_blocks > 0
                && (g.blocks.len() as u64).is_multiple_of(g.limits.snapshot_every_blocks)
            {
                persist_snapshot(
                    &g.db,
                    blk.header.number,
                    &g.balances,
                    &g.nonces,
                    &g.gamemaster,
                );
            }
            // update EMA and possibly retarget difficulty (same logic as local mining)
            let observed_interval = if g.blocks.len() >= 2 {
                let len = g.blocks.len();
                let prev_ts = g.blocks[len - 2].header.timestamp as f64;
                let cur_ts = g.blocks[len - 1].header.timestamp as f64;
                (cur_ts - prev_ts).max(1.0)
            } else {
                g.limits.target_block_time as f64
            };
            let alpha = 0.3_f64;
            g.ema_block_time = alpha * observed_interval + (1.0 - alpha) * g.ema_block_time;
            let win = g.limits.retarget_window as usize;
            if g.blocks.len() >= win {
                let target = g.limits.target_block_time as f64;
                let cur = g.difficulty as f64;
                let scale = (target / g.ema_block_time).clamp(0.25, 4.0);
                let max_change = 0.25_f64;
                let mut factor = scale;
                if factor > 1.0 + max_change {
                    factor = 1.0 + max_change;
                }
                if factor < 1.0 - max_change {
                    factor = 1.0 - max_change;
                }
                let next = ((cur * factor).round() as u64).clamp(1, 248);
                g.difficulty = next;
            }
            persist_ema(&g.db, g.ema_block_time);
            persist_difficulty(&g.db, g.difficulty);
            PROM_VISION_BLOCKS_MINED.inc();
            let _txs = blk.txs.len() as u64;
            PROM_VISION_TXS_APPLIED.inc_by(_txs);

            // Phase 4: Record block validation metrics
            tracing::info!(
                duration_ms = validation_start.elapsed().as_millis(),
                "block validated"
            );
        }
        return Ok(());
    }

    // Heavier chain found -> perform reorg to heaviest_hash
    info!(heaviest = %heaviest_hash, "reorg: adopting heavier tip");
    PROM_VISION_REORGS.inc();
    // MAX_REORG guard: don't accept reorganizations that are too large
    let max_reorg = g.limits.max_reorg;
    let old_tip_index = g.blocks.len().saturating_sub(1);
    // compute ancestor index (we compute it below, but we can preliminarily check by walking back from heaviest_hash)
    // For safety, find the ancestor as we already compute path; we'll compute path first then check length.
    let reorg_start = std::time::Instant::now();
    // Build path from heaviest_hash back to a block in current main chain
    let mut path: Vec<String> = Vec::new();
    let mut cursor = heaviest_hash.clone();
    loop {
        if g.blocks.iter().any(|b| b.header.pow_hash == cursor) {
            break;
        }
        path.push(cursor.clone());
        if let Some(b) = g.side_blocks.get(&cursor) {
            cursor = b.header.parent_hash.clone();
        } else {
            // missing parent, cannot adopt
            return Err("missing parent for candidate tip".into());
        }
    }
    // cursor is now ancestor hash that exists in main chain (could be genesis)
    let ancestor_hash = cursor.clone();
    // find ancestor index in main chain
    let ancestor_index = g
        .blocks
        .iter()
        .position(|b| b.header.pow_hash == ancestor_hash)
        .unwrap();

    // Now check the reorg size: old_tip_index - ancestor_index
    if old_tip_index.saturating_sub(ancestor_index) as u64 > max_reorg {
        PROM_VISION_REORG_REJECTED.inc();
        return Err(format!(
            "reorg too large: {} > max {}",
            old_tip_index.saturating_sub(ancestor_index),
            max_reorg
        ));
    }

    // compute orphaned blocks (old main blocks after ancestor)
    let orphaned: Vec<Block> = if ancestor_index < g.blocks.len() - 1 {
        g.blocks.iter().skip(ancestor_index + 1).cloned().collect()
    } else {
        Vec::new()
    };

    // First try fast rollback using per-block undos
    let mut undo_ok = true;
    let old_tip_index = g.blocks.len().saturating_sub(1);

    // Track height before rollback
    let height_before_rollback = g.current_height();

    for h in (ancestor_index + 1..=old_tip_index).rev() {
        let height = g.blocks[h].header.number;
        if let Some(undo) = load_undo(&g.db, height) {
            // apply undo: revert balances
            for (k, vopt) in undo.balances.iter() {
                match vopt {
                    Some(v) => {
                        g.balances.insert(k.clone(), *v);
                    }
                    _ => {
                        g.balances.remove(k);
                    }
                }
            }
            // revert nonces
            for (k, vopt) in undo.nonces.iter() {
                match vopt {
                    Some(v) => {
                        g.nonces.insert(k.clone(), *v);
                    }
                    _ => {
                        g.nonces.remove(k);
                    }
                }
            }
            // revert gamemaster if present
            if let Some(prev_gm_opt) = &undo.gamemaster {
                g.gamemaster = prev_gm_opt.clone();
            }
            // drop the block from in-memory chain
            g.blocks.pop();
        } else {
            // missing undo for this height -> need snapshot fallback
            undo_ok = false;
            break;
        }
    }

    // If undos were not available for a full fast rollback, fallback to snapshot+replay
    if !undo_ok {
        // look for the best snapshot <= ancestor_index
        let mut best_snap: Option<u64> = None;
        for (k, _v) in g.db.scan_prefix("meta:snapshot:".as_bytes()).flatten() {
            if let Ok(s) = String::from_utf8(k.to_vec()) {
                if let Some(hs) = s.strip_prefix("meta:snapshot:") {
                    if let Ok(hv) = hs.parse::<u64>() {
                        if hv <= g.blocks[ancestor_index].header.number {
                            best_snap = Some(best_snap.map_or(hv, |b| b.max(hv)));
                        }
                    }
                }
            }
        }
        if best_snap.is_none() {
            return Err("missing undos and no usable snapshot for rollback".into());
        }
        let snap_h = best_snap.unwrap();
        // load snapshot contents
        let snap_key = format!("meta:snapshot:{}", snap_h);
        let snap_bytes =
            g.db.get(snap_key.as_bytes())
                .unwrap()
                .ok_or_else(|| "failed to read snapshot".to_string())?;
        let snap_val: serde_json::Value =
            serde_json::from_slice(&snap_bytes).map_err(|e| e.to_string())?;
        let balances: BTreeMap<String, u128> =
            serde_json::from_value(snap_val["balances"].clone()).unwrap_or_default();
        let nonces: BTreeMap<String, u64> =
            serde_json::from_value(snap_val["nonces"].clone()).unwrap_or_default();
        let gm: Option<String> = serde_json::from_value(snap_val["gm"].clone()).ok();

        // rebuild in-memory blocks 0..=ancestor_index from persisted DB
        let mut rebuilt: Vec<Block> = Vec::new();
        for h in 0..=g.blocks[ancestor_index].header.number {
            let key = blk_key(h);
            if let Some(bytes) = g.db.get(&key).unwrap() {
                let b: Block = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                rebuilt.push(b);
            } else {
                return Err(format!("missing block {} in DB during snapshot replay", h));
            }
        }
        // apply snapshot state
        let height_before_snapshot_restore = g.current_height();
        g.blocks = rebuilt;
        let height_after_snapshot_restore = g.current_height();
        g.log_height_change(
            height_before_snapshot_restore,
            height_after_snapshot_restore,
            "reorg_snapshot_restore",
        );

        g.balances = balances;
        g.nonces = nonces;
        g.gamemaster = gm;
        persist_state(&g.db, &g.balances, &g.nonces, &g.gamemaster);
    }

    // At this point, memory state is at ancestor. Now apply the new branch blocks in order.
    path.reverse();
    let miner_key = acct_key("miner");
    let mut applied = 0usize;
    for hsh in &path {
        let b = if let Some(bb) = g.side_blocks.get(hsh) {
            bb.clone()
        } else {
            return Err("missing side block during reorg".into());
        };

        // execute txs against current state
        let mut balances2 = g.balances.clone();
        let mut nonces2 = g.nonces.clone();
        let mut gm2 = g.gamemaster.clone();
        balances2.entry(miner_key.clone()).or_insert(0);
        nonces2.entry(miner_key.clone()).or_insert(0);
        let mut exec_results: BTreeMap<String, Result<(), String>> = BTreeMap::new();
        for tx in &b.txs {
            let h = hex::encode(tx_hash(tx));
            let res = execute_tx_with_nonce_and_fees(
                tx,
                &mut balances2,
                &mut nonces2,
                &miner_key,
                &mut gm2,
                &g.legacy_manager,
            );
            if res.is_err() {
                return Err(format!(
                    "replay/apply failed for block {}: {}",
                    b.header.number,
                    res.err().unwrap_or_default()
                ));
            }
            exec_results.insert(h, res);
        }
        // Optionally enforce strict validation
        if reorg_strict() {
            let new_state_root = compute_state_root(&balances2, &gm2);
            if new_state_root != b.header.state_root {
                return Err("state_root mismatch during strict reorg apply".into());
            }
            let tip = g.blocks.last().unwrap();
            let tx_root = if b.txs.is_empty() {
                tip.header.tx_root.clone()
            } else {
                tx_root_placeholder(&b.txs)
            };
            if tx_root != b.header.tx_root {
                return Err("tx_root mismatch during strict reorg apply".into());
            }
        }

        // compute and persist undo for this applied block
        let undo = compute_undo(
            &g.balances,
            &g.nonces,
            &g.gamemaster,
            &balances2,
            &nonces2,
            &gm2,
        );
        persist_undo(&g.db, b.header.number, &undo);

        // accept block
        g.balances = balances2;
        g.nonces = nonces2;
        g.gamemaster = gm2;
        for (txh, res) in exec_results.iter() {
            let r = Receipt {
                ok: res.is_ok(),
                error: res.clone().err(),
                height: b.header.number,
                block_hash: b.header.pow_hash.clone(),
            };
            let key = format!("{}{}", RCPT_PREFIX, txh);
            let _ = g.db.insert(key.as_bytes(), serde_json::to_vec(&r).unwrap());
        }
        persist_state(&g.db, &g.balances, &g.nonces, &g.gamemaster);
        persist_block_only(&g.db, b.header.number, &b);
        // record last block weight in Prometheus (remove legacy atomic)
        PROM_VISION_BLOCK_WEIGHT_LAST.set(b.weight as i64);
        let _ = g.db.flush();

        // Track height change during sync/recovery
        let old_height = g.current_height();
        g.blocks.push(b.clone());
        let new_height = g.current_height();
        g.log_height_change(old_height, new_height, "sync_recovery_block");

        g.seen_blocks.insert(b.header.pow_hash.clone());
        for tx in &b.txs {
            g.seen_txs.insert(hex::encode(tx_hash(tx)), ());
        }
        applied += 1;
    }

    // record reorg length (number of blocks switched)
    let reorg_len = applied as u64;
    PROM_VISION_REORG_LENGTH_TOTAL.inc_by(reorg_len);

    // Log overall height change from reorg (rollback + new blocks)
    let height_after_reorg = g.current_height();
    if height_after_reorg != height_before_rollback {
        tracing::warn!(
            old_height = height_before_rollback,
            new_height = height_after_reorg,
            blocks_removed = height_before_rollback.saturating_sub(ancestor_index as u64 + 1),
            blocks_applied = applied,
            "📊 REORG: Height changed from {} to {} (ancestor at {})",
            height_before_rollback,
            height_after_reorg,
            ancestor_index
        );
    }

    // recompute cumulative_work and ensure side block metric
    g.cumulative_work.clear();
    let mut prev_cum: u128 = 0;
    for b in &g.blocks {
        prev_cum = prev_cum.saturating_add(block_work(b.header.difficulty));
        g.cumulative_work
            .insert(b.header.pow_hash.clone(), prev_cum);
    }

    // snapshot after reorg
    persist_snapshot(
        &g.db,
        g.blocks.last().unwrap().header.number,
        &g.balances,
        &g.nonces,
        &g.gamemaster,
    );

    let dur_ms = reorg_start.elapsed().as_millis() as u64;
    PROM_VISION_REORG_DURATION_MS.set(dur_ms as i64);

    // Re-add orphaned txs to mempool if not present in new chain
    let now = now_ts();
    for b in orphaned {
        for tx in b.txs {
            let th = hex::encode(tx_hash(&tx));
            if !g.seen_txs.contains_key(&th) {
                // push orphaned txs into bulk lane
                g.mempool_bulk.push_back(tx.clone());
                g.mempool_ts.insert(th, now);
            }
        }
    }

    // update side-block metric
    PROM_VISION_SIDE_BLOCKS.set(g.side_blocks.len() as i64);

    Ok(())
}
