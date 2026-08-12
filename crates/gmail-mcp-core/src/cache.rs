//! A minimal process-local TTL cache.
//!
//! This is the only caching the application uses. There is no persistent mail
//! cache by design.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A simple TTL cache keyed by `String`.
///
/// `ponytail: single Mutex<HashMap>; per-key sharding only if a profiler shows
/// contention. Entries are lazily evicted on access and on `prune`.
pub struct TtlCache<V> {
    inner: Mutex<Inner<V>>,
}

struct Inner<V> {
    map: HashMap<String, (Instant, V)>,
    ttl: Duration,
}

impl<V> TtlCache<V> {
    pub fn new(ttl: Duration) -> Self {
        TtlCache {
            inner: Mutex::new(Inner {
                map: HashMap::new(),
                ttl,
            }),
        }
    }

    pub fn get(&self, key: &str) -> Option<V>
    where
        V: Clone,
    {
        let mut inner = self.inner.lock().unwrap();
        let now = Instant::now();
        match inner.map.get(key) {
            Some((inserted, value)) if now.duration_since(*inserted) < inner.ttl => {
                Some(value.clone())
            }
            _ => {
                inner.map.remove(key);
                None
            }
        }
    }

    pub fn insert(&self, key: String, value: V) {
        let mut inner = self.inner.lock().unwrap();
        inner.map.insert(key, (Instant::now(), value));
    }

    /// Remove expired entries. Returns the number removed.
    pub fn prune(&self) -> usize {
        let mut inner = self.inner.lock().unwrap();
        let now = Instant::now();
        let ttl = inner.ttl;
        let before = inner.map.len();
        inner
            .map
            .retain(|_, (inserted, _)| now.duration_since(*inserted) < ttl);
        before - inner.map.len()
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_expires() {
        let cache = TtlCache::new(Duration::from_millis(50));
        cache.insert("k".into(), 42);
        assert_eq!(cache.get("k"), Some(42));
        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(cache.get("k"), None);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn prune_removes_expired() {
        let cache = TtlCache::new(Duration::from_millis(20));
        cache.insert("a".into(), 1);
        cache.insert("b".into(), 2);
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(cache.prune(), 2);
        assert!(cache.is_empty());
    }
}
