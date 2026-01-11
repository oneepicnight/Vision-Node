#![allow(dead_code)]
//! Orphan Block Handling
//!
//! Manages blocks received out-of-order (parent not yet known)

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

/// LRU cache for orphan blocks
pub struct OrphanPool {
    /// Orphan blocks indexed by hash
    orphans: HashMap<String, OrphanEntry>,
    /// Children waiting for parent (parent_hash -> child_hashes)
    waiting: HashMap<String, Vec<String>>,
    /// LRU queue for eviction
    lru: VecDeque<String>,
    /// Maximum orphans to keep
    max_size: usize,
}

#[derive(Debug, Clone)]
struct OrphanEntry {
    block: crate::Block,
    added_at: Instant,
}

impl OrphanPool {
    pub fn new(max_size: usize) -> Self {
        Self {
            orphans: HashMap::new(),
            waiting: HashMap::new(),
            lru: VecDeque::new(),
            max_size,
        }
    }

    /// Add an orphan block
    pub fn add_orphan(&mut self, block: crate::Block) -> bool {
        let hash = block.header.pow_hash.clone();
        let parent_hash = block.header.parent_hash.clone();

        // Check if already exists
        if self.orphans.contains_key(&hash) {
            return false;
        }

        // Evict oldest if at capacity
        if self.orphans.len() >= self.max_size {
            if let Some(oldest) = self.lru.pop_front() {
                self.orphans.remove(&oldest);
                // Clean up waiting map
                if let Some(children) = self.waiting.remove(&oldest) {
                    for child in children {
                        self.orphans.remove(&child);
                    }
                }
            }
        }

        // Add to orphan pool
        self.orphans.insert(
            hash.clone(),
            OrphanEntry {
                block,
                added_at: Instant::now(),
            },
        );

        // Track parent-child relationship
        self.waiting
            .entry(parent_hash)
            .or_default()
            .push(hash.clone());

        // Add to LRU
        self.lru.push_back(hash);

        true
    }

    /// Get orphans that can now be processed (parent just arrived)
    pub fn get_children(&mut self, parent_hash: &str) -> Vec<crate::Block> {
        if let Some(child_hashes) = self.waiting.remove(parent_hash) {
            let mut children = Vec::new();
            for hash in child_hashes {
                if let Some(entry) = self.orphans.remove(&hash) {
                    // Remove from LRU
                    self.lru.retain(|h| h != &hash);
                    children.push(entry.block);
                }
            }
            children
        } else {
            Vec::new()
        }
    }

    /// Check if a block is orphaned
    pub fn is_orphan(&self, hash: &str) -> bool {
        self.orphans.contains_key(hash)
    }

    /// Get a specific orphan block by hash
    pub fn get_orphan(&self, hash: &str) -> Option<crate::Block> {
        self.orphans.get(hash).map(|entry| entry.block.clone())
    }

    /// Get orphan count
    pub fn len(&self) -> usize {
        self.orphans.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.orphans.is_empty()
    }

    /// Get missing parent hashes (blocks we need to request)
    pub fn get_missing_parents(&self) -> Vec<String> {
        self.waiting.keys().cloned().collect()
    }

    /// Prune orphans older than age_secs
    pub fn prune_old(&mut self, age_secs: u64) {
        let cutoff = Instant::now() - std::time::Duration::from_secs(age_secs);
        let mut to_remove = Vec::new();

        for (hash, entry) in &self.orphans {
            if entry.added_at < cutoff {
                to_remove.push(hash.clone());
            }
        }

        for hash in to_remove {
            self.orphans.remove(&hash);
            self.lru.retain(|h| h != &hash);
            // Note: waiting map cleanup happens naturally
        }
    }

    /// Expire orphans older than specified duration
    pub fn expire_older_than(&mut self, max_age: std::time::Duration) {
        let cutoff = Instant::now() - max_age;
        let mut expired = Vec::new();

        for (hash, entry) in &self.orphans {
            if entry.added_at < cutoff {
                expired.push(hash.clone());
            }
        }

        let count = expired.len();
        for hash in expired {
            self.orphans.remove(&hash);
            self.lru.retain(|h| h != &hash);

            // Clean up waiting map
            self.waiting.retain(|_, children| {
                children.retain(|h| h != &hash);
                !children.is_empty()
            });
        }

        if count > 0 {
            eprintln!(
                "🗑️ Expired {} orphan blocks (older than {:?})",
                count, max_age
            );
        }
    }

    /// Adopt children of a parent that just arrived
    /// Returns children ready for processing
    pub fn adopt_children(&mut self, parent_hash: &str) -> Vec<crate::Block> {
        self.get_children(parent_hash)
    }
}

/// Seen filters for deduplication
pub struct SeenFilters {
    /// Recently seen header hashes
    seen_headers: LruSet,
    /// Recently seen block hashes
    seen_blocks: LruSet,
}

impl SeenFilters {
    pub fn new(capacity: usize) -> Self {
        Self {
            seen_headers: LruSet::new(capacity),
            seen_blocks: LruSet::new(capacity),
        }
    }

    /// Check and mark header as seen
    pub fn mark_header_seen(&mut self, hash: &str) -> bool {
        self.seen_headers.insert(hash.to_string())
    }

    /// Check and mark block as seen
    pub fn mark_block_seen(&mut self, hash: &str) -> bool {
        self.seen_blocks.insert(hash.to_string())
    }
}

/// Simple LRU set
struct LruSet {
    items: HashMap<String, ()>,
    lru: VecDeque<String>,
    capacity: usize,
}

impl LruSet {
    fn new(capacity: usize) -> Self {
        Self {
            items: HashMap::new(),
            lru: VecDeque::new(),
            capacity,
        }
    }

    /// Insert item, returns true if already existed
    fn insert(&mut self, item: String) -> bool {
        if self.items.contains_key(&item) {
            return true; // Already seen
        }

        // Evict oldest if at capacity
        if self.items.len() >= self.capacity {
            if let Some(oldest) = self.lru.pop_front() {
                self.items.remove(&oldest);
            }
        }

        self.items.insert(item.clone(), ());
        self.lru.push_back(item);

        false // Not seen before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seen_filters() {
        let mut filters = SeenFilters::new(100);

        assert!(!filters.mark_header_seen("h1")); // First time
        assert!(filters.mark_header_seen("h1")); // Second time (seen)

        assert!(!filters.mark_block_seen("b1"));
        assert!(filters.mark_block_seen("b1"));
    }
}
