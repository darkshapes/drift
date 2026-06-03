## 4. Coordinator Aggregation (Items 11-16)

**Note:** The coordinator collects signatures from nodes and aggregates them using Ed25519's additive property.

**Create drift-auth/src/aggregator.rs:**

```rust
// drift-auth/src/aggregator.rs
// Checklist items: 11, 12, 13, 14, 15, 16

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use thiserror::Error;

use super::{AggregateAuthMessage, SignedAuthMessage, AuthMessage, CryptoError};

/// === Item 11: Aggregator struct ===
#[derive(Debug)]
pub struct Aggregator {
    /// Map of node_id -> received SignedAuthMessage
    signatures: HashMap<String, SignedAuthMessage>,
    /// Set of nodes we're waiting for
    expected_nodes: HashSet<String>,
    /// Threshold of signatures required
    threshold: usize,
    /// Total nodes in this round
    total_nodes: usize,
    /// Timeout for collection
    timeout: Duration,
    /// When we started collecting
    start_time: Instant,
}

/// === Item 15: Error types ===
#[derive(Error, Debug)]
pub enum AggregationError {
    #[error("timeout waiting for signatures")]
    Timeout,
    #[error("threshold not reached: {0}/{1}")]
    ThresholdNotReached(usize, usize),
    #[error("duplicate signature from node {0}")]
    DuplicateSignature(String),
    #[error("message mismatch: {0}")]
    MessageMismatch(String),
}

impl Aggregator {
    /// Create new aggregator for a round
    pub fn new(
        expected_nodes: Vec<String>,
        threshold: usize,
        timeout: Duration,
    ) -> Self {
        let total_nodes = expected_nodes.len();
        Self {
            signatures: HashMap::new(),
            expected_nodes: expected_nodes.into_iter().collect(),
            threshold,
            total_nodes,
            timeout,
            start_time: Instant::now(),
        }
    }
    
    /// === Item 12: Add signature ===
    pub fn add_signature(
        &mut self,
        node_id: &str,
        signed: SignedAuthMessage,
    ) -> Result<(), AggregationError> {
        // Check if we're expecting this node
        if !self.expected_nodes.contains(node_id) {
            return Err(AggregationError::MessageMismatch(
                format!("node {} not in expected set", node_id)
            ));
        }
        
        // Check for duplicates
        if self.signatures.contains_key(node_id) {
            return Err(AggregationError::DuplicateSignature(node_id.to_string()));
        }
        
        // Validate that all signatures have same message content
        if let Some((_first_id, first_sig)) = self.signatures.iter().next() {
            let first_msg = &first_sig.message;
            if signed.message.node_id != first_msg.node_id ||
               signed.message.repo_hash != first_msg.repo_hash ||
               signed.message.timestamp != first_msg.timestamp ||
               signed.message.nonce != first_msg.nonce ||
               signed.message.sequence != first_msg.sequence {
                return Err(AggregationError::MessageMismatch(
                    "inconsistent message content across signatures".to_string()
                ));
            }
        }
        
        self.signatures.insert(node_id.to_string(), signed);
        Ok(())
    }
    
    /// === Item 13: Check threshold ===
    pub fn has_threshold(&self) -> bool {
        self.signatures.len() >= self.threshold
    }
    
    /// === Item 14: Create aggregate ===
    pub fn create_aggregate(&self) -> Result<AggregateAuthMessage, AggregationError> {
        if !self.has_threshold() {
            return Err(AggregationError::ThresholdNotReached(
                self.signatures.len(), self.threshold
            ));
        }
        
        let signed_messages: Vec<SignedAuthMessage] = self.signatures
            .values()
            .cloned()
            .collect();
        
        AggregateAuthMessage::create(
            signed_messages,
            self.threshold,
            self.total_nodes,
        ).map_err(|e| AggregationError::MessageMismatch(e.to_string()))
    }
    
    /// === Item 15: Timeout handling ===
    pub fn is_timed_out(&self) -> bool {
        self.start_time.elapsed() > self.timeout
    }
    
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    /// Get number of signatures collected
    pub fn collected_count(&self) -> usize {
        self.signatures.len()
    }
    
    /// Get remaining nodes we haven't heard from
    pub fn missing_nodes(&self) -> Vec<&String> {
        self.expected_nodes.difference(&self.signatures.keys().collect())
            .collect()
    }
}

/// === Item 16: Test aggregation with 3-of-5 threshold ===
#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Keypair;
    
    #[test]
    fn test_aggregator_threshold() {
        let nodes: Vec<String> = (0..5).map(|i| format!("node{}", i)).collect();
        let mut agg = Aggregator::new(nodes.clone(), 3, Duration::from_secs(30));
        
        // Create signatures from 5 nodes
        let mut rng = rand::rngs::OsRng;
        let keypairs: Vec<Keypair> = (0..5).map(|_| Keypair::generate(&mut rng)).collect();
        let msg = AuthMessage::new("test_repo", "abc123", 1);
        
        for (i, kp) in keypairs.iter().enumerate() {
            let signed = SignedAuthMessage::sign(&msg, kp).unwrap();
            agg.add_signature(&nodes[i], signed).unwrap();
        }
        
        assert!(agg.has_threshold());
        assert_eq!(agg.collected_count(), 5);
        assert!(agg.missing_nodes().is_empty());
        
        let aggregate = agg.create_aggregate().unwrap();
        assert_eq!(aggregate.threshold, 3);
        assert_eq!(aggregate.total_nodes, 6);
    }
    
    #[test]
    fn test_aggregator_timeout() {
        let nodes = vec!["node1".to_string(), "node2".to_string()];
        let agg = Aggregator::new(nodes, 2, Duration::from_millis(10));
        
        // Immediately check timeout - should not be timed out
        assert!(!agg.is_timed_out());
    }
}
```

---
