//! Bounded collections to prevent memory exhaustion attacks
//!
//! Provides LRU-based collections with size limits for mempool, caches, etc.

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

/// LRU cache with size limit
#[derive(Debug, Clone)]
pub struct LruCache<K, V> {
    map: HashMap<K, V>,
    order: VecDeque<K>,
    capacity: usize,
}

impl<K: Clone + Eq + Hash, V> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Insert item, evicting oldest if at capacity
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        // If key exists, remove from order list
        if self.map.contains_key(&key) {
            self.order.retain(|k| k != &key);
        }

        // Evict oldest if at capacity
        if self.map.len() >= self.capacity && !self.map.contains_key(&key) {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            }
        }

        // Add to end (most recent)
        self.order.push_back(key.clone());
        self.map.insert(key, value)
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            // Move to end (mark as recently used)
            self.order.retain(|k| k != key);
            self.order.push_back(key.clone());
            self.map.get(key)
        } else {
            None
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.order.retain(|k| k != key);
        self.map.remove(key)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Retain only items matching predicate
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&K, &V) -> bool,
    {
        let keys_to_remove: Vec<K> = self
            .map
            .iter()
            .filter(|(k, v)| !f(k, v))
            .map(|(k, _)| k.clone())
            .collect();

        for key in keys_to_remove {
            self.remove(&key);
        }
    }
}

/// Bounded set with FIFO eviction
#[derive(Debug, Clone)]
pub struct BoundedSet<T> {
    items: VecDeque<T>,
    capacity: usize,
}

impl<T: Clone + Eq> BoundedSet<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            items: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Insert item, returns true if inserted (false if duplicate)
    pub fn insert(&mut self, item: T) -> bool {
        if self.items.contains(&item) {
            return false;
        }

        if self.items.len() >= self.capacity {
            self.items.pop_front();
        }

        self.items.push_back(item);
        true
    }

    pub fn contains(&self, item: &T) -> bool {
        self.items.contains(item)
    }

    pub fn remove(&mut self, item: &T) -> bool {
        if let Some(pos) = self.items.iter().position(|x| x == item) {
            self.items.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_cache_basic() {
        let mut cache = LruCache::new(3);

        cache.insert("a", 1);
        cache.insert("b", 2);
        cache.insert("c", 3);

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&"a"), Some(&1));
    }

    #[test]
    fn test_lru_cache_eviction() {
        let mut cache = LruCache::new(3);

        cache.insert("a", 1);
        cache.insert("b", 2);
        cache.insert("c", 3);

        // Should evict "a"
        cache.insert("d", 4);

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.get(&"d"), Some(&4));
    }

    #[test]
    fn test_lru_cache_lru_behavior() {
        let mut cache = LruCache::new(3);

        cache.insert("a", 1);
        cache.insert("b", 2);
        cache.insert("c", 3);

        // Access "a", making it most recent
        cache.get(&"a");

        // Insert new item, should evict "b" (least recently used)
        cache.insert("d", 4);

        assert_eq!(cache.get(&"a"), Some(&1));
        assert_eq!(cache.get(&"b"), None);
        assert_eq!(cache.get(&"c"), Some(&3));
        assert_eq!(cache.get(&"d"), Some(&4));
    }

    #[test]
    fn test_bounded_set_basic() {
        let mut set = BoundedSet::new(3);

        assert!(set.insert("a"));
        assert!(set.insert("b"));
        assert!(set.insert("c"));

        assert!(!set.insert("a")); // Duplicate
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_bounded_set_eviction() {
        let mut set = BoundedSet::new(3);

        set.insert("a");
        set.insert("b");
        set.insert("c");

        // Should evict "a"
        set.insert("d");

        assert!(!set.contains(&"a"));
        assert!(set.contains(&"d"));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_lru_cache_update() {
        let mut cache = LruCache::new(3);

        cache.insert("a", 1);
        cache.insert("a", 10); // Update

        assert_eq!(cache.get(&"a"), Some(&10));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_lru_cache_retain() {
        let mut cache = LruCache::new(10);

        for i in 0..5 {
            cache.insert(i, i * 10);
        }

        // Retain only even keys
        cache.retain(|k, _v| k % 2 == 0);

        assert_eq!(cache.len(), 3); // 0, 2, 4
        assert_eq!(cache.get(&0), Some(&0));
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&2), Some(&20));
    }
}
