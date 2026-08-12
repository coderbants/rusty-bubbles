//! Cleanroom Rust port of upstream Go source file: `internal/memoization/memoization.go`
//! Upstream Target Tag / Version: `v2.1.0`

use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Hasher is an interface that requires a Hash method. The Hash method is
/// expected to return a string representation of the hash of the object.
pub trait Hasher {
    /// Hash returns the string representation of the hash of the object.
    fn hash(&self) -> String;
}

struct Entry<T> {
    #[allow(dead_code)]
    key: String,
    value: T,
}

/// MemoCache is a struct that represents a cache with a set capacity. It
/// uses an LRU (Least Recently Used) eviction policy.
pub struct MemoCache<T> {
    capacity: usize,
    cache: HashMap<String, Entry<T>>,
    order: Vec<String>,
}

/// NewMemoCache is a function that creates a new MemoCache with a given
/// capacity.
pub fn new_memo_cache<T>(capacity: usize) -> MemoCache<T> {
    MemoCache {
        capacity,
        cache: HashMap::new(),
        order: Vec::new(),
    }
}

impl<T> MemoCache<T> {
    /// Capacity returns the capacity of the MemoCache.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Size returns the current size of the MemoCache. It is the number of
    /// items currently stored in the cache.
    pub fn size(&self) -> usize {
        self.order.len()
    }

    /// Get returns the value associated with the given hashable item in the
    /// MemoCache. If there is no corresponding value, the method returns None.
    pub fn get<H: Hasher>(&mut self, h: &H) -> Option<&T> {
        let hashed_key = h.hash();
        if let Some(entry) = self.cache.get(&hashed_key) {
            // Move to front (most recently used).
            if let Some(pos) = self.order.iter().position(|k| *k == hashed_key) {
                let k = self.order.remove(pos);
                self.order.push(k);
            }
            return Some(&entry.value);
        }
        None
    }

    /// Set sets the value for the given hashable item in the MemoCache. If
    /// the cache is at capacity, it evicts the least recently used item
    /// before adding the new item.
    pub fn set<H: Hasher>(&mut self, h: &H, value: T) {
        let hashed_key = h.hash();
        if let Some(entry) = self.cache.get_mut(&hashed_key) {
            entry.value = value;
            if let Some(pos) = self.order.iter().position(|k| *k == hashed_key) {
                let k = self.order.remove(pos);
                self.order.push(k);
            }
            return;
        }

        // Check if the cache is at capacity
        if self.order.len() >= self.capacity {
            // Evict the least recently used item from the cache
            if let Some(oldest) = self.order.first().cloned() {
                self.order.remove(0);
                self.cache.remove(&oldest);
            }
        }

        let entry = Entry {
            key: hashed_key.clone(),
            value,
        };
        self.cache.insert(hashed_key.clone(), entry);
        self.order.push(hashed_key);
    }
}

/// HString is a type that implements the Hasher interface for strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HString(pub String);

impl Hasher for HString {
    fn hash(&self) -> String {
        let digest = Sha256::digest(self.0.as_bytes());
        let mut s = String::new();
        for b in digest.iter() {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }
}

/// HInt is a type that implements the Hasher interface for integers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HInt(pub i64);

impl Hasher for HInt {
    fn hash(&self) -> String {
        let digest = Sha256::digest(self.0.to_string().as_bytes());
        let mut s = String::new();
        for b in digest.iter() {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }
}
