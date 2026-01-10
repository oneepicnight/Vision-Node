//! Block acceptance module - single-track validation for all blocks
//!
//! All blocks (from P2P, local mining, sync, etc.) MUST go through this module.
//! This ensures consistent VisionX PoW validation with consensus params.

use crate::*;
use crate::metrics::{PROM_VISION_ATOMIC_FAILURES, PROM_VISION_ATOMIC_TXS};
use std::collections::BTreeMap;
use tracing::info;

/// Apply and validate a block through the unified acceptance pipeline.
///
/// This is the ONLY function that should add blocks to the chain.
/// All paths (P2P, local mining, sync) must use this.
/// 
/// # Parameters
/// - `g`: Mutable chain state
/// - `blk`: Block to validate and apply
/// - `source_peer`: Optional peer address/ID that sent this block (for orphan tracking)
pub fn apply_block(g: &mut Chain, blk: &Block, source_peer: Option<&str>) -> Result<(), String> {
    let _span = tracing::info_span!("block_validation", block_hash = %blk.header.pow_hash, height = blk.header.number).entered();
    let validation_start = std::time::Instant::now();

    let blk_hash_canon = crate::canon_hash(&blk.header.pow_hash);
    let parent_hash_canon = crate::canon_hash(&blk.header.parent_hash);

    // dedupe
    if g.seen_blocks.contains(&blk_hash_canon) {
        return Err(format!("duplicate block: {}", blk.header.pow_hash));
    }

    // Enforce miner identity presence (required for reward distribution)
    if blk.header.miner.is_empty() {
        return Err(format!(
            "block {} rejected: miner field is empty (required for reward distribution)",
            blk.header.number
        ));
    }

    // ⚠️ FORK-CRITICAL: Verify PoW using VisionX with hardcoded consensus params
    // ALL nodes must use identical params or chain will fork!
    // This MUST match the params used by miners when computing digest.
    let params = crate::consensus_pow::consensus_params_to_visionx(&crate::consensus_pow::VISIONX_CONSENSUS_PARAMS);
    let target = pow::u256_from_difficulty(blk.header.difficulty);

    tracing::info!(
        target = "chain::pow",
        block_height = blk.header.number,
        block_hash = %blk.header.pow_hash,
        parent_hash = %blk.header.parent_hash,
        timestamp = blk.header.timestamp,
        difficulty = blk.header.difficulty,
        nonce = blk.header.nonce,
        tx_root = %blk.header.tx_root,
        state_root = %blk.header.state_root,
        receipts_root = %blk.header.receipts_root,
        base_fee_per_gas = blk.header.base_fee_per_gas,
        "CHAIN-POW: validating block header inputs"
    );

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

    // Compute epoch_seed using DETERMINISTIC, FORK-INDEPENDENT derivation
    // This ensures all nodes use the SAME dataset for a given epoch,
    // regardless of which blocks they've mined or received.
    // 
    // CRITICAL: DO NOT use epoch boundary block hash - that creates forks!
    // When multiple miners find different blocks at height 32, they'd use
    // different seeds and create mutually-unverifiable chains.
    let genesis_pow_hash = {
        let mut hash = [0u8; 32];
        // Genesis block uses all zeros as pow_hash
        hash
    };
    let chain_id = crate::vision_constants::VISION_NETWORK_ID;
    let epoch_seed = crate::consensus_pow::visionx_epoch_seed(chain_id, genesis_pow_hash, epoch);

    // Create PoW message (stable binary encoding with strict validation)
    // CRITICAL: Use nonce=0 in the header when building the message
    // The miner builds the job with nonce=0 in the header, then varies nonce separately
    // The nonce will be passed separately to visionx_hash()
    let mut header_for_msg = blk.header.clone();
    header_for_msg.nonce = 0;  // Zero out nonce for message encoding

    // DIAGNOSTIC: Log what we're about to validate
    eprintln!("[ACCEPT-POW-MSG] Block #{} validation:", blk.header.number);
    eprintln!("  parent_hash: {}", header_for_msg.parent_hash);
    eprintln!("  number: {}", header_for_msg.number);
    eprintln!("  timestamp: {}", header_for_msg.timestamp);
    eprintln!("  difficulty: {}", header_for_msg.difficulty);
    eprintln!("  nonce (actual): {} (zeroed in message)", blk.header.nonce);
    eprintln!("  tx_root: {}", header_for_msg.tx_root);
    eprintln!("  block.pow_hash: {}", blk.header.pow_hash);

    let msg = crate::consensus_pow::pow_message_bytes(&header_for_msg).map_err(|e| {
        eprintln!("[ACCEPT-POW-MSG] pow_message_bytes failed: {}", e);
        format!(
            "Block validation failed: {} [from peer providing block {}]",
            e, blk.header.pow_hash
        )
    })?;

    eprintln!("  pow_msg length: {} bytes", msg.len());
    eprintln!("  pow_msg (first 64 bytes): {}", hex::encode(&msg[..64.min(msg.len())]));
    eprintln!("  epoch: {}, epoch_seed (first 4 bytes): {:02x}{:02x}{:02x}{:02x}",
        epoch, epoch_seed[0], epoch_seed[1], epoch_seed[2], epoch_seed[3]);

    tracing::info!("[POW-PARAMS] {}", params.fingerprint());
    tracing::info!(
        "[POW-SEED] epoch={} seed_prefix={:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        epoch,
        epoch_seed[0], epoch_seed[1], epoch_seed[2], epoch_seed[3],
        epoch_seed[4], epoch_seed[5], epoch_seed[6], epoch_seed[7]
    );

    // Compute digest (MUST be identical to what miner computed)
    // Use epoch_seed for dataset lookup, just like the miner does
    let (dataset, mask) =
        pow::visionx::VisionXDataset::get_cached(&params, &epoch_seed, epoch);
    let digest = pow::visionx::visionx_hash(&params, &dataset, mask, &msg, blk.header.nonce);

    eprintln!("  computed_digest: {}", hex::encode(digest));

    // Verify digest meets target and matches block's pow_hash
    let digest_hex = hex::encode(digest);
    let block_pow_hash = if blk.header.pow_hash.starts_with("0x") {
        &blk.header.pow_hash[2..]
    } else {
        &blk.header.pow_hash
    };

    // CRITICAL DIAGNOSTIC: Log all PoW components for mismatch debugging
    let msg_prefix_hex = hex::encode(&msg[0..16.min(msg.len())]);
    tracing::info!(
        "[POW-CHECK] height={} computed_pow={} header_pow={} msg_len={} msg_prefix={}",
        blk.header.number,
        digest_hex,
        block_pow_hash,
        msg.len(),
        msg_prefix_hex
    );

    if !pow::u256_leq(&digest, &target) {
        tracing::warn!(
            "[CHAIN-REJECT] bad_pow: height={} hash={} reason=digest_exceeds_target computed_digest={} target_difficulty={}",
            blk.header.number,
            blk.header.pow_hash,
            digest_hex,
            blk.header.difficulty
        );
        return Err(
            errors::NodeError::Consensus(errors::ConsensusError::InvalidPoW(format!(
                "block {} digest {} does not meet target difficulty {}",
                blk.header.number, digest_hex, blk.header.difficulty
            )))
            .to_string(),
        );
    }

    if digest_hex != block_pow_hash {
        tracing::warn!(
            "[CHAIN-REJECT] bad_pow: height={} hash={} reason=pow_hash_mismatch computed={} block_has={}",
            blk.header.number,
            blk.header.pow_hash,
            digest_hex,
            block_pow_hash
        );
        return Err(
            errors::NodeError::Consensus(errors::ConsensusError::InvalidPoW(format!(
                "block {} pow_hash mismatch: computed {}, block has {}",
                blk.header.number, digest_hex, block_pow_hash
            )))
            .to_string(),
        );
    }

    tracing::info!(
        block_height = blk.header.number,
        block_hash = %blk.header.pow_hash,
        "✅ POW ok → attempting insert"
    );

    // Insert into side_blocks (we'll decide whether to reorg)
    g.side_blocks
        .insert(blk_hash_canon.clone(), blk.clone());
    PROM_VISION_SIDE_BLOCKS.set(g.side_blocks.len() as i64);

    // compute cumulative work for this block
    let parent_cum = g
        .cumulative_work
        .get(&parent_hash_canon)
        .cloned()
        .unwrap_or(0);
    let my_cum = parent_cum.saturating_add(block_work(blk.header.difficulty));
    g.cumulative_work
        .insert(blk_hash_canon.clone(), my_cum);

    // 🎯 INSERT_RESULT: Log immediately after insert to diagnose fork issues
    let old_tip_height = g.current_height();
    let old_tip_hash = g.blocks.last().map(|b| b.header.pow_hash.clone()).unwrap_or_default();
    let old_tip_work = g.cumulative_work.get(&crate::canon_hash(&old_tip_hash)).cloned().unwrap_or(0);
    
    tracing::info!(
        inserted_height = blk.header.number,
        inserted_hash = %blk.header.pow_hash,
        inserted_work = my_cum,
        old_tip_height = old_tip_height,
        old_tip_hash = %old_tip_hash,
        old_tip_work = old_tip_work,
        became_canonical = "checking...",
        "[INSERT_RESULT] Block inserted into side_blocks, checking if it becomes canonical"
    );

    if !g.cumulative_work.contains_key(&parent_hash_canon)
        && !g.side_blocks.contains_key(&parent_hash_canon)
        && !g
            .blocks
            .iter()
            .any(|b| crate::canon_hash(&b.header.pow_hash) == parent_hash_canon)
    {
        // Store orphan block for later processing
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let peer_info = source_peer.unwrap_or("local_miner").to_string();
        let orphan_count = g.orphan_pool.values().map(|v| v.len()).sum::<usize>();
        
        // Enforce max orphan pool size (prevent DoS)
        const MAX_ORPHANS: usize = 500;
        if orphan_count >= MAX_ORPHANS {
            tracing::warn!(
                orphan_pool_size = orphan_count,
                "[ORPHAN-POOL] max size reached, rejecting orphan"
            );
            return Err(format!("orphan pool full ({}), rejecting block", MAX_ORPHANS));
        }
        
        // Convert source_peer to String for storage
        let peer_string = source_peer.unwrap_or("unknown").to_string();
        
        // Store in orphan pool
        g.orphan_pool
            .entry(parent_hash_canon.clone())
            .or_insert_with(Vec::new)
            .push((blk.clone(), now, peer_string.clone()));
        
        g.orphan_by_hash
            .insert(blk_hash_canon.clone(), parent_hash_canon.clone());
        
        let new_orphan_count = orphan_count + 1;
        
        // Update Prometheus metrics
        crate::PROM_P2P_ORPHANS.set(new_orphan_count as i64);
        crate::PROM_P2P_ORPHANS_INSERTED.inc();
        
        tracing::warn!(
            height = blk.header.number,
            block_hash = %blk.header.pow_hash,
            parent_hash = %blk.header.parent_hash,
            source_peer = %peer_string,
            orphan_pool_size = new_orphan_count,
            "[CHAIN-REJECT] parent_missing: stored in orphan pool"
        );
        
        // Spawn async task to fetch parent
        let parent_for_fetch = parent_hash_canon.clone();
        let peer_for_fetch = peer_string.clone();
        let orphan_height = blk.header.number;
        tokio::spawn(async move {
            crate::chain::parent_fetch::fetch_parent_for_orphan(parent_for_fetch, orphan_height, peer_for_fetch).await;
        });
        
        tracing::info!(
            block_height = blk.header.number,
            block_hash = %blk.header.pow_hash,
            parent_hash = %blk.header.parent_hash,
            orphan_pool_size = new_orphan_count,
            source_peer = %peer_string,
            "📦 orphaned (parent missing) - requesting parent from peer"
        );
        return Ok(());
    }

    // find heaviest tip among current main tip and CONNECTED side blocks only
    // BUG FIX: Don't consider orphaned/disconnected blocks for heaviest tip selection
    let mut heaviest_hash = crate::canon_hash(&g.blocks.last().unwrap().header.pow_hash);
    let mut heaviest_work = *g.cumulative_work.get(&heaviest_hash).unwrap_or(&0);
    
    // Only consider side blocks that have a connected chain back to main
    for (hsh, w) in g.cumulative_work.iter() {
        if *w > heaviest_work {
            // Verify this block is actually reachable (has connected ancestry)
            // Check if it's in side_blocks (it must be if it has cumulative_work)
            if let Some(side_blk) = g.side_blocks.get(hsh) {
                // Walk back to verify ancestry connects to main chain
                let mut cursor = crate::canon_hash(&side_blk.header.parent_hash);
                let mut is_connected = false;
                let mut visited = std::collections::HashSet::new();
                
                // Check if parent is in main chain OR can reach main chain via side blocks
                while !visited.contains(&cursor) {
                    visited.insert(cursor.clone());
                    
                    // Check if we've reached main chain
                    if g.blocks.iter().any(|b| crate::canon_hash(&b.header.pow_hash) == cursor) {
                        is_connected = true;
                        break;
                    }
                    
                    // Check if parent is in side blocks
                    if let Some(parent_blk) = g.side_blocks.get(&cursor) {
                        cursor = crate::canon_hash(&parent_blk.header.parent_hash);
                    } else {
                        // Parent not found anywhere - this is an orphan/disconnected chain
                        tracing::debug!(
                            candidate_hash = %hsh,
                            candidate_weight = w,
                            missing_parent = %cursor,
                            "[HEAVIEST-TIP] Skipping disconnected candidate (missing parent in chain)"
                        );
                        break;
                    }
                    
                    // Safety: prevent infinite loops
                    if visited.len() > 10000 {
                        tracing::warn!(
                            candidate_hash = %hsh,
                            "[HEAVIEST-TIP] Ancestry check hit depth limit - treating as disconnected"
                        );
                        break;
                    }
                }
                
                if is_connected {
                    heaviest_work = *w;
                    heaviest_hash = hsh.clone();
                    tracing::debug!(
                        new_heaviest = %hsh,
                        weight = w,
                        "[HEAVIEST-TIP] Updated to connected heavier chain"
                    );
                }
            }
        }
    }

    // if heaviest is current tip, we're done (no reorg)
    let current_tip_hash = crate::canon_hash(&g.blocks.last().unwrap().header.pow_hash);
    if heaviest_hash == current_tip_hash {
        // if block extends current tip, append it
        if parent_hash_canon == current_tip_hash {
            // execute and append
            let mut balances = g.balances.clone();
            let mut nonces = g.nonces.clone();
            let mut gm = g.gamemaster.clone();
            let miner_key = acct_key(&blk.header.miner);
            balances.entry(miner_key.clone()).or_insert(0);
            nonces.entry(miner_key.clone()).or_insert(0);
            let mut exec_results: BTreeMap<String, Result<(), String>> = BTreeMap::new();
            
            // Calculate total transaction fees
            let mut tx_fees_total = 0u128;
            for tx in &blk.txs {
                let h = hex::encode(tx_hash(tx));
                let res = execute_tx_with_nonce_and_fees(
                    tx,
                    &mut balances,
                    &mut nonces,
                    &miner_key,
                    &mut gm,
                );
                exec_results.insert(h, res);
                
                // Calculate fee for this transaction
                if tx.module == "cash" && tx.method == "transfer" {
                    let (fee_and_tip, _miner_reward) = fee_for_transfer(1, tx.tip);
                    tx_fees_total = tx_fees_total.saturating_add(fee_and_tip);
                }
            }
            
            // Apply tokenomics: emission, halving, fee distribution, miner rewards
            // Use the actual miner identity embedded in the block header
            let block_miner_addr = &blk.header.miner;
            let mev_revenue = 0u128; // TODO: track from bundles if any
            
            // Temporarily update chain state so apply_tokenomics can modify balances
            g.balances = balances.clone();
            g.nonces = nonces.clone();
            g.gamemaster = gm.clone();
            
            let (miner_reward, fees_distributed, treasury_total) = crate::apply_tokenomics(
                g,
                blk.header.number,
                block_miner_addr,
                tx_fees_total,
                mev_revenue,
            );
            
            // Get updated state after tokenomics
            balances = g.balances.clone();
            nonces = g.nonces.clone();
            gm = g.gamemaster.clone();
            
            tracing::info!(
                "💰 Reward applied → miner={} block={} reward={} fees={} treasury={}",
                block_miner_addr,
                blk.header.number,
                miner_reward,
                tx_fees_total,
                treasury_total
            );
            
            let new_state_root = compute_state_root(&balances, &gm);
            tracing::info!(
                block_height = blk.header.number,
                block_hash = %blk.header.pow_hash,
                received_state_root = %blk.header.state_root,
                computed_state_root = %new_state_root,
                "CHAIN-ACCEPT: State root validation"
            );
            if new_state_root != blk.header.state_root {
                tracing::error!(
                    block_height = blk.header.number,
                    block_hash = %blk.header.pow_hash,
                    received = %blk.header.state_root,
                    computed = %new_state_root,
                    "❌ rejected (state_root mismatch)"
                );
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
            tracing::info!(
                block_height = blk.header.number,
                block_hash = %blk.header.pow_hash,
                tx_count = blk.txs.len(),
                received_tx_root = %blk.header.tx_root,
                computed_tx_root = %tx_root,
                "CHAIN-ACCEPT: Tx root validation"
            );
            if tx_root != blk.header.tx_root {
                tracing::error!(
                    block_height = blk.header.number,
                    block_hash = %blk.header.pow_hash,
                    received = %blk.header.tx_root,
                    computed = %tx_root,
                    "❌ rejected (tx_root mismatch)"
                );
                return Err("tx_root mismatch".into());
            }
            // Validate receipts_root deterministically from execution outcomes
            let receipts_root = receipts_root_deterministic(&blk.txs, &exec_results);
            if receipts_root != blk.header.receipts_root {
                tracing::error!(
                    block_height = blk.header.number,
                    block_hash = %blk.header.pow_hash,
                    "❌ rejected (receipts_root mismatch)"
                );
                return Err("receipts_root mismatch".into());
            }
            // Accept with atomic state update
            // Phase 3: Use atomic transaction for state persistence

            // Phase 4: Trace atomic transaction performance
            let atomic_span =
                tracing::info_span!("atomic_state_update", accounts = balances.len()).entered();
            let atomic_start = std::time::Instant::now();
            let atomic_result: Result<(), ()> = {
                // Fallback: non-atomic state update
                persist_state(&g.db, &balances, &nonces, &gm);
                Ok(())
            };
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

            // Commit state and receipts (non-atomic fallback path)
            g.balances = balances;
            g.nonces = nonces;
            g.gamemaster = gm;

            tracing::info!(
                block_height = blk.header.number,
                block_hash = %blk.header.pow_hash,
                "✅ inserted"
            );

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
            // update last-seen block weight metric
            PROM_VISION_BLOCK_WEIGHT_LAST.set(blk.weight as i64);
            let _ = g.db.flush();

            // Track height change before accepting reorg block
            let old_height = g.current_height();
            g.blocks.push(blk.clone());
            let new_height = g.current_height();
            g.log_height_change(old_height, new_height, "reorg_accept_block");
            
            // 🎯 INSERT_RESULT: Final status - block became canonical
            let (final_tip_height, final_tip_hash, final_tip_work) = g.canonical_head();
            tracing::info!(
                inserted_height = blk.header.number,
                inserted_hash = %blk.header.pow_hash,
                became_canonical = true,
                new_tip_height = final_tip_height,
                new_tip_hash = %final_tip_hash,
                new_tip_work = final_tip_work,
                "[INSERT_RESULT] ✅ Block became CANONICAL (extends current tip)"
            );
            
            // [DIAGNOSTIC] Log canonical block commit
            tracing::info!(
                "[CHAIN-ACCEPT] committed canonical block height={} hash={}",
                blk.header.number,
                blk.header.pow_hash
            );

            tracing::info!(
                block_height = blk.header.number,
                block_hash = %blk.header.pow_hash,
                "⬆️ head updated to height={}", 
                new_height
            );

            g.seen_blocks.insert(blk_hash_canon.clone());
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
            
            // Process any orphans waiting for this block
            let orphans_processed = process_orphans(g, &blk_hash_canon);
            if orphans_processed > 0 {
                tracing::info!(
                    parent_hash = %blk_hash_canon,
                    orphans_processed = orphans_processed,
                    "[ORPHAN-POOL] processed children of accepted block"
                );
            }
        } else {
            // Block doesn't extend current tip - stays in side_blocks
            let (tip_height, tip_hash, tip_work) = g.canonical_head();
            tracing::info!(
                inserted_height = blk.header.number,
                inserted_hash = %blk.header.pow_hash,
                inserted_work = my_cum,
                became_canonical = false,
                current_tip_height = tip_height,
                current_tip_hash = %tip_hash,
                current_tip_work = tip_work,
                "[INSERT_RESULT] ⚠️ Block stays in SIDE_BLOCKS (doesn't extend tip)"
            );
        }
        return Ok(());
    }

    // Heavier chain found -> perform reorg to heaviest_hash
    info!(heaviest = %heaviest_hash, "reorg: adopting heavier tip");
    PROM_VISION_REORGS.inc();
    
    // MAX_REORG guard with bootstrap mode support
    // During initial sync (low height OR far behind network), allow deeper reorgs
    let reorg_start = std::time::Instant::now();
    
    // Build path from heaviest_hash back to a block in current main chain
    let mut path: Vec<String> = Vec::new();
    let mut cursor = heaviest_hash.clone();
    loop {
        if g
            .blocks
            .iter()
            .any(|b| crate::canon_hash(&b.header.pow_hash) == cursor)
        {
            break;
        }
        path.push(cursor.clone());
        if let Some(b) = g.side_blocks.get(&cursor) {
            cursor = crate::canon_hash(&b.header.parent_hash);
        } else {
            // Missing parent for reorg candidate - this should be rare now with ancestry checking
            // But if it happens, treat it like an orphan: fetch parent and defer reorg
            let missing_parent = cursor.clone();
            let candidate_tip = heaviest_hash.clone();
            
            tracing::warn!(
                candidate_tip = %candidate_tip,
                missing_parent = %missing_parent,
                current_height = g.current_height(),
                "[REORG-DEFERRED] Missing parent for candidate tip - fetching parent chain"
            );
            
            // Spawn async task to fetch the missing parent
            let peer_string = source_peer.unwrap_or("unknown").to_string();
            tokio::spawn(async move {
                crate::chain::parent_fetch::fetch_parent_for_orphan(
                    missing_parent,
                    0, // Unknown height for reorg candidate
                    peer_string
                ).await;
            });
            
            // Don't attempt reorg until parent chain is complete
            return Ok(());
        }
    }
    // cursor is now ancestor hash that exists in main chain (could be genesis)
    let ancestor_hash = cursor.clone();
    // find ancestor index in main chain
    let ancestor_index = g
        .blocks
        .iter()
        .position(|b| crate::canon_hash(&b.header.pow_hash) == ancestor_hash)
        .unwrap();
    let old_tip_index = g.blocks.len().saturating_sub(1);
    let reorg_depth = old_tip_index.saturating_sub(ancestor_index) as u64;
    let current_height = g.current_height();
    
    // Determine if we're in bootstrap/initial sync mode
    // Criteria: low absolute height OR very far behind the incoming chain
    let incoming_height = blk.header.number;
    let behind_by = incoming_height.saturating_sub(current_height);
    let is_bootstrap_mode = current_height < 128 || behind_by > 24;
    
    // Choose reorg limit based on mode
    let max_reorg = if is_bootstrap_mode {
        // During bootstrap, allow much deeper reorgs but only if we can verify ancestry
        tracing::info!(
            current_height = current_height,
            incoming_height = incoming_height,
            behind_by = behind_by,
            reorg_depth = reorg_depth,
            ancestor_height = ancestor_index,
            "[REORG-BOOTSTRAP] Initial sync mode active - allowing deep reorg with ancestry check"
        );
        g.limits.max_reorg_bootstrap
    } else {
        // Normal operation - strict reorg limit for safety
        g.limits.max_reorg
    };

    // Now check the reorg size
    if reorg_depth > max_reorg {
        PROM_VISION_REORG_REJECTED.inc();
        tracing::warn!(
            reorg_depth = reorg_depth,
            max_reorg = max_reorg,
            is_bootstrap = is_bootstrap_mode,
            current_height = current_height,
            incoming_height = incoming_height,
            ancestor_height = ancestor_index,
            "[REORG-REJECTED] Reorg too large"
        );
        return Err(format!(
            "reorg too large: {} > max {} (bootstrap={})",
            reorg_depth,
            max_reorg,
            is_bootstrap_mode
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
        
        // Calculate total transaction fees
        let mut tx_fees_total = 0u128;
        for tx in &b.txs {
            let h = hex::encode(tx_hash(tx));
            let res = execute_tx_with_nonce_and_fees(
                tx,
                &mut balances2,
                &mut nonces2,
                &miner_key,
                &mut gm2,
            );
            if res.is_err() {
                return Err(format!(
                    "replay/apply failed for block {}: {}",
                    b.header.number,
                    res.err().unwrap_or_default()
                ));
            }
            exec_results.insert(h, res);
            
            // Calculate fee for this transaction
            if tx.module == "cash" && tx.method == "transfer" {
                let (fee_and_tip, _miner_reward) = fee_for_transfer(1, tx.tip);
                tx_fees_total = tx_fees_total.saturating_add(fee_and_tip);
            }
        }
        
        // Apply tokenomics for this block during reorg
        // For blocks received from peers, use a default miner address
        let block_miner_addr = &b.header.miner;
        let mev_revenue = 0u128;
        
        // Temporarily update chain state
        g.balances = balances2.clone();
        g.nonces = nonces2.clone();
        g.gamemaster = gm2.clone();
        
        let (miner_reward, fees_distributed, treasury_total) = crate::apply_tokenomics(
            g,
            b.header.number,
            block_miner_addr,
            tx_fees_total,
            mev_revenue,
        );
        
        // Get updated state after tokenomics
        balances2 = g.balances.clone();
        nonces2 = g.nonces.clone();
        gm2 = g.gamemaster.clone();
        
        tracing::info!(
            "💰 Reward applied → miner={} block={} reward={} fees={} treasury={} (reorg)",
            block_miner_addr,
            b.header.number,
            miner_reward,
            tx_fees_total,
            treasury_total
        );
        
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
            let receipts_root = receipts_root_deterministic(&b.txs, &exec_results);
            if receipts_root != b.header.receipts_root {
                return Err("receipts_root mismatch during strict reorg apply".into());
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
        
        // 🎯 INSERT_RESULT: Block applied during reorg
        let (final_tip_height, final_tip_hash, final_tip_work) = g.canonical_head();
        tracing::info!(
            inserted_height = b.header.number,
            inserted_hash = %b.header.pow_hash,
            became_canonical = true,
            new_tip_height = final_tip_height,
            new_tip_hash = %final_tip_hash,
            new_tip_work = final_tip_work,
            "[INSERT_RESULT] ✅ Block became CANONICAL (via reorg)"
        );
        
        // [DIAGNOSTIC] Log canonical block commit
        tracing::info!(
            "[CHAIN-ACCEPT] committed canonical block height={} hash={}",
            b.header.number,
            b.header.pow_hash
        );

        g.seen_blocks.insert(crate::canon_hash(&b.header.pow_hash));
        for tx in &b.txs {
            g.seen_txs.insert(hex::encode(tx_hash(tx)));
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
            .insert(crate::canon_hash(&b.header.pow_hash), prev_cum);
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
            if !g.seen_txs.contains(&th) {
                // push orphaned txs into bulk lane
                g.mempool_bulk.push_back(tx.clone());
                g.mempool_ts.insert(th, now);
            }
        }
    }

    // update side-block metric
    PROM_VISION_SIDE_BLOCKS.set(g.side_blocks.len() as i64);

    // Process orphans after reorg completes
    let new_tip_hash = crate::canon_hash(&g.blocks.last().unwrap().header.pow_hash);
    let orphans_processed = process_orphans(g, &new_tip_hash);
    if orphans_processed > 0 {
        tracing::info!(
            tip_hash = %new_tip_hash,
            orphans_processed = orphans_processed,
            "[ORPHAN-POOL] processed orphans after reorg"
        );
    }

    Ok(())
}

/// Process orphan blocks after their parent has been accepted
/// 
/// When a block is accepted, check if any orphans are waiting for this block
/// as their parent. Attempt to accept them recursively.
pub fn process_orphans(g: &mut Chain, parent_hash: &str) -> usize {
    let mut processed = 0;
    let mut to_process = vec![parent_hash.to_string()];
    
    while let Some(current_parent) = to_process.pop() {
        // Check if any orphans are waiting for this parent
        if let Some(orphans) = g.orphan_pool.remove(&current_parent) {
            tracing::info!(
                parent_hash = %current_parent,
                orphan_count = orphans.len(),
                "[ORPHAN-POOL] processing orphans"
            );
            
            for (orphan_block, received_ts, source_peer) in orphans {
                let orphan_hash = crate::canon_hash(&orphan_block.header.pow_hash);
                
                // Remove from reverse index
                g.orphan_by_hash.remove(&orphan_hash);
                
                // Check TTL (5 minutes)
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                
                if now.saturating_sub(received_ts) > 300 {
                    tracing::warn!(
                        orphan_hash = %orphan_block.header.pow_hash,
                        age_seconds = now.saturating_sub(received_ts),
                        "[ORPHAN-POOL] expired, dropping"
                    );
                    continue;
                }
                
                tracing::info!(
                    orphan_hash = %orphan_block.header.pow_hash,
                    orphan_height = orphan_block.header.number,
                    source_peer = %source_peer,
                    "[ORPHAN-POOL] attempting to accept"
                );
                
                // Try to accept the orphan
                match apply_block(g, &orphan_block, Some(&source_peer)) {
                    Ok(()) => {
                        processed += 1;
                        crate::PROM_P2P_ORPHANS_ADOPTED.inc();
                        crate::PROM_P2P_ORPHANS_RESOLVED.inc();
                        
                        // This orphan might be the parent of other orphans
                        to_process.push(orphan_hash.clone());
                        tracing::info!(
                            orphan_hash = %orphan_block.header.pow_hash,
                            orphan_height = orphan_block.header.number,
                            "[ORPHAN-POOL] ✅ accepted"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            orphan_hash = %orphan_block.header.pow_hash,
                            error = %e,
                            "[ORPHAN-POOL] ❌ rejected"
                        );
                    }
                }
            }
        }
    }
    
    if processed > 0 {
        let remaining = g.orphan_pool.values().map(|v| v.len()).sum::<usize>();
        crate::PROM_P2P_ORPHANS.set(remaining as i64);
        tracing::info!(
            processed_count = processed,
            remaining_orphans = remaining,
            "[ORPHAN-POOL] processing complete"
        );
    }
    
    processed
}

/// Prune old orphans based on TTL
pub fn prune_old_orphans(g: &mut Chain) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let mut to_remove = Vec::new();
    let mut pruned_count = 0;
    
    for (parent_hash, orphans) in g.orphan_pool.iter_mut() {
        orphans.retain(|(blk, received_ts, _)| {
            let age = now.saturating_sub(*received_ts);
            if age > 300 { // 5 minute TTL
                to_remove.push(crate::canon_hash(&blk.header.pow_hash));
                pruned_count += 1;
                crate::PROM_P2P_ORPHANS_PRUNED.inc();
                crate::PROM_P2P_ORPHANS_RESOLVED.inc();
                false
            } else {
                true
            }
        });
        
        if orphans.is_empty() {
            to_remove.push(parent_hash.clone());
        }
    }
    
    // Remove empty entries and update reverse index
    for parent_hash in &to_remove {
        g.orphan_pool.remove(parent_hash);
        g.orphan_by_hash.remove(parent_hash);
    }
    
    if pruned_count > 0 {
        tracing::info!(
            pruned_count = pruned_count,
            "[ORPHAN-POOL] pruned expired orphans"
        );
    }
}

