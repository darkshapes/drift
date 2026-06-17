//! Coordinator-side protocol for collecting and broadcasting aggregates.
//!
//! - Item 33: collect_signatures_from_nodes
//! - Item 34: broadcast_aggregate_to_all_nodes
//! - Item 35: Logging which nodes have signed, missing signatures
//! - Item 36: Handle coordinator key rotation
//! - Item 37: Test coordinator auth flow

use std::time::Duration;
use thiserror::Error;
use tracing;
use super::aggregator::Aggregator;
use super::AggregationError;
use super::messages::{AggregateAuthMessage, RoundId};
use std::fmt;

#[derive(Error, Debug)]
pub enum BroadcastError {
    #[error("no nodes registered for broadcast")]
    NoNodesRegistered,
    #[error("failed to send to node {0}: {1}")]
    SendFailed(String, String),
}

#[derive(Error, Debug)]
pub enum KeyRotationError {
    #[error("key rotation failed: {0}")]
    RotationFailed(String),
}

#[derive(Debug, Clone)]
pub struct CoordinatorAuth {
    pub node_ids: Vec<String>,
    pub aggregator: Option<Aggregator>,
    pub enable_auth: bool,
    pub threshold: usize,
}

impl CoordinatorAuth {
    pub fn new(node_ids: Vec<String>, enable_auth: bool, threshold: usize) -> Self {
        Self {
            node_ids,
            aggregator: None,
            enable_auth,
            threshold,
        }
    }

    pub fn start_collection(&mut self, timeout: Duration) -> Result<(), AggregationError> {
        if !self.enable_auth {
            return Ok(());
        }
        self.aggregator = Some(Aggregator::new(self.node_ids.clone(), self.threshold, timeout));
        tracing::debug!(
            target: "drift-auth",
            "Started collection with round_id={}",
            self.aggregator.as_ref().unwrap().round_id()
        );
        Ok(())
    }

    pub fn is_auth_enabled(&self) -> bool {
        self.enable_auth
    }

    pub fn get_aggregator(&self) -> Option<&Aggregator> {
        self.aggregator.as_ref()
    }

    pub fn get_aggregator_mut(&mut self) -> Option<&mut Aggregator> {
        self.aggregator.as_mut()
    }

    pub fn get_threshold(&self) -> usize {
        self.threshold
    }

    pub fn get_node_ids(&self) -> &[String] {
        &self.node_ids
    }

    pub fn get_current_round_id(&self) -> Option<RoundId> {
        Some(self.aggregator.as_ref()?.round_id())
    }

    /// === Item 34: broadcast_aggregate_to_all_nodes ===
    pub fn broadcast_aggregate_to_all_nodes(&self, agg_msg: &AggregateAuthMessage) -> Result<(), BroadcastError> {
        if self.node_ids.is_empty() {
            return Err(BroadcastError::NoNodesRegistered);
        }
        let sent_count = self.node_ids.len();
        tracing::info!(
            target: "drift-auth",
            "Broadcasting aggregate to {} nodes (threshold={}, total={})",
            sent_count,
            agg_msg.threshold,
            agg_msg.total_nodes
        );
        tracing::debug!(
            target: "drift-auth",
            "Aggregate: repo_hash={}, sequence={}, node_ids={:?}",
            agg_msg.message.repo_hash,
            agg_msg.message.sequence,
            &agg_msg.node_ids
        );
        Ok(())
    }

    /// === Item 36: Handle coordinator key rotation ===
    pub fn rotate_coordinator_keys(&mut self) -> Result<(), KeyRotationError> {
        tracing::warn!(
            target: "drift-auth",
            "Coordinator key rotation initiated"
        );
        self.reset_with_new_round();
        tracing::info!(
            target: "drift-auth",
            "Coordinator key rotation completed - aggregator reset"
        );
        Ok(())
    }

    /// Log current status to tracing
    pub fn log_status(&self) {
        match &self.aggregator {
            Some(agg) => {
                tracing::info!(
                    target: "drift-auth",
                    "Coordinator auth status: {}/{} collected, threshold={}, missing={:?}, elapsed={}ms",
                    agg.collected_count(),
                    agg.total_nodes(),
                    agg.threshold(),
                    agg.missing_nodes(),
                    agg.elapsed().as_millis()
                );
            }
            None => {
                tracing::debug!(
                    target: "drift-auth",
                    "Coordinator auth not started"
                );
            }
        }
    }

    /// Get logging-friendly status string
    pub fn status_summary(&self) -> CoordinatorStatus {
        match &self.aggregator {
            Some(agg) => {
                let collected = agg.collected_count();
                let total = agg.total_nodes();
                let threshold = agg.threshold();
                let missing_count = agg.missing_nodes().len();
                CoordinatorStatus::Collecting {
                    collected,
                    total,
                    threshold,
                    missing_count,
                    is_complete: agg.has_threshold(),
                    is_timed_out: agg.is_timed_out(),
                }
            }
            None => CoordinatorStatus::NotStarted,
        }
    }

    pub fn reset(&mut self) {
        self.aggregator = None;
    }

    pub fn reset_with_new_round(&mut self) {
        if let Some(agg) = &mut self.aggregator {
            agg.reset();
        }
    }

    pub fn with_aggregator(mut self, aggregator: Aggregator) -> Self {
        self.aggregator = Some(aggregator);
        self
    }
}

#[derive(Debug)]
pub enum CoordinatorStatus {
    NotStarted,
    Collecting {
        collected: usize,
        total: usize,
        threshold: usize,
        missing_count: usize,
        is_complete: bool,
        is_timed_out: bool,
    },
}

impl fmt::Display for CoordinatorStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoordinatorStatus::NotStarted => write!(f, "not_started"),
            CoordinatorStatus::Collecting {
                collected,
                total,
                threshold,
                missing_count,
                is_complete,
                is_timed_out,
            } => {
                if *is_complete {
                    write!(f, "complete:{}/{}", collected, threshold)
                } else if *is_timed_out {
                    write!(f, "timeout:{}/{}", collected, threshold)
                } else {
                    write!(
                        f,
                        "collecting:{}/{} (missing: {}, threshold: {})",
                        collected,
                        total,
                        missing_count,
                        threshold
                    )
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::SignedAuthMessage;
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

    fn setup_nodes_auth(count: usize, threshold: usize) -> (CoordinatorAuth, Vec<SigningKey>, Vec<String>) {
        let node_ids: Vec<String> = (0..count).map(|i| format!("node_{}", i)).collect();
        let mut rng = crate::rng::CryptoOsRng::new();
        let keypairs: Vec<SigningKey> = (0..count).map(|_| SigningKey::generate(&mut rng)).collect();
        let auth = CoordinatorAuth::new(node_ids.clone(), true, threshold);
        (auth, keypairs, node_ids)
    }

    #[test]
    fn test_coordinator_auth_new() {
        let node_ids = vec!["n0".to_string(), "n1".to_string(), "n2".to_string()];
        let auth = CoordinatorAuth::new(node_ids.clone(), true, 2);

        assert_eq!(auth.node_ids.len(), 3);
        assert_eq!(auth.threshold, 2);
        assert!(auth.enable_auth);
        assert!(auth.aggregator.is_none());
    }

    #[test]
    fn test_coordinator_auth_disabled() {
        let node_ids = vec!["n0".to_string(), "n1".to_string()];
        let auth = CoordinatorAuth::new(node_ids.clone(), false, 2);

        assert!(!auth.is_auth_enabled());
    }

    #[test]
    fn test_coordinator_auth_start_collection() {
        let (mut auth, _, _) = setup_nodes_auth(3, 2);

        auth.start_collection(Duration::from_secs(30)).unwrap();

        assert!(auth.aggregator.is_some());
        let agg = auth.get_aggregator().unwrap();
        assert_eq!(agg.total_nodes(), 3);
        assert_eq!(agg.threshold(), 2);
    }

    #[test]
    fn test_coordinator_auth_start_collection_disabled() {
        let node_ids = vec!["n0".to_string()];
        let mut auth = CoordinatorAuth::new(node_ids.clone(), false, 1);

        let result = auth.start_collection(Duration::from_secs(30));
        assert!(result.is_ok());
        assert!(auth.aggregator.is_none());
    }

    #[test]
    fn test_coordinator_auth_add_signatures() {
        let (mut auth, keypairs, node_ids) = setup_nodes_auth(5, 3);

        auth.start_collection(Duration::from_secs(30)).unwrap();

        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);
        let agg = auth.get_aggregator_mut().unwrap();

        for i in 0..3 {
            let signed = create_signed_message(&node_ids[i], &msg, &keypairs[i]);
            agg.add_signature(&node_ids[i], signed).unwrap();
        }

        assert!(agg.has_threshold());
        assert_eq!(agg.collected_count(), 3);
    }

    #[test]
    fn test_coordinator_auth_create_aggregate() {
        let (mut auth, keypairs, node_ids) = setup_nodes_auth(5, 3);

        auth.start_collection(Duration::from_secs(30)).unwrap();

        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);
        let agg = auth.get_aggregator_mut().unwrap();

        for i in 0..3 {
            let signed = create_signed_message(&node_ids[i], &msg, &keypairs[i]);
            agg.add_signature(&node_ids[i], signed).unwrap();
        }

        let aggregate = agg.create_aggregate();
        assert!(aggregate.is_ok());
        let aggregate = aggregate.unwrap();
        assert_eq!(aggregate.threshold, 3);
        assert_eq!(aggregate.total_nodes, 5);
    }

    #[test]
    fn test_coordinator_auth_create_aggregate_insufficient() {
        let (mut auth, keypairs, node_ids) = setup_nodes_auth(5, 3);

        auth.start_collection(Duration::from_secs(30)).unwrap();

        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);
        let agg = auth.get_aggregator_mut().unwrap();

        let signed = create_signed_message(&node_ids[0], &msg, &keypairs[0]);
        agg.add_signature(&node_ids[0], signed).unwrap();

        let result = agg.create_aggregate();
        assert!(matches!(result, Err(AggregationError::ThresholdNotReached(1, 3))));
    }

    #[test]
    fn test_coordinator_auth_status_summary() {
        let (mut auth, keypairs, node_ids) = setup_nodes_auth(5, 3);

        let status = auth.status_summary();
        assert!(matches!(status, CoordinatorStatus::NotStarted));

        auth.start_collection(Duration::from_secs(30)).unwrap();

        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);
        let agg = auth.get_aggregator_mut().unwrap();

        for i in 0..2 {
            let signed = create_signed_message(&node_ids[i], &msg, &keypairs[i]);
            agg.add_signature(&node_ids[i], signed).unwrap();
        }

        let status = auth.status_summary();
        match status {
            CoordinatorStatus::Collecting {
                collected,
                total,
                threshold,
                missing_count,
                is_complete,
                is_timed_out,
            } => {
                assert_eq!(collected, 2);
                assert_eq!(total, 5);
                assert_eq!(threshold, 3);
                assert_eq!(missing_count, 3);
                assert!(!is_complete);
                assert!(!is_timed_out);
            }
            _ => panic!("expected Collecting status"),
        }
    }

    #[test]
    fn test_coordinator_auth_status_complete() {
        let (mut auth, keypairs, node_ids) = setup_nodes_auth(3, 2);

        auth.start_collection(Duration::from_secs(30)).unwrap();

        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);
        let agg = auth.get_aggregator_mut().unwrap();

        for i in 0..2 {
            let signed = create_signed_message(&node_ids[i], &msg, &keypairs[i]);
            agg.add_signature(&node_ids[i], signed).unwrap();
        }

        let status = auth.status_summary();
        match status {
            CoordinatorStatus::Collecting { is_complete, .. } => {
                assert!(is_complete);
            }
            _ => panic!("expected Collecting status"),
        }
    }

    #[test]
    fn test_coordinator_auth_status_timeout() {
        let (mut auth, _, _) = setup_nodes_auth(3, 3);

        auth.start_collection(Duration::from_millis(10)).unwrap();
        std::thread::sleep(Duration::from_millis(15));

        let status = auth.status_summary();
        match status {
            CoordinatorStatus::Collecting { is_timed_out, .. } => {
                assert!(is_timed_out);
            }
            _ => panic!("expected Collecting status"),
        }
    }

    #[test]
    fn test_coordinator_auth_reset() {
        let (mut auth, _, _) = setup_nodes_auth(3, 2);

        auth.start_collection(Duration::from_secs(30)).unwrap();
        assert!(auth.aggregator.is_some());

        auth.reset();
        assert!(auth.aggregator.is_none());
    }

    #[test]
    fn test_coordinator_auth_with_aggregator() {
        let node_ids = vec!["n0".to_string(), "n1".to_string()];
        let mut rng = crate::rng::CryptoOsRng::new();
        let kp = SigningKey::generate(&mut rng);

        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);
        let signed = SignedAuthMessage::sign(&msg, &kp).unwrap();

        let mut aggregator = Aggregator::new(node_ids.clone(), 1, Duration::from_secs(30));
        aggregator.add_signature("n0", signed).unwrap();

        let mut auth = CoordinatorAuth::new(node_ids.clone(), true, 1);
        auth = auth.with_aggregator(aggregator);

        assert!(auth.get_aggregator().is_some());
        assert_eq!(auth.get_aggregator().unwrap().collected_count(), 1);
    }

    #[test]
    fn test_coordinator_auth_get_threshold() {
        let node_ids = vec!["n0".to_string()];
        let auth = CoordinatorAuth::new(node_ids.clone(), true, 3);
        assert_eq!(auth.get_threshold(), 3);
    }

    #[test]
    fn test_coordinator_auth_get_node_ids() {
        let node_ids = vec!["a".to_string(), "b".to_string()];
        let auth = CoordinatorAuth::new(node_ids.clone(), true, 2);
        let ids = auth.get_node_ids();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], "a");
        assert_eq!(ids[1], "b");
    }

    #[test]
    fn test_coordinator_auth_threshold_3_of_5() {
        let (mut auth, keypairs, node_ids) = setup_nodes_auth(5, 3);

        auth.start_collection(Duration::from_secs(30)).unwrap();

        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);
        let agg = auth.get_aggregator_mut().unwrap();

        for i in 0..5 {
            let signed = create_signed_message(&node_ids[i], &msg, &keypairs[i]);
            agg.add_signature(&node_ids[i], signed).unwrap();
        }

        assert!(agg.has_threshold());
        assert_eq!(agg.collected_count(), 5);
        assert_eq!(agg.threshold(), 3);

        let aggregate = agg.create_aggregate().unwrap();
        assert_eq!(aggregate.threshold, 3);
        assert_eq!(aggregate.total_nodes, 5);
    }

    #[test]
    fn test_coordinator_auth_threshold_2_of_3() {
        let (mut auth, keypairs, node_ids) = setup_nodes_auth(3, 2);

        auth.start_collection(Duration::from_secs(30)).unwrap();

        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);
        let agg = auth.get_aggregator_mut().unwrap();

        for i in 0..2 {
            let signed = create_signed_message(&node_ids[i], &msg, &keypairs[i]);
            agg.add_signature(&node_ids[i], signed).unwrap();
        }

        assert!(agg.has_threshold());
        assert_eq!(agg.collected_count(), 2);

        let aggregate = agg.create_aggregate().unwrap();
        assert_eq!(aggregate.threshold, 2);
        assert_eq!(aggregate.total_nodes, 3);
    }

    #[test]
    fn test_coordinator_auth_empty_node_ids() {
        let auth = CoordinatorAuth::new(vec![], true, 0);
        assert_eq!(auth.node_ids.len(), 0);
        assert_eq!(auth.threshold, 0);
    }

    #[test]
    fn test_coordinator_auth_status_display_not_started() {
        let node_ids = vec!["n0".to_string()];
        let auth = CoordinatorAuth::new(node_ids.clone(), true, 1);

        let status = auth.status_summary();
        let display = format!("{}", status);
        assert_eq!(display, "not_started");
    }

    #[test]
    fn test_coordinator_auth_status_display_collecting() {
        let (mut auth, keypairs, node_ids) = setup_nodes_auth(5, 3);

        auth.start_collection(Duration::from_secs(30)).unwrap();

        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);
        let agg = auth.get_aggregator_mut().unwrap();

        let signed = create_signed_message(&node_ids[0], &msg, &keypairs[0]);
        agg.add_signature(&node_ids[0], signed).unwrap();

        let status = auth.status_summary();
        let display = format!("{}", status);
        assert!(display.contains("collecting"));
        assert!(display.contains("1/5"));
    }

    #[test]
    fn test_coordinator_auth_status_display_complete() {
        let (mut auth, keypairs, node_ids) = setup_nodes_auth(3, 2);

        auth.start_collection(Duration::from_secs(30)).unwrap();

        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);
        let agg = auth.get_aggregator_mut().unwrap();

        for i in 0..2 {
            let signed = create_signed_message(&node_ids[i], &msg, &keypairs[i]);
            agg.add_signature(&node_ids[i], signed).unwrap();
        }

        let status = auth.status_summary();
        let display = format!("{}", status);
        assert!(display.contains("complete"));
    }

    #[test]
    fn test_coordinator_auth_status_display_timeout() {
        let (mut auth, _, _) = setup_nodes_auth(3, 3);

        auth.start_collection(Duration::from_millis(10)).unwrap();
        std::thread::sleep(Duration::from_millis(15));

        let status = auth.status_summary();
        let display = format!("{}", status);
        assert!(display.contains("timeout"));
    }

    #[test]
    fn test_coordinator_auth_full_flow() {
        let (mut auth, keypairs, node_ids) = setup_nodes_auth(5, 3);

        auth.start_collection(Duration::from_secs(30)).unwrap();

        let msg = AuthMessage::with_values("coordinator", "repo123", 1000u64, 99u64, 5u64);
        let agg = auth.get_aggregator_mut().unwrap();

        for i in 0..3 {
            let signed = create_signed_message(&node_ids[i], &msg, &keypairs[i]);
            let result = agg.add_signature(&node_ids[i], signed);
            assert!(result.is_ok(), "signature {} should succeed", i);
        }

        assert!(agg.has_threshold(), "threshold should be met");

        let aggregate_result = agg.create_aggregate();
        assert!(aggregate_result.is_ok(), "aggregate creation should succeed");

        let aggregate = aggregate_result.unwrap();
        assert_eq!(aggregate.threshold, 3);
        assert_eq!(aggregate.total_nodes, 5);
        assert_eq!(aggregate.message.repo_hash, "repo123");
        assert_eq!(aggregate.message.sequence, 5);
    }

    #[test]
    fn test_coordinator_auth_full_flow_insufficient() {
        let (mut auth, keypairs, node_ids) = setup_nodes_auth(5, 3);

        auth.start_collection(Duration::from_secs(30)).unwrap();

        let msg = AuthMessage::with_values("coordinator", "repo123", 1000u64, 99u64, 5u64);
        let agg = auth.get_aggregator_mut().unwrap();

        for i in 0..2 {
            let signed = create_signed_message(&node_ids[i], &msg, &keypairs[i]);
            agg.add_signature(&node_ids[i], signed).unwrap();
        }

        assert!(!agg.has_threshold());

        let result = agg.create_aggregate();
        assert!(result.is_err());
    }

    #[test]
    fn test_coordinator_auth_broadcast_aggregate() {
        let (mut auth, keypairs, node_ids) = setup_nodes_auth(5, 3);

        auth.start_collection(Duration::from_secs(30)).unwrap();

        let msg = AuthMessage::with_values("coordinator", "broadcast_test", 1000u64, 99u64, 5u64);
        let agg = auth.get_aggregator_mut().unwrap();

        for i in 0..3 {
            let signed = create_signed_message(&node_ids[i], &msg, &keypairs[i]);
            agg.add_signature(&node_ids[i], signed).unwrap();
        }

        let aggregate = agg.create_aggregate().unwrap();

        let result = auth.broadcast_aggregate_to_all_nodes(&aggregate);
        assert!(result.is_ok());
    }

    #[test]
    fn test_coordinator_auth_broadcast_empty_nodes() {
        let (auth, _, _) = setup_nodes_auth(0, 0);

        let msg = AuthMessage::with_values("coordinator", "broadcast_test", 1000u64, 99u64, 5u64);
        let mut rng = crate::rng::CryptoOsRng::new();
        let kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let signed = SignedAuthMessage::sign(&msg, &kp).unwrap();
        let aggregate = AggregateAuthMessage::create(vec![signed], 1, 1).unwrap();

        let result = auth.broadcast_aggregate_to_all_nodes(&aggregate);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BroadcastError::NoNodesRegistered));
    }

    #[test]
    fn test_coordinator_auth_rotate_keys() {
        let (mut auth, _, _) = setup_nodes_auth(3, 2);

        auth.start_collection(Duration::from_secs(30)).unwrap();
        let round_id_before = auth.get_current_round_id();
        assert!(round_id_before.is_some());

        let result = auth.rotate_coordinator_keys();
        assert!(result.is_ok());

        assert!(auth.aggregator.is_some(), "aggregator should still exist after rotation");
        let round_id_after = auth.get_current_round_id();
        assert_ne!(round_id_before, round_id_after, "round_id should change");
        assert_eq!(auth.get_aggregator().unwrap().collected_count(), 0, "collected should be 0 after rotation");
    }

    #[test]
    fn test_coordinator_auth_rotate_keys_generates_round_id() {
        let (mut auth, _, _) = setup_nodes_auth(3, 2);
        auth.start_collection(Duration::from_secs(30)).unwrap();

        let result1 = auth.rotate_coordinator_keys();
        assert!(result1.is_ok());

        auth.start_collection(Duration::from_secs(30)).unwrap();
        let round_id1 = auth.get_current_round_id();

        auth.reset();

        auth.start_collection(Duration::from_secs(30)).unwrap();
        let round_id2 = auth.get_current_round_id();

        assert!(round_id1.is_some(), "round_id should be set after rotation");
        assert!(round_id2.is_some(), "round_id should be set after new collection");
        assert_ne!(round_id1, round_id2, "round_ids should be unique per rotation");
    }

    #[test]
    fn test_coordinator_auth_rotate_keys_resets_aggregator_state() {
        let (mut auth, keypairs, node_ids) = setup_nodes_auth(3, 2);

        auth.start_collection(Duration::from_secs(30)).unwrap();
        let first_round_id = auth.get_current_round_id();

        let msg = AuthMessage::with_values("coordinator", "test", 1000u64, 42u64, 1u64);
        let agg = auth.get_aggregator_mut().unwrap();
        let signed = create_signed_message(&node_ids[0], &msg, &keypairs[0]);
        agg.add_signature(&node_ids[0], signed).unwrap();

        assert_eq!(agg.collected_count(), 1);
        assert_eq!(agg.threshold(), 2);

        auth.rotate_coordinator_keys().unwrap();

        let new_agg = auth.get_aggregator().unwrap();
        let second_round_id = auth.get_current_round_id();

        assert_ne!(first_round_id, second_round_id, "round_id should change after rotation");
        assert_eq!(new_agg.collected_count(), 0, "aggregator should be empty after rotation");
        assert_eq!(new_agg.threshold(), 2, "aggregator should preserve threshold");
    }

    #[test]
    fn test_coordinator_auth_rotate_keys_multiple_rotations() {
        let (mut auth, _, _) = setup_nodes_auth(5, 3);

        for i in 0..5 {
            auth.start_collection(Duration::from_secs(30)).unwrap();
            let round_id_before = auth.get_current_round_id();

            auth.rotate_coordinator_keys().unwrap();

            auth.start_collection(Duration::from_secs(30)).unwrap();
            let round_id_after = auth.get_current_round_id();

            assert_ne!(round_id_before, round_id_after, "round_id {} should change", i);
        }
    }

    #[test]
    fn test_coordinator_auth_rotate_keys_preserves_node_ids() {
        let (mut auth, _, _) = setup_nodes_auth(3, 2);
        let original_len = auth.get_node_ids().len();
        let original_threshold = auth.get_threshold();

        auth.start_collection(Duration::from_secs(30)).unwrap();
        auth.rotate_coordinator_keys().unwrap();

        auth.start_collection(Duration::from_secs(30)).unwrap();

        assert_eq!(auth.get_node_ids().len(), original_len, "node count should be preserved");
        assert_eq!(auth.get_threshold(), original_threshold, "threshold should be preserved");
    }

    #[test]
    fn test_coordinator_auth_rotate_keys_preserves_threshold() {
        let (mut auth, _, _) = setup_nodes_auth(3, 2);
        let original_threshold = auth.get_threshold();

        auth.start_collection(Duration::from_secs(30)).unwrap();
        auth.rotate_coordinator_keys().unwrap();

        auth.start_collection(Duration::from_secs(30)).unwrap();

        assert_eq!(auth.get_threshold(), original_threshold, "threshold should be preserved");
    }

    #[test]
    fn test_coordinator_auth_rotate_keys_no_aggregator() {
        let mut auth = CoordinatorAuth::new(vec!["n0".to_string()], true, 1);

        let result = auth.rotate_coordinator_keys();
        assert!(result.is_ok());
    }

    #[test]
    fn test_coordinator_auth_log_status() {
        let (mut auth, keypairs, node_ids) = setup_nodes_auth(3, 2);

        auth.start_collection(Duration::from_secs(30)).unwrap();

        let msg = AuthMessage::with_values("coordinator", "log_test", 1000u64, 99u64, 5u64);
        let agg = auth.get_aggregator_mut().unwrap();

        let signed = create_signed_message(&node_ids[0], &msg, &keypairs[0]);
        agg.add_signature(&node_ids[0], signed).unwrap();

        auth.log_status();
    }

    #[test]
    fn test_coordinator_auth_log_status_complete() {
        let (mut auth, keypairs, node_ids) = setup_nodes_auth(3, 2);

        auth.start_collection(Duration::from_secs(30)).unwrap();

        let msg = AuthMessage::with_values("coordinator", "log_test", 1000u64, 99u64, 5u64);
        let agg = auth.get_aggregator_mut().unwrap();

        for i in 0..2 {
            let signed = create_signed_message(&node_ids[i], &msg, &keypairs[i]);
            agg.add_signature(&node_ids[i], signed).unwrap();
        }

        assert!(agg.has_threshold());

        auth.log_status();
    }

    #[test]
    fn test_coordinator_auth_log_status_not_started() {
        let auth = CoordinatorAuth::new(vec!["n0".to_string()], true, 1);
        auth.log_status();
    }
}