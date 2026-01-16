// Block submission handler for VisionX PoW

use crate::pow::visionx::VisionXParams;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

// Legacy struct for backward compatibility with old submit_block API
#[derive(Clone, Debug)]
pub struct MineableBlock {
    pub header: MineableBlockHeader,
    pub transactions: Vec<u8>, // Dummy field
}

#[derive(Clone, Debug)]
pub struct MineableBlockHeader {
    pub height: u64,
    pub timestamp: u64,
    pub difficulty: u64,
    pub nonce: u64,
    pub transactions_root: [u8; 32],
}

#[derive(Debug, Clone)]
pub enum SubmitResult {
    Accepted { height: u64, hash: [u8; 32] },
    Rejected { reason: String },
    Duplicate,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MiningStats {
    pub blocks_found: u64,
    pub blocks_accepted: u64,
    pub blocks_rejected: u64,
    pub last_block_time: Option<u64>,
    pub last_block_height: Option<u64>,
    pub total_rewards: u64,
    pub recent_blocks: Vec<RecentBlock>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecentBlock {
    pub height: u64,
    pub timestamp: u64,
    pub reward: u64,
    pub hash: [u8; 32],
}

pub struct BlockSubmitter {
    stats: Arc<Mutex<MiningStatsInner>>,
    params: VisionXParams,
    found_block_callback:
        Option<tokio::sync::mpsc::UnboundedSender<crate::consensus_pow::FoundPowBlock>>,
}

struct MiningStatsInner {
    blocks_found: u64,
    blocks_accepted: u64,
    blocks_rejected: u64,
    last_block_time: Option<u64>,
    last_block_height: Option<u64>,
    total_rewards: u64,
    recent_blocks: VecDeque<RecentBlock>,
    seen_hashes: VecDeque<[u8; 32]>,
}

impl BlockSubmitter {
    pub fn new(
        params: VisionXParams,
        found_block_callback: Option<
            tokio::sync::mpsc::UnboundedSender<crate::consensus_pow::FoundPowBlock>,
        >,
    ) -> Self {
        Self {
            stats: Arc::new(Mutex::new(MiningStatsInner {
                blocks_found: 0,
                blocks_accepted: 0,
                blocks_rejected: 0,
                last_block_time: None,
                last_block_height: None,
                total_rewards: 0,
                recent_blocks: VecDeque::new(),
                seen_hashes: VecDeque::new(),
            })),
            params,
            found_block_callback,
        }
    }

    /// Submit block from FoundPowBlock (new format with full header)
    pub fn submit_block_from_found(
        &self,
        found: &crate::consensus_pow::FoundPowBlock,
        target: [u8; 32],
        _epoch_seed: [u8; 32],
    ) -> SubmitResult {
        let mut stats = self.stats.lock().unwrap();
        stats.blocks_found += 1;

        let digest = found.digest; // U256 is already [u8; 32]

        if stats.seen_hashes.contains(&digest) {
            return SubmitResult::Duplicate;
        }

        // Use proper U256 comparison from pow module
        let meets_target = crate::pow::u256_leq(&digest, &target);

        if !meets_target {
            stats.blocks_rejected += 1;
            return SubmitResult::Rejected {
                reason: format!(
                    "Digest does not meet target (digest > target)\n  Digest: {}\n  Target: {}",
                    hex::encode(digest),
                    hex::encode(target)
                ),
            };
        }

        stats.blocks_accepted += 1;
        stats.last_block_time = Some(found.header.timestamp);
        stats.last_block_height = Some(found.header.number);

        let reward = 50_000_000;
        stats.total_rewards += reward;

        stats.recent_blocks.push_back(RecentBlock {
            height: found.header.number,
            timestamp: found.header.timestamp,
            reward,
            hash: digest,
        });

        if stats.recent_blocks.len() > 100 {
            stats.recent_blocks.pop_front();
        }

        stats.seen_hashes.push_back(digest);
        if stats.seen_hashes.len() > 100 {
            stats.seen_hashes.pop_front();
        }

        if let Some(ref sender) = self.found_block_callback {
            tracing::info!(
                height = found.header.number,
                digest8 = format!(
                    "{:02x}{:02x}{:02x}{:02x}",
                    digest[0], digest[1], digest[2], digest[3]
                ),
                "[MINER-CHANNEL-SEND] Sending FoundPowBlock to integration task"
            );
            let send_result = sender.send(found.clone());
            if send_result.is_err() {
                tracing::error!(
                    height = found.header.number,
                    "[MINER-CHANNEL-SEND] FAILED - channel closed!"
                );
            }
        } else {
            tracing::warn!(
                height = found.header.number,
                "[MINER-CHANNEL-SEND] SKIPPED - no callback configured!"
            );
        }

        SubmitResult::Accepted {
            height: found.header.number,
            hash: digest,
        }
    }

    // Legacy submit_block method - DEPRECATED, use submit_block_from_found instead
    #[allow(dead_code)]
    pub fn submit_block(
        &self,
        block: MineableBlock,
        digest: [u8; 32],
        target: [u8; 32],
        _epoch_seed: [u8; 32],
    ) -> SubmitResult {
        let mut stats = self.stats.lock().unwrap();
        stats.blocks_found += 1;

        if stats.seen_hashes.contains(&digest) {
            return SubmitResult::Duplicate;
        }

        // Use proper U256 comparison from pow module
        let meets_target = crate::pow::u256_leq(&digest, &target);

        if !meets_target {
            stats.blocks_rejected += 1;
            return SubmitResult::Rejected {
                reason: format!(
                    "Digest does not meet target (digest > target)\n  Digest: {}\n  Target: {}",
                    hex::encode(digest),
                    hex::encode(target)
                ),
            };
        }

        stats.blocks_accepted += 1;
        stats.last_block_time = Some(block.header.timestamp);
        stats.last_block_height = Some(block.header.height);

        let reward = 50_000_000;
        stats.total_rewards += reward;

        stats.recent_blocks.push_back(RecentBlock {
            height: block.header.height,
            timestamp: block.header.timestamp,
            reward,
            hash: digest,
        });

        if stats.recent_blocks.len() > 100 {
            stats.recent_blocks.pop_front();
        }

        stats.seen_hashes.push_back(digest);
        if stats.seen_hashes.len() > 100 {
            stats.seen_hashes.pop_front();
        }

        // Note: Old callback code removed - use submit_block_from_found instead
        SubmitResult::Accepted {
            height: block.header.height,
            hash: digest,
        }
    }

    pub fn stats(&self) -> MiningStats {
        let stats = self.stats.lock().unwrap();
        MiningStats {
            blocks_found: stats.blocks_found,
            blocks_accepted: stats.blocks_accepted,
            blocks_rejected: stats.blocks_rejected,
            last_block_time: stats.last_block_time,
            last_block_height: stats.last_block_height,
            total_rewards: stats.total_rewards,
            recent_blocks: stats.recent_blocks.iter().cloned().collect(),
        }
    }
}
