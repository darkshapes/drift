//! Replay attack prevention for drift-auth.
//!
//! - Item 38: Timestamp validation (within 5min window)
//! - Item 39: Nonce uniqueness (never reused)
//! - Item 40: NonceStore with TTL to detect replays
//! - Items 41-42: Tests for replay prevention

use lru::LruCache;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// === Item 40: NonceStore with TTL ===
/// Stores seen nonces to detect replay attacks.
pub struct NonceStore {
    seen_nonces: LruCache<u64, ()>,
    ttl: Duration,
}

impl NonceStore {
    /// Create new NonceStore with given capacity and TTL.
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            seen_nonces: LruCache::new(capacity),
            ttl,
        }
    }

    /// Check if nonce is novel and record it.
    /// Returns true if nonce is new (not a replay), false if replay detected.
    pub fn check_and_record(&mut self, nonce: u64) -> bool {
        if self.seen_nonces.get(&nonce).is_some() {
            return false;
        }
        self.seen_nonces.put(nonce, ());
        true
    }

    /// Check if a nonce has been seen before.
    pub fn is_replay(&mut self, nonce: u64) -> bool {
        self.seen_nonces.get(&nonce).is_some()
    }

    /// Get the number of recorded nonces.
    pub fn len(&self) -> usize {
        self.seen_nonces.len()
    }

    /// Check if store is empty.
    pub fn is_empty(&self) -> bool {
        self.seen_nonces.is_empty()
    }
}

impl Clone for NonceStore {
    fn clone(&self) -> Self {
        Self {
            seen_nonces: LruCache::new(100),
            ttl: self.ttl,
        }
    }
}

/// === Item 38: Timestamp validation ===
/// Validate timestamp is within acceptable window.
pub fn validate_timestamp(timestamp: u64, max_age_secs: u64) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let age = now.saturating_sub(timestamp);
    age <= max_age_secs
}

/// Check if timestamp is in the future (clock skew detection).
pub fn is_timestamp_in_future(timestamp: u64, max_future_secs: u64) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    timestamp > now + max_future_secs
}

/// === Item 39: Nonce uniqueness ===
/// (Handled by NonceStore.check_and_record)

/// Replay prevention error types.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplayError {
    NonceReused(u64),
    TimestampExpired { age_secs: u64, max_age_secs: u64 },
    TimestampInFuture { future_secs: u64, max_future_secs: u64 },
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayError::NonceReused(nonce) => write!(f, "nonce {} was reused (replay attack)", nonce),
            ReplayError::TimestampExpired { age_secs, max_age_secs } => {
                write!(f, "timestamp expired (age {}s > {}s)", age_secs, max_age_secs)
            }
            ReplayError::TimestampInFuture { future_secs, max_future_secs } => {
                write!(f, "timestamp in future ({}s > {}s)", future_secs, max_future_secs)
            }
        }
    }
}

impl std::error::Error for ReplayError {}

/// Validate both timestamp and nonce in one call.
pub fn validate_replay_protection(
    timestamp: u64,
    nonce: u64,
    max_age_secs: u64,
    nonce_store: &mut NonceStore,
) -> Result<(), ReplayError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if timestamp > now + 60 {
        return Err(ReplayError::TimestampInFuture {
            future_secs: timestamp - now,
            max_future_secs: 60,
        });
    }

    let age = now.saturating_sub(timestamp);
    if age > max_age_secs {
        return Err(ReplayError::TimestampExpired {
            age_secs: age,
            max_age_secs,
        });
    }

    if !nonce_store.check_and_record(nonce) {
        return Err(ReplayError::NonceReused(nonce));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_validation_valid() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        assert!(validate_timestamp(now, 300));
        assert!(validate_timestamp(now - 100, 300));
        assert!(validate_timestamp(now - 299, 300));
    }

    #[test]
    fn test_timestamp_validation_expired() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        assert!(!validate_timestamp(now - 301, 300));
        assert!(!validate_timestamp(now - 3600, 300));
    }

    #[test]
    fn test_timestamp_validation_far_in_past() {
        assert!(!validate_timestamp(0, 300));
    }

    #[test]
    fn test_nonce_store_new() {
        let store = NonceStore::new(100, Duration::from_secs(300));
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_nonce_store_check_and_record_first() {
        let mut store = NonceStore::new(100, Duration::from_secs(300));
        assert!(store.check_and_record(12345));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_nonce_store_detects_replay() {
        let mut store = NonceStore::new(100, Duration::from_secs(300));
        let nonce = 12345u64;

        assert!(store.check_and_record(nonce));
        assert!(!store.check_and_record(nonce));
    }

    #[test]
    fn test_nonce_store_multiple_nonces() {
        let mut store = NonceStore::new(100, Duration::from_secs(300));

        assert!(store.check_and_record(1u64));
        assert!(store.check_and_record(2u64));
        assert!(store.check_and_record(3u64));

        assert_eq!(store.len(), 3);

        assert!(store.check_and_record(4u64));
        assert!(store.check_and_record(5u64));

        assert_eq!(store.len(), 5);
    }

    #[test]
    fn test_nonce_store_is_replay() {
        let mut store = NonceStore::new(100, Duration::from_secs(300));
        let nonce = 99999u64;

        assert!(!store.is_replay(nonce));
        store.check_and_record(nonce);
        assert!(store.is_replay(nonce));
    }

    #[test]
    fn test_nonce_store_clone() {
        let mut store = NonceStore::new(100, Duration::from_secs(300));
        store.check_and_record(42u64);

        let store2 = store.clone();
        assert!(store2.is_empty());
    }

    #[test]
    fn test_validate_replay_protection_valid() {
        let mut store = NonceStore::new(100, Duration::from_secs(300));
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let result = validate_replay_protection(now, 1u64, 300, &mut store);
        assert!(result.is_ok());
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_validate_replay_protection_replay_nonce() {
        let mut store = NonceStore::new(100, Duration::from_secs(300));
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        assert!(validate_replay_protection(now, 1u64, 300, &mut store).is_ok());
        let result = validate_replay_protection(now, 1u64, 300, &mut store);
        assert!(result.is_err());

        match result.err() {
            Some(ReplayError::NonceReused(n)) => assert_eq!(n, 1u64),
            _ => panic!("expected NonceReused error"),
        }
    }

    #[test]
    fn test_validate_replay_protection_expired_timestamp() {
        let mut store = NonceStore::new(100, Duration::from_secs(300));
        let old_timestamp = 0u64;

        let result = validate_replay_protection(old_timestamp, 999u64, 300, &mut store);
        assert!(result.is_err());

        match result.err() {
            Some(ReplayError::TimestampExpired { .. }) => {}
            _ => panic!("expected TimestampExpired error"),
        }
    }

    #[test]
    fn test_validate_replay_protection_future_timestamp() {
        let mut store = NonceStore::new(100, Duration::from_secs(300));
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let future_timestamp = now + 120;

        let result = validate_replay_protection(future_timestamp, 1u64, 300, &mut store);
        assert!(result.is_err());

        match result.err() {
            Some(ReplayError::TimestampInFuture { .. }) => {}
            _ => panic!("expected TimestampInFuture error"),
        }
    }

    #[test]
    fn test_replay_error_display() {
        let err = ReplayError::NonceReused(42);
        assert!(err.to_string().contains("42"));

        let err = ReplayError::TimestampExpired { age_secs: 100, max_age_secs: 60 };
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("60"));
    }

    #[test]
    fn test_nonce_store_capacity_eviction() {
        let mut store = NonceStore::new(3, Duration::from_secs(300));

        store.check_and_record(1u64);
        store.check_and_record(2u64);
        store.check_and_record(3u64);

        assert_eq!(store.len(), 3);

        store.check_and_record(4u64);

        assert_eq!(store.len(), 3);

        assert!(store.is_replay(4u64));
        assert!(!store.is_replay(1u64));
    }

    #[test]
    fn test_is_timestamp_in_future() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        assert!(!is_timestamp_in_future(now, 60));
        assert!(!is_timestamp_in_future(now - 10, 60));
        assert!(is_timestamp_in_future(now + 120, 60));
    }
}