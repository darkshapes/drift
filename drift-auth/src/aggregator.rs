//! Aggregator for collecting and aggregating signatures from multiple nodes.
//!
//! - Item 11: Aggregator struct to track collected signatures per round
//! - Item 12: add_signature to add a node's signature
//! - Item 13: check_threshold to see if we have m-of-n
//! - Item 14: create_aggregate to build the final aggregate message
//! - Item 15: Timeout handling for signature collection
//! - Item 16: Tests for 3-of-5 threshold aggregation

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use thiserror::Error;
use uuid::Uuid;

use super::{AggregateAuthMessage, SignedAuthMessage};
use crate::messages::RoundId;

/// === Item 11: Aggregator struct ===
#[derive(Debug, Clone)]
pub struct Aggregator {
    signatures: HashMap<String, SignedAuthMessage>,
    expected_nodes: HashSet<String>,
    threshold: usize,
    total_nodes: usize,
    timeout: Duration,
    start_time: Instant,
    round_id: RoundId,
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
            round_id: Uuid::new_v4(),
        }
    }

    pub fn round_id(&self) -> RoundId {
        self.round_id
    }

    pub fn reset(&mut self) {
        self.signatures.clear();
        self.start_time = Instant::now();
        self.round_id = Uuid::new_v4();
    }

    /// === Item 12: Add signature ===
    pub fn add_signature(
        &mut self,
        node_id: &str,
        signed: SignedAuthMessage,
    ) -> Result<(), AggregationError> {
        if !self.expected_nodes.contains(node_id) {
            return Err(AggregationError::MessageMismatch(
                format!("node {} not in expected set", node_id)
            ));
        }

        if self.signatures.contains_key(node_id) {
            return Err(AggregationError::DuplicateSignature(node_id.to_string()));
        }

        if let Some((_first_id, first_sig)) = self.signatures.iter().next() {
            let first_msg = &first_sig.message;
            if signed.message.node_id != first_msg.node_id
                || signed.message.repo_hash != first_msg.repo_hash
                || signed.message.timestamp != first_msg.timestamp
                || signed.message.nonce != first_msg.nonce
                || signed.message.sequence != first_msg.sequence
            {
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
                self.signatures.len(),
                self.threshold
            ));
        }

        let signed_messages: Vec<SignedAuthMessage> = self.signatures.values().cloned().collect();

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

    pub fn collected_count(&self) -> usize {
        self.signatures.len()
    }

    pub fn total_nodes(&self) -> usize {
        self.total_nodes
    }

    pub fn threshold(&self) -> usize {
        self.threshold
    }

    pub fn missing_nodes(&self) -> Vec<String> {
        self.expected_nodes
            .iter()
            .filter(|n| !self.signatures.contains_key(*n))
            .map(|s| s.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuthMessage;
    use ed25519_dalek::SigningKey;
    use std::time::Duration;

    fn create_signed_message(
        _node_id: &str,
        msg: &AuthMessage,
        keypair: &SigningKey,
    ) -> SignedAuthMessage {
        SignedAuthMessage::sign(msg, keypair).unwrap()
    }

    #[test]
    fn test_aggregator_new() {
        let nodes = vec!["node1".to_string(), "node2".to_string(), "node3".to_string()];
        let agg = Aggregator::new(nodes.clone(), 2, Duration::from_secs(30));

        assert_eq!(agg.threshold, 2);
        assert_eq!(agg.total_nodes, 3);
        assert_eq!(agg.collected_count(), 0);
        assert!(!agg.is_timed_out());
    }

    #[test]
    fn test_aggregator_add_signature() {
        let mut rng = crate::rng::CryptoOsRng::new();
        let kp = SigningKey::generate(&mut rng);
        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);

        let mut agg = Aggregator::new(
            vec!["node1".to_string()],
            1,
            Duration::from_secs(30),
        );

        let signed = create_signed_message("node1", &msg, &kp);
        let result = agg.add_signature("node1", signed);
        assert!(result.is_ok());
        assert_eq!(agg.collected_count(), 1);
    }

    #[test]
    fn test_aggregator_duplicate_signature() {
        let mut rng = crate::rng::CryptoOsRng::new();
        let kp = SigningKey::generate(&mut rng);
        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);

        let mut agg = Aggregator::new(
            vec!["node1".to_string(), "node2".to_string()],
            2,
            Duration::from_secs(30),
        );

        let signed1 = create_signed_message("node1", &msg, &kp);
        agg.add_signature("node1", signed1).unwrap();

        let signed2 = create_signed_message("node1", &msg, &kp);
        let result = agg.add_signature("node1", signed2);
        assert!(matches!(result, Err(AggregationError::DuplicateSignature(_))));
    }

    #[test]
    fn test_aggregator_unexpected_node() {
        let mut rng = crate::rng::CryptoOsRng::new();
        let kp = SigningKey::generate(&mut rng);
        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);

        let mut agg = Aggregator::new(
            vec!["node1".to_string()],
            1,
            Duration::from_secs(30),
        );

        let signed = create_signed_message("node1", &msg, &kp);
        let result = agg.add_signature("node2", signed);
        assert!(matches!(result, Err(AggregationError::MessageMismatch(_))));
    }

    #[test]
    fn test_aggregator_mismatched_messages() {
        let mut rng = crate::rng::CryptoOsRng::new();
        let kp1 = SigningKey::generate(&mut rng);
        let kp2 = SigningKey::generate(&mut rng);

        let msg1 = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);
        let msg2 = AuthMessage::with_values("node2", "def456", 1000u64, 42u64, 1u64);

        let mut agg = Aggregator::new(
            vec!["node1".to_string(), "node2".to_string()],
            2,
            Duration::from_secs(30),
        );

        let signed1 = create_signed_message("node1", &msg1, &kp1);
        agg.add_signature("node1", signed1).unwrap();

        let signed2 = create_signed_message("node2", &msg2, &kp2);
        let result = agg.add_signature("node2", signed2);
        assert!(matches!(result, Err(AggregationError::MessageMismatch(_))));
    }

    #[test]
    fn test_aggregator_threshold() {
        let mut rng = crate::rng::CryptoOsRng::new();
        let keypairs: Vec<SigningKey> = (0..5).map(|_| SigningKey::generate(&mut rng)).collect();
        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);

        let mut agg = Aggregator::new(
            vec!["n0".to_string(), "n1".to_string(), "n2".to_string(), "n3".to_string(), "n4".to_string()],
            3,
            Duration::from_secs(30),
        );

        assert!(!agg.has_threshold());

        for i in 0..3 {
            let signed = create_signed_message(&format!("n{}", i), &msg, &keypairs[i]);
            agg.add_signature(&format!("n{}", i), signed).unwrap();
        }

        assert!(agg.has_threshold());
    }

    #[test]
    fn test_aggregator_create_aggregate_success() {
        let mut rng = crate::rng::CryptoOsRng::new();
        let keypairs: Vec<SigningKey> = (0..5).map(|_| SigningKey::generate(&mut rng)).collect();
        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);

        let mut agg = Aggregator::new(
            vec!["n0".to_string(), "n1".to_string(), "n2".to_string(), "n3".to_string(), "n4".to_string()],
            3,
            Duration::from_secs(30),
        );

        for i in 0..3 {
            let signed = create_signed_message(&format!("n{}", i), &msg, &keypairs[i]);
            agg.add_signature(&format!("n{}", i), signed).unwrap();
        }

        let aggregate = agg.create_aggregate();
        assert!(aggregate.is_ok());
        let aggregate = aggregate.unwrap();
        assert_eq!(aggregate.threshold, 3);
        assert_eq!(aggregate.total_nodes, 5);
    }

    #[test]
    fn test_aggregator_create_aggregate_insufficient() {
        let mut rng = crate::rng::CryptoOsRng::new();
        let kp = SigningKey::generate(&mut rng);
        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);

        let mut agg = Aggregator::new(
            vec!["n0".to_string(), "n1".to_string(), "n2".to_string(), "n3".to_string(), "n4".to_string()],
            3,
            Duration::from_secs(30),
        );

        let signed = create_signed_message("n0", &msg, &kp);
        agg.add_signature("n0", signed).unwrap();

        let result = agg.create_aggregate();
        assert!(matches!(result, Err(AggregationError::ThresholdNotReached(1, 3))));
    }

   #[test]
    fn test_aggregator_timeout() {
        let agg = Aggregator::new(
            vec!["n0".to_string(), "n1".to_string()],
            2,
            Duration::from_millis(10),
        );

        std::thread::sleep(Duration::from_millis(15));

        assert!(agg.is_timed_out());
    }

    #[test]
    fn test_aggregator_elapsed() {
        let agg = Aggregator::new(
            vec!["n0".to_string()],
            1,
            Duration::from_secs(30),
        );

        std::thread::sleep(Duration::from_millis(5));

        let elapsed = agg.elapsed();
        assert!(elapsed.as_millis() >= 5);
    }

    #[test]
    fn test_aggregator_missing_nodes() {
        let mut rng = crate::rng::CryptoOsRng::new();
        let kp = SigningKey::generate(&mut rng);
        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);

        let mut agg = Aggregator::new(
            vec!["n0".to_string(), "n1".to_string(), "n2".to_string()],
            2,
            Duration::from_secs(30),
        );

        let mut missing = agg.missing_nodes();
        missing.sort();
        assert_eq!(missing, vec!["n0".to_string(), "n1".to_string(), "n2".to_string()]);

        let signed = create_signed_message("n0", &msg, &kp);
        agg.add_signature("n0", signed).unwrap();

        missing = agg.missing_nodes();
        missing.sort();
        assert_eq!(missing, vec!["n1".to_string(), "n2".to_string()]);
    }

    #[test]
    fn test_aggregator_missing_nodes_empty() {
        let mut rng = crate::rng::CryptoOsRng::new();
        let kp = SigningKey::generate(&mut rng);
        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);

        let mut agg = Aggregator::new(
            vec!["n0".to_string(), "n1".to_string()],
            2,
            Duration::from_secs(30),
        );

        let signed1 = create_signed_message("n0", &msg, &kp);
        agg.add_signature("n0", signed1).unwrap();

        let signed2 = create_signed_message("n1", &msg, &kp);
        agg.add_signature("n1", signed2).unwrap();

        assert!(agg.missing_nodes().is_empty());
    }

    #[test]
    fn test_aggregator_threshold_3_of_5() {
        let mut rng = crate::rng::CryptoOsRng::new();
        let keypairs: Vec<SigningKey> = (0..5).map(|_| SigningKey::generate(&mut rng)).collect();
        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);

        let mut agg = Aggregator::new(
            vec!["n0".to_string(), "n1".to_string(), "n2".to_string(), "n3".to_string(), "n4".to_string()],
            3,
            Duration::from_secs(30),
        );

        for i in 0..5 {
            let signed = create_signed_message(&format!("n{}", i), &msg, &keypairs[i]);
            agg.add_signature(&format!("n{}", i), signed).unwrap();
        }

        assert!(agg.has_threshold());
        assert_eq!(agg.collected_count(), 5);

        let aggregate = agg.create_aggregate().unwrap();
        assert_eq!(aggregate.threshold, 3);
        assert_eq!(aggregate.total_nodes, 5);
        assert_eq!(aggregate.node_ids.len(), 5);
    }

    #[test]
    fn test_aggregator_threshold_2_of_3() {
        let mut rng = crate::rng::CryptoOsRng::new();
        let keypairs: Vec<SigningKey> = (0..3).map(|_| SigningKey::generate(&mut rng)).collect();
        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);

        let mut agg = Aggregator::new(
            vec!["n0".to_string(), "n1".to_string(), "n2".to_string()],
            2,
            Duration::from_secs(30),
        );

        for i in 0..2 {
            let signed = create_signed_message(&format!("n{}", i), &msg, &keypairs[i]);
            agg.add_signature(&format!("n{}", i), signed).unwrap();
        }

        assert!(agg.has_threshold());
        assert_eq!(agg.collected_count(), 2);
    }

    #[test]
    fn test_aggregator_empty_expected_nodes() {
        let agg = Aggregator::new(
            vec![],
            0,
            Duration::from_secs(30),
        );

        assert_eq!(agg.threshold, 0);
        assert_eq!(agg.total_nodes, 0);
        assert!(agg.missing_nodes().is_empty());
    }

    #[test]
    fn test_aggregator_collected_count() {
        let mut rng = crate::rng::CryptoOsRng::new();
        let kp = SigningKey::generate(&mut rng);
        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);

        let mut agg = Aggregator::new(
            vec!["n0".to_string(), "n1".to_string(), "n2".to_string()],
            2,
            Duration::from_secs(30),
        );

        assert_eq!(agg.collected_count(), 0);

        let signed = create_signed_message("n0", &msg, &kp);
        agg.add_signature("n0", signed).unwrap();
        assert_eq!(agg.collected_count(), 1);

        let signed = create_signed_message("n1", &msg, &kp);
        agg.add_signature("n1", signed).unwrap();
        assert_eq!(agg.collected_count(), 2);
    }
}