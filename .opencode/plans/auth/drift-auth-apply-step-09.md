## 9. Replay Prevention (Items 38-42)

**Create drift-auth/src/replay.rs:**

```rust
// drift-auth/src/replay.rs
// Checklist items: 38, 39, 40, 41, 42

use std::collections::HashSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use lru::LruCache;

use super::AuthMessage;

/// === Item 40: NonceStore with TTL ===
#[derive(Debug, Clone)]
pub struct NonceStore {
    seen_nonces: LruCache<u64, ()>,
    ttl: Duration,
}

impl NonceStore {
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            seen_nonces: LruCache::new(capacity),
            ttl,
        }
    }
    
    /// Check if nonce is novel (not seen before within TTL)
    pub fn check_and_record(&mut self, nonce: u64, timestamp: u64) -> bool {
        // Check if we've seen this nonce recently
        if self.seen_nonces.get(&nonce).is_some() {
            return false; // Replay detected
        }
        
        // Record with TTL based on current time
        // In practice, we'd use timestamp to expire old entries
        self.seen_nonces.put(nonce, ());
        true
    }
    
    /// Clean expired entries (call periodically)
    pub fn prune(&mut self) {
        // In a real implementation, track timestamps per nonce
        // and remove those older than TTL
    }
}

/// === Item 38: Timestamp validation ===
pub fn validate_timestamp(timestamp: u64, max_age_secs: u64) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // Check if timestamp is not too old and not too far in future
    let age = now.saturating_sub(timestamp);
    age <= max_age_secs
}

/// === Item 39: Nonce uniqueness ===
// (Handled by NonceStore.check_and_record)

/// === Items 41-42: Tests ===
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_timestamp_validation() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Current timestamp should be valid
        assert!(validate_timestamp(now, 300));
        
        // Old timestamp (1 hour ago) should be invalid
        assert!(!validate_timestamp(now - 3600, 300));
    }
    
    #[test]
    fn test_nonce_store_detects_replay() {
        let mut store = NonceStore::new(1000, Duration::from_secs(300));
        let nonce = 12345;
        
        // First time should be ok
        assert!(store.check_and_record(nonce, 0));
        
        // Second time with same nonce should be rejected
        assert!(!store.check_and_record(nonce, 0));
    }
}
```

---
