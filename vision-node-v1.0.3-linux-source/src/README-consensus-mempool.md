# Vision: consensus + mempool hardening (demo helpers)

This drop adds small, demo-grade modules:

- `src/types.rs` — `leading_zero_bits()`, `work_from_hash()`
- `src/consensus.rs` — `ConsensusParams`, `meets_target()`, PoW/time validation helpers
- `src/mempool.rs` — `MempoolCfg`, `admit_tx_with_policy()`
- `src/p2p.rs` — tiny per-peer throttle helper

## Wiring (minimal)

1) **Cargo.toml** — add module paths (no extra deps beyond what you have).
   You already include `once_cell`, `parking_lot`, `serde`, `hex`, `blake3`.

2) **In `src/main.rs`** add at top:
```rust
mod types;
mod consensus;
mod mempool;
mod p2p;
use consensus::{ConsensusParams, meets_target};
use mempool::{MempoolCfg, admit_tx_with_policy};
use types::leading_zero_bits;
```

3) **Store params** (global is fine for now):
```rust
static CONSENSUS: once_cell::sync::Lazy<ConsensusParams> = once_cell::sync::Lazy::new(|| ConsensusParams { target_bits: 16, ..Default::default() });
static MEMPOOL_CFG: once_cell::sync::Lazy<MempoolCfg> = once_cell::sync::Lazy::new(|| MempoolCfg { min_tip: 0, ..Default::default() });
```

4) **Use in `submit_tx`** (before pushing to mempool):
```rust
// estimate mempool bytes and per-sender count (naive)
let mem_len = g.mempool.len();
let mem_bytes = mem_len * 512; // cheap estimate
let mut per_sender = std::collections::BTreeMap::new();
for t in g.mempool.iter() {
    *per_sender.entry(acct_key(&t.sender_pubkey)).or_insert(0) += 1;
}
admit_tx_with_policy(&tx, mem_len, mem_bytes, &per_sender, &MEMPOOL_CFG)?;
// if Ok, push_back(tx.clone())
```

5) **Use in mining check**: replace your `meets_difficulty(&h)` with
```rust
if meets_target(&h, CONSENSUS.target_bits) { /* found */ }
```

6) **On receiving a block** add time sanity:
```rust
use consensus::validate_time_rules;
let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
let recent_ts: Vec<u64> = g.blocks.iter().rev().take(CONSENSUS.median_window).map(|b| b.header.timestamp).collect();
validate_time_rules(block.header.timestamp, g.blocks.last().unwrap().header.timestamp, &recent_ts, now, &CONSENSUS)?;
```

This keeps your API and storage intact while adding basic safety nets.
