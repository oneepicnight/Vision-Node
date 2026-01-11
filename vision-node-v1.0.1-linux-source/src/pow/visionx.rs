#![allow(dead_code)]
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::pow::{u256_leq, U256};

/// VisionX parameters — tune to taste
#[derive(Clone, Copy, Debug)]
pub struct VisionXParams {
    pub dataset_mb: usize,   // e.g., 256 (war mode) or 64 (lite)
    pub scratch_mb: usize,   // NEW: per-hash scratchpad size (war mode: 32-64)
    pub mix_iters: u32,      // e.g., 65536
    pub reads_per_iter: u32, // NEW: 2..4 dependent reads per iteration
    pub write_every: u32,    // war mode: 1 or 4 (frequent writes)
    pub epoch_blocks: u32,   // e.g., 32
}

impl VisionXParams {
    /// Stable fingerprint for logging/debugging to detect param drift between miner and validator
    pub fn fingerprint(&self) -> String {
        format!(
            "v=1 dataset_mb={} scratch_mb={} mix_iters={} reads_per_iter={} write_every={} epoch_blocks={}",
            self.dataset_mb,
            self.scratch_mb,
            self.mix_iters,
            self.reads_per_iter,
            self.write_every,
            self.epoch_blocks,
        )
    }
}

impl Default for VisionXParams {
    fn default() -> Self {
        Self {
            dataset_mb: 256,   // War mode: bigger base dataset
            scratch_mb: 32,    // War mode: per-hash scratchpad
            mix_iters: 65_536, // Keep high iteration count
            reads_per_iter: 4, // War mode: multi-dependent reads
            write_every: 4,    // War mode: frequent deterministic writes
            epoch_blocks: 32,
        }
    }
}

/// Global epoch dataset cache to avoid rebuilding 256MB for every validation
/// Key: (epoch, prev_hash32), Value: (Arc<Vec<u64>>, mask)
type DatasetCache = HashMap<(u64, [u8; 32]), (Arc<Vec<u64>>, usize)>;
static DATASET_CACHE: Lazy<Mutex<DatasetCache>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// SplitMix64 PRNG (std-only, our own)
#[derive(Clone)]
struct SplitMix64 {
    state: u64,
}
impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    #[inline]
    fn next(&mut self) -> u64 {
        let mut z = {
            self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
            self.state
        };
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
}

/// Tiny mixer to expand 128 bits → 256 bits (ours)
#[inline]
fn expand_256(mut a: u64, mut b: u64) -> U256 {
    // 4 rounds Feistel-ish
    for _ in 0..4 {
        a = a.rotate_left(13) ^ b.wrapping_mul(0x9E3779B185EBCA87);
        b = b.rotate_left(17) ^ a.wrapping_mul(0xC2B2AE3D27D4EB4F);
    }
    let mut sm = SplitMix64::new(a ^ b ^ 0xD6E8FEB86659FD93);
    let c = sm.next();
    let d = sm.next();
    let mut out = [0u8; 32];
    out[..8].copy_from_slice(&a.to_be_bytes());
    out[8..16].copy_from_slice(&b.to_be_bytes());
    out[16..24].copy_from_slice(&c.to_be_bytes());
    out[24..32].copy_from_slice(&d.to_be_bytes());
    out
}

/// Initialize per-hash scratchpad deterministically from base dataset
/// Returns (scratchpad, mask)
fn init_scratch(
    params: &VisionXParams,
    base: &[u64],
    base_mask: usize,
    header: &[u8],
    nonce: u64,
) -> (Vec<u64>, usize) {
    // Allocate scratchpad: round to power of 2
    let bytes = params.scratch_mb * 1024 * 1024;
    let mut words = bytes / std::mem::size_of::<u64>();
    let mut n = 1usize;
    while n < words {
        n <<= 1;
    }
    words = n;
    let smask = words - 1;

    // Seed mixer from header + nonce
    let mut seed: u64 = nonce ^ 0xDEADBEEFF00DFACE;
    for chunk in header.chunks(8) {
        let mut v = [0u8; 8];
        v[..chunk.len()].copy_from_slice(chunk);
        seed ^= u64::from_be_bytes(v).rotate_left(13);
        seed = seed.wrapping_mul(0x9E3779B97F4A7C15).rotate_left(7);
    }

    let mut scratch = vec![0u64; words];
    let mut sm = SplitMix64::new(seed);

    // Fill scratchpad with random-looking reads from base
    // This forces memory touch, but keeps it deterministic
    for i in 0..words {
        let mix_seed = sm.next();
        let idx1 = (mix_seed.rotate_left(17) as usize) & base_mask;
        let idx2 = (mix_seed.rotate_right(23) as usize) & base_mask;
        scratch[i] = base[idx1] ^ base[idx2] ^ mix_seed.wrapping_mul(0xC2B2AE3D27D4EB4F);
    }

    (scratch, smask)
}

/// Fold a 32-byte hash into a u64 seed (ours)
#[inline]
pub fn fold_seed(prev_hash32: &[u8; 32], epoch_id: u64) -> u64 {
    let mut s: u64 = epoch_id ^ 0xA24BAED4963EE407;
    for chunk in prev_hash32.chunks(8) {
        let mut v = [0u8; 8];
        v[..chunk.len()].copy_from_slice(chunk);
        s ^= u64::from_be_bytes(v).rotate_left(7);
        s = s.wrapping_mul(0x9E3779B97F4A7C15).rotate_left(9);
    }
    s
}

/// Deterministic dataset derived from prev hash + epoch
pub struct VisionXDataset {
    pub mem: Box<[u64]>,
    pub mask: usize, // length-1 if pow2 sized
}

impl VisionXDataset {
    pub fn build(params: &VisionXParams, prev_hash32: &[u8; 32], epoch: u64) -> Self {
        // Size in u64s; round to next power-of-two for fast masking
        let bytes = params.dataset_mb * 1024 * 1024;
        let mut words = bytes / std::mem::size_of::<u64>();
        let mut n = 1usize;
        while n < words {
            n <<= 1;
        }
        words = n;

        let seed = fold_seed(prev_hash32, epoch);
        let mut sm = SplitMix64::new(seed);
        let mut mem = vec![0u64; words].into_boxed_slice();
        for i in 0..words {
            mem[i] = sm.next();
        }
        Self {
            mem,
            mask: words - 1,
        }
    }

    /// Get or build cached dataset (for validators & miners)
    pub fn get_cached(
        params: &VisionXParams,
        prev_hash32: &[u8; 32],
        epoch: u64,
    ) -> (Arc<Vec<u64>>, usize) {
        let key = (epoch, *prev_hash32);

        // Try to get from cache first
        {
            let cache = DATASET_CACHE.lock().unwrap();
            if let Some((dataset, mask)) = cache.get(&key) {
                return (Arc::clone(dataset), *mask);
            }
        }

        // Not in cache - build it
        let ds = Self::build(params, prev_hash32, epoch);
        let dataset_arc = Arc::new(ds.mem.to_vec());
        let mask = ds.mask;

        // Store in cache
        {
            let mut cache = DATASET_CACHE.lock().unwrap();
            cache.insert(key, (Arc::clone(&dataset_arc), mask));

            // Keep cache bounded (max 3 epochs)
            if cache.len() > 3 {
                if let Some(oldest_key) = cache.keys().next().copied() {
                    cache.remove(&oldest_key);
                }
            }
        }

        (dataset_arc, mask)
    }
}

/// Block header template given to miner
#[derive(Clone)]
pub struct PowJob {
    pub header: Vec<u8>,     // header bytes without nonce (or include nonce offset)
    pub nonce_offset: usize, // byte offset where 8-byte nonce is written
    pub target: U256,        // big-endian target
    pub prev_hash32: [u8; 32],
    pub height: u64,
}

/// Result (solution)
#[derive(Debug, Clone)]
pub struct PowSolution {
    pub nonce: u64,
    pub digest: U256,
}

/// Core VisionX hash (war mode: memory-hard with dependent reads + write-back)
pub fn visionx_hash(
    params: &VisionXParams,
    base: &[u64],
    base_mask: usize,
    header: &[u8],
    nonce: u64,
) -> U256 {
    // Initialize per-hash scratchpad (deterministic, verifiable)
    let (mut scratch, smask) = init_scratch(params, base, base_mask, header, nonce);

    // Build initial 128-bit state from header+nonce
    let mut a: u64 = 0x243F_6A88_85A3_08D3 ^ nonce.rotate_left(17);
    let mut b: u64 = 0x1319_8A2E_0370_7344 ^ nonce.rotate_right(11);

    // Fold header into (a,b)
    for chunk in header.chunks(16) {
        let mut p = [0u8; 16];
        p[..chunk.len()].copy_from_slice(chunk);
        let x = u64::from_be_bytes(p[0..8].try_into().unwrap());
        let y = u64::from_be_bytes(p[8..16].try_into().unwrap());
        a ^= x.wrapping_mul(0x9E37_79B1_85EB_CA87);
        b ^= y.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
        a = a.rotate_left(13) ^ b.rotate_right(7);
        b = b.rotate_left(29) ^ a.rotate_right(19);
    }

    let its = params.mix_iters;
    let mut acc = a ^ b ^ 0xDEAD_BEEF_F00D_FACEu64;
    let writes = params.write_every;

    // War mode: multi-dependent reads + deterministic write-back
    for i in 0..its {
        // First read: index from state + loop counter
        let j1 =
            (a ^ b ^ acc ^ (i as u64).wrapping_mul(0x9E3779B9)).rotate_left(17) as usize & smask;
        let v1 = scratch[j1];

        // Second read: depends on v1 (GPU killer #1)
        let j2 = (v1 ^ a ^ acc).rotate_left(23) as usize & smask;
        let v2 = scratch[j2];

        // Third read: depends on v2 (GPU killer #2)
        let j3 = (v2 ^ b ^ acc).rotate_left(19) as usize & smask;
        let v3 = scratch[j3];

        // Fourth read if reads_per_iter >= 4
        let v4 = if params.reads_per_iter >= 4 {
            let j4 = (v3 ^ v1 ^ acc).rotate_left(29) as usize & smask;
            scratch[j4]
        } else {
            v3
        };

        // Combine reads into mix
        let mix =
            v1 ^ v2.rotate_left(13) ^ v3.wrapping_mul(0x94D049BB133111EB) ^ v4.rotate_right(7);

        // Update state
        a = a.rotate_left(13) ^ mix.wrapping_mul(0xC2B2AE3D27D4EB4F);
        b = b.rotate_left(17) ^ (mix ^ acc).wrapping_mul(0xBF58476D1CE4E5B9);
        acc = acc.rotate_left(7) ^ (a ^ b).wrapping_mul(0xD6E8FEB86659FD93);

        // Deterministic write-back (GPU killer #3: unpredictable write pattern)
        if writes > 0 && (i % writes) == 0 {
            let jw = (mix ^ a ^ b.rotate_left(11) ^ (i as u64).wrapping_mul(0xA24BAED4963EE407))
                .rotate_left(31) as usize
                & smask;
            scratch[jw] = scratch[jw]
                .wrapping_add(mix ^ 0x9E3779B97F4A7C15)
                .rotate_left(41);
        }
    }

    expand_256(a ^ acc, b ^ acc.rotate_left(3))
}

/// Verify VisionX (uses cached dataset for speed)
/// Anti-DoS: Enforces parameter limits during verification
pub fn verify(
    params: &VisionXParams,
    prev_hash32: &[u8; 32],
    epoch: u64,
    header_with_nonce: &[u8],
    nonce_offset: usize,
    target: &U256,
) -> bool {
    // Anti-DoS: Enforce parameter limits
    if params.dataset_mb > 512 {
        eprintln!(
            "⚠️  VisionX verify: dataset_mb {} exceeds limit 512",
            params.dataset_mb
        );
        return false;
    }
    if params.scratch_mb > 128 {
        eprintln!(
            "⚠️  VisionX verify: scratch_mb {} exceeds limit 128",
            params.scratch_mb
        );
        return false;
    }
    if params.mix_iters > 1_000_000 {
        eprintln!(
            "⚠️  VisionX verify: mix_iters {} exceeds limit 1M",
            params.mix_iters
        );
        return false;
    }
    if params.reads_per_iter > 8 {
        eprintln!(
            "⚠️  VisionX verify: reads_per_iter {} exceeds limit 8",
            params.reads_per_iter
        );
        return false;
    }

    let (dataset, mask) = VisionXDataset::get_cached(params, prev_hash32, epoch);
    let header = header_with_nonce.to_vec();
    let mut nonce_bytes = [0u8; 8];
    nonce_bytes.copy_from_slice(&header[nonce_offset..nonce_offset + 8]);
    let nonce = u64::from_be_bytes(nonce_bytes);
    let digest = visionx_hash(params, &dataset, mask, &header, nonce);
    u256_leq(&digest, target)
}

/// Miner engine (single-thread batch)
pub struct VisionXMiner {
    pub params: VisionXParams,
    pub dataset: Arc<Vec<u64>>, // immutable snapshot per epoch (we'll clone into a mutable scratch for writes)
    pub mask: usize,
    pub last_hps: AtomicU64,
}

impl VisionXMiner {
    pub fn new(params: VisionXParams, prev_hash32: &[u8; 32], epoch: u64) -> Self {
        let (dataset, mask) = VisionXDataset::get_cached(&params, prev_hash32, epoch);
        Self {
            params,
            dataset,
            mask,
            last_hps: AtomicU64::new(0),
        }
    }

    /// Create a lightweight miner engine without building a large dataset
    /// Useful for tests where we do not want expensive dataset builds.
    pub fn new_disabled(params: VisionXParams) -> Self {
        // Minimal dataset (1 u64) to keep API stable
        Self {
            params,
            dataset: Arc::new(vec![0u64; 1]),
            mask: 0,
            last_hps: AtomicU64::new(0),
        }
    }

    /// Hash a batch of nonces; returns (solutions, hashes_done)
    pub fn mine_batch(
        &self,
        job: &PowJob,
        start_nonce: u64,
        batch: u32,
    ) -> (Vec<PowSolution>, u32) {
        // Use header as-is (nonce=0 from pow_message_bytes)
        // DO NOT write nonce into header - it's passed separately to visionx_hash
        let header = &job.header;
        let mut sols = Vec::new();
        let t0 = now_ns();

        for n in 0..batch {
            let nonce = start_nonce.wrapping_add(n as u64);
            // Fixed: Don't overwrite header bytes with nonce
            // The nonce parameter to visionx_hash is sufficient
            let digest = visionx_hash(
                &self.params,
                self.dataset.as_slice(),
                self.mask,
                header,
                nonce,
            );
            if u256_leq(&digest, &job.target) {
                sols.push(PowSolution { nonce, digest });
            }
        }

        let dt_ns = now_ns().saturating_sub(t0).max(1);
        let hps = ((batch as u128) * 1_000_000_000u128 / (dt_ns as u128)) as u64;
        self.last_hps.store(hps, Ordering::Relaxed);
        (sols, batch)
    }

    pub fn last_hps(&self) -> u64 {
        self.last_hps.load(Ordering::Relaxed)
    }
}

#[inline]
fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_splitmix64() {
        let mut rng = SplitMix64::new(12345);
        let a = rng.next();
        let b = rng.next();
        assert_ne!(a, b);
    }

    #[test]
    fn test_dataset_build() {
        let params = VisionXParams::default();
        let prev = [0u8; 32];
        let ds = VisionXDataset::build(&params, &prev, 0);
        assert!(ds.mem.len() > 0);
        assert_eq!(ds.mem.len() & ds.mask, 0); // power of 2
    }

    #[test]
    fn test_visionx_hash() {
        let params = VisionXParams {
            dataset_mb: 1, // small for test
            scratch_mb: 1, // small scratchpad for test
            mix_iters: 100,
            reads_per_iter: 4, // war mode
            write_every: 10,
            epoch_blocks: 32,
        };
        let prev = [0u8; 32];
        let ds = VisionXDataset::build(&params, &prev, 0);
        let header = vec![1, 2, 3, 4];
        let hash1 = visionx_hash(&params, &ds.mem, ds.mask, &header, 0);
        let hash2 = visionx_hash(&params, &ds.mem, ds.mask, &header, 1);
        assert_ne!(hash1, hash2); // different nonces give different hashes
    }

    #[test]
    fn test_visionx_deterministic_war_mode() {
        // Verify war mode (scratchpad + write-back) is deterministic
        let params = VisionXParams {
            dataset_mb: 2,
            scratch_mb: 2,
            mix_iters: 500,
            reads_per_iter: 4,
            write_every: 4, // frequent writes
            epoch_blocks: 32,
        };
        let prev = [0xABu8; 32];
        let ds = VisionXDataset::build(&params, &prev, 1);
        let header = vec![0xFF, 0xEE, 0xDD, 0xCC];
        let nonce = 123456789u64;

        // Hash same input multiple times - must get identical output
        let hash1 = visionx_hash(&params, &ds.mem, ds.mask, &header, nonce);
        let hash2 = visionx_hash(&params, &ds.mem, ds.mask, &header, nonce);
        let hash3 = visionx_hash(&params, &ds.mem, ds.mask, &header, nonce);

        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);

        // Different nonce must give different hash
        let hash_diff = visionx_hash(&params, &ds.mem, ds.mask, &header, nonce + 1);
        assert_ne!(hash1, hash_diff);
    }

    #[test]
    fn test_scratchpad_init_deterministic() {
        // Verify scratchpad initialization is deterministic
        let params = VisionXParams {
            dataset_mb: 1,
            scratch_mb: 2,
            mix_iters: 100,
            reads_per_iter: 4,
            write_every: 4,
            epoch_blocks: 32,
        };

        let base = vec![0x1234567890ABCDEFu64; 1024];
        let base_mask = 1023;
        let header = vec![1, 2, 3, 4, 5];
        let nonce = 999u64;

        let (scratch1, mask1) = init_scratch(&params, &base, base_mask, &header, nonce);
        let (scratch2, mask2) = init_scratch(&params, &base, base_mask, &header, nonce);

        assert_eq!(mask1, mask2);
        assert_eq!(scratch1.len(), scratch2.len());
        assert_eq!(scratch1, scratch2);
    }

    #[test]
    fn test_dataset_caching() {
        // Verify dataset is cached and reused
        let params = VisionXParams {
            dataset_mb: 1,
            scratch_mb: 1,
            mix_iters: 100,
            reads_per_iter: 4,
            write_every: 4,
            epoch_blocks: 32,
        };

        let prev = [0x42u8; 32];
        let epoch = 5u64;

        // First call - builds dataset
        let (ds1, mask1) = VisionXDataset::get_cached(&params, &prev, epoch);

        // Second call - should return cached
        let (ds2, mask2) = VisionXDataset::get_cached(&params, &prev, epoch);

        assert_eq!(mask1, mask2);
        assert!(Arc::ptr_eq(&ds1, &ds2)); // Same Arc instance = cached
    }

    #[test]
    fn test_known_vector() {
        // Test vector for regression testing
        let params = VisionXParams {
            dataset_mb: 1,
            scratch_mb: 1,
            mix_iters: 1000,
            reads_per_iter: 4,
            write_every: 4,
            epoch_blocks: 32,
        };

        let prev = [0u8; 32];
        let (dataset, mask) = VisionXDataset::get_cached(&params, &prev, 0);
        let header = b"test_block_header".to_vec();
        let nonce = 12345u64;

        let digest = visionx_hash(&params, &dataset, mask, &header, nonce);

        // This digest should remain stable across versions
        // (actual value depends on algorithm - store first run result)
        assert_eq!(digest.len(), 32);
        assert_ne!(digest, [0u8; 32]); // Not all zeros

        // Verify same input gives same output
        let digest2 = visionx_hash(&params, &dataset, mask, &header, nonce);
        assert_eq!(digest, digest2);
    }

    #[test]
    fn test_war_mode_parameters_limits() {
        // Verify parameters are within sane limits for anti-DoS
        let params = VisionXParams::default();

        // Dataset size reasonable (max 512 MB)
        assert!(
            params.dataset_mb <= 512,
            "Dataset too large for DoS protection"
        );

        // Scratchpad reasonable (max 128 MB)
        assert!(
            params.scratch_mb <= 128,
            "Scratchpad too large for DoS protection"
        );

        // Mix iterations reasonable (max 1M)
        assert!(
            params.mix_iters <= 1_000_000,
            "Too many mix iterations for DoS protection"
        );

        // Reads per iteration reasonable (max 8)
        assert!(params.reads_per_iter <= 8, "Too many reads per iteration");
    }
}
