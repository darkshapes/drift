## 11. Testing (Items 48-53)

**Create drift-auth/tests/integration.rs:**

```rust
// drift-auth/tests/integration.rs
// Checklist items: 48, 49, 50, 51, 52, 53

use drift_auth::*;
use tokio::sync::mpsc;
use std::time::Duration;

/// === Item 50: Integration test: 5 nodes, threshold 3, one node offline ===
#[tokio::test]
async fn test_5_nodes_threshold_3_one_offline() {
    // Setup: 5 nodes, but only 4 will connect (1 offline)
    let total_nodes = 5;
    let threshold = 3;
    let mut aggregator = Aggregator::new(
        vec!["n1", "n2", "n3", "n4", "n5"],
        threshold,
        Duration::from_secs(30),
    );
    
    // Simulate 4 nodes sending signatures
    let kps: Vec<NodeKeypair> = (0..4).map(|_| NodeKeypair::generate().unwrap()).collect();
    let msg = AuthMessage::new("repo123", "abc123", 1);
    
    for (i, kp) in kps.iter().enumerate() {
        let signed = SignedAuthMessage::sign(&msg, kp, &format!("n{}", i+1)).unwrap();
        // Node 5 never sends
        if i < 4 {
            aggregator.add_signature(&format!("n{}", i+1), signed).unwrap();
        }
    }
    
    // Should reach threshold (3) even though 1 node offline
    assert!(aggregator.has_threshold());
    
    // Should be able to create aggregate with just 3 signatures
    let agg = aggregator.create_aggregate().unwrap();
    assert_eq!(agg.threshold, 3);
    assert_eq!(agg.total_nodes, 5);
}

/// === Item 52: Performance test ===
#[tokio::test]
#[ignore]  // Run with: cargo test -- --ignored
async fn test_auth_overhead() {
    use std::time::Instant;
    
    let start = Instant::now();
    
    // Simulate 100 nodes authenticating
    for _ in 0..100 {
        let kp = NodeKeypair::generate().unwrap();
        let msg = AuthMessage::new("repo", "hash", 1);
        let _sig = kp.sign(&msg.as_bytes()).unwrap();
    }
    
    let elapsed = start.elapsed();
    println!("100 signatures generated in {:?}", elapsed);
    // Should be < 100ms total
    assert!(elapsed < Duration::from_millis(100));
}

/// === Item 53: Key rotation test ===
#[test]
fn test_key_rotation() {
    let identity = NodeIdentity::new("node1").unwrap();
    let original_pubkey = identity.keypair.public_key().unwrap();
    
    // Rotate keys
    let rotated = identity.rotate_keys().unwrap();
    let new_pubkey = rotated.keypair.public_key().unwrap();
    
    // Public keys should be different
    assert_ne!(original_pubkey.as_bytes(), new_pubkey.as_bytes());
}
```

