//! Coordinator-side authentication protocol for multi-signature consensus.
//!
//! Items 33-37: Coordinator protocol for collecting and broadcasting aggregates

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use anyhow::{Context, Result};
use thiserror::Error;

use drift_proto::{DriftMessage, TrainConfig};
use drift_auth::{Aggregator, AggregateAuthMessage, AggregationError};

#[derive(Debug, Error)]
pub enum CoordinatorAuthError {
    #[error("timeout waiting for signatures")]
    Timeout,
    #[error("auth disabled in train config")]
    AuthDisabled,
    #[error("no nodes registered")]
    NoNodesRegistered,
    #[error("broadcast failed: {0}")]
    BroadcastFailed(String),
    #[error("aggregation failed: {0}")]
    AggregationFailed(#[from] AggregationError),
}

pub struct CoordinatorAuth {
    node_channels: HashMap<String, mpsc::Sender<DriftMessage>>,
    aggregator: Option<Aggregator>,
    train_config: TrainConfig,
}

impl CoordinatorAuth {
    pub fn new(train_config: TrainConfig) -> Self {
        Self {
            node_channels: HashMap::new(),
            aggregator: None,
            train_config,
        }
    }

    pub fn register_node(&mut self, node_id: String, tx: mpsc::Sender<DriftMessage>) {
        self.node_channels.insert(node_id, tx);
    }

    pub fn registered_node_count(&self) -> usize {
        self.node_channels.len()
    }

    pub fn get_train_config(&self) -> &TrainConfig {
        &self.train_config
    }

    pub fn is_auth_enabled(&self) -> bool {
        self.train_config.enable_auth
    }

    pub fn get_threshold(&self) -> usize {
        self.train_config.auth_threshold
    }

    pub fn expected_nodes(&self) -> Vec<String> {
        self.node_channels.keys().map(|s| s.clone()).collect()
    }

    pub fn get_aggregator(&self) -> Option<&Aggregator> {
        self.aggregator.as_ref()
    }

    pub fn has_aggregator(&self) -> bool {
        self.aggregator.is_some()
    }

    pub async fn collect_signatures(
        &mut self,
        expected_nodes: Vec<String>,
        timeout: Duration,
    ) -> Result<AggregateAuthMessage, CoordinatorAuthError> {
        if !self.train_config.enable_auth {
            return Err(CoordinatorAuthError::AuthDisabled);
        }

        if expected_nodes.is_empty() && self.node_channels.is_empty() {
            return Err(CoordinatorAuthError::NoNodesRegistered);
        }

        let threshold = self.train_config.auth_threshold;
        let nodes_to_collect: Vec<String> = if expected_nodes.is_empty() {
            self.node_channels.keys().map(|s| s.clone()).collect()
        } else {
            expected_nodes
        };

        let aggregator = Aggregator::new(nodes_to_collect, threshold, timeout);
        self.aggregator = Some(aggregator);
        let agg_ref = self.aggregator.as_mut().unwrap();

        let start = Instant::now();
        loop {
            if agg_ref.is_timed_out() {
                return Err(CoordinatorAuthError::Timeout);
            }

            if agg_ref.has_threshold() {
                break;
            }

            if start.elapsed() < Duration::from_millis(10) {
                tokio::time::sleep(Duration::from_millis(10)).await;
            } else {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        let agg_msg = agg_ref.create_aggregate()
            .map_err(CoordinatorAuthError::AggregationFailed)?;

        Ok(agg_msg)
    }

    pub async fn broadcast_aggregate(
        &self,
        agg_msg: &AggregateAuthMessage,
    ) -> Result<(), CoordinatorAuthError> {
        if !self.train_config.enable_auth {
            return Err(CoordinatorAuthError::AuthDisabled);
        }

        if self.node_channels.is_empty() {
            return Err(CoordinatorAuthError::NoNodesRegistered);
        }

        for (node_id, tx) in self.node_channels.iter() {
            let msg = DriftMessage::AuthAggregate(agg_msg.clone());
            tx.send(msg).await
                .context(format!("failed to send to node {}", node_id))
                .map_err(|e| CoordinatorAuthError::BroadcastFailed(e.to_string()))?;
        }

        Ok(())
    }

    pub fn log_status(&self) {
        if let Some(agg) = &self.aggregator {
            println!("Auth status: {}/{} signatures collected, need {}",
           agg.collected_count(),
                agg.total_nodes(),
                agg.threshold(),
        );

            let missing: Vec<String> = agg.missing_nodes();
            if !missing.is_empty() {
                println!("  Missing from: {:?}", missing);
            }
        } else if self.train_config.enable_auth {
            println!("Auth status: {} nodes registered, threshold {}",
            self.node_channels.len(),
            self.train_config.auth_threshold,
        );
        } else {
            println!("Auth status: disabled");
        }
    }

    pub fn log_status_string(&self) -> String {
        let mut lines = Vec::new();

        if let Some(agg) = &self.aggregator {
            lines.push(format!(
                "Auth status: {}/{} signatures collected, need {}",
                agg.collected_count(),
                agg.total_nodes(),
                agg.threshold()
            ));

            let missing = agg.missing_nodes();
            if !missing.is_empty() {
                lines.push(format!("  Missing from: {:?}", missing));
            }
        } else if self.train_config.enable_auth {
            lines.push(format!(
                "Auth status: {} nodes registered, threshold {}",
                self.node_channels.len(),
                self.train_config.auth_threshold
            ));
        } else {
            lines.push("Auth status: disabled".to_string());
        }

        lines.join("\n")
    }

    pub fn rotate_coordinator_keys(&mut self) -> Result<()> {
        Err(anyhow::anyhow!("coordinator key rotation not yet implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drift_auth::AuthMessage;
    use drift_auth::SignedAuthMessage;
    use ed25519_dalek::SigningKey;
    use std::time::Duration;

    fn create_signed_message(
        node_id: &str,
        msg: &AuthMessage,
        keypair: &SigningKey,
    ) -> SignedAuthMessage {
        SignedAuthMessage::sign(msg, keypair).unwrap()
    }

    fn create_train_config(enable_auth: bool, auth_threshold: usize) -> TrainConfig {
        TrainConfig {
            enable_auth,
            auth_threshold,
            model_path: "/tmp/model".to_string(),
            dataset_path: "/tmp/dataset".to_string(),
            batch_size: 32,
            learning_rate: 0.001,
            epochs: 10,
            train_repo_url: Some("https://github.com/example/train".to_string()),
            script_entrypoint: Some("train.py".to_string()),
            dataset_repo_url: None,
            model_artifact_ref: None,
        dataset_urls: vec![],
        git_commit: None,
        gpu_compute_capability: None,
        repo_path: None,
        training_spawn_cmd: None,
        }
    }

    #[test]
    fn test_coordinator_auth_new() {
        let config = create_train_config(true, 2);
        let auth = CoordinatorAuth::new(config);

        assert_eq!(auth.registered_node_count(), 0);
        assert!(auth.is_auth_enabled());
        assert_eq!(auth.get_threshold(), 2);
        assert!(!auth.has_aggregator());
    }

    #[test]
    fn test_coordinator_auth_disabled() {
        let config = create_train_config(false, 2);
        let auth = CoordinatorAuth::new(config);

        assert!(!auth.is_auth_enabled());
    }

    #[test]
    fn test_coordinator_auth_register_node() {
        let config = create_train_config(true, 2);
        let mut auth = CoordinatorAuth::new(config);

        let (tx, _rx) = mpsc::channel(16);
        auth.register_node("node1".to_string(), tx);

        assert_eq!(auth.registered_node_count(), 1);
        assert!(auth.expected_nodes().contains(&"node1".to_string()));
    }

    #[test]
    fn test_coordinator_auth_register_multiple_nodes() {
        let config = create_train_config(true, 3);
        let mut auth = CoordinatorAuth::new(config);

        let (tx1, _rx1) = mpsc::channel(16);
        let (tx2, _rx2) = mpsc::channel(16);
        let (tx3, _rx3) = mpsc::channel(16);

        auth.register_node("node1".to_string(), tx1);
        auth.register_node("node2".to_string(), tx2);
        auth.register_node("node3".to_string(), tx3);

        assert_eq!(auth.registered_node_count(), 3);
        assert_eq!(auth.get_threshold(), 3);
    }

    #[test]
    fn test_coordinator_auth_get_train_config() {
        let config = create_train_config(true, 3);
        let auth = CoordinatorAuth::new(config);

        let retrieved = auth.get_train_config();
        assert_eq!(retrieved.auth_threshold, 3);
        assert!(retrieved.enable_auth);
    }

    #[tokio::test]
    async fn test_coordinator_auth_collect_signatures_disabled() {
        let config = create_train_config(false, 2);
        let mut auth = CoordinatorAuth::new(config);

        let result = auth.collect_signatures(vec![], Duration::from_secs(30)).await;

        assert!(matches!(result, Err(CoordinatorAuthError::AuthDisabled)));
    }

    #[tokio::test]
    async fn test_coordinator_auth_collect_signatures_no_nodes() {
        let config = create_train_config(true, 2);
        let mut auth = CoordinatorAuth::new(config);

        let result = auth.collect_signatures(vec![], Duration::from_secs(30)).await;

        assert!(matches!(result, Err(CoordinatorAuthError::NoNodesRegistered)));
    }

    #[tokio::test]
    async fn test_coordinator_auth_collect_signatures_timeout() {
        let config = create_train_config(true, 2);
        let mut auth = CoordinatorAuth::new(config);

        let (tx1, _rx1) = mpsc::channel(16);
        auth.register_node("node1".to_string(), tx1);

        let result = auth.collect_signatures(
            vec!["node1".to_string(), "node2".to_string()],
            Duration::from_millis(50),
        ).await;

        assert!(matches!(result, Err(CoordinatorAuthError::Timeout)));
    }

    #[tokio::test]
    async fn test_coordinator_auth_broadcast_disabled() {
        let config = create_train_config(false, 2);
        let auth = CoordinatorAuth::new(config);

        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);
        let mut rng = drift_auth::CryptoOsRng::new();
        let kp = SigningKey::generate(&mut rng);
        let signed = create_signed_message("coordinator", &msg, &kp);

        let agg = AggregateAuthMessage::create(vec![signed], 1, 1).unwrap();
        let result = auth.broadcast_aggregate(&agg).await;

        assert!(matches!(result, Err(CoordinatorAuthError::AuthDisabled)));
    }

    #[tokio::test]
    async fn test_coordinator_auth_broadcast_no_nodes() {
        let config = create_train_config(true, 2);
        let auth = CoordinatorAuth::new(config);

        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);
        let mut rng = drift_auth::CryptoOsRng::new();
        let kp = SigningKey::generate(&mut rng);
        let signed = create_signed_message("coordinator", &msg, &kp);

        let agg = AggregateAuthMessage::create(vec![signed], 1, 1).unwrap();
        let result = auth.broadcast_aggregate(&agg).await;

        assert!(matches!(result, Err(CoordinatorAuthError::NoNodesRegistered)));
    }

    #[tokio::test]
    async fn test_coordinator_auth_broadcast_success() {
        let config = create_train_config(true, 2);
        let mut auth = CoordinatorAuth::new(config);

        let (tx1, mut rx1) = mpsc::channel(16);
        let (tx2, mut rx2) = mpsc::channel(16);

        auth.register_node("node1".to_string(), tx1);
        auth.register_node("node2".to_string(), tx2);

        let mut rng = drift_auth::CryptoOsRng::new();
        let kp1 = SigningKey::generate(&mut rng);
        let kp2 = SigningKey::generate(&mut rng);

        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);
        let signed1 = create_signed_message("node1", &msg, &kp1);
        let signed2 = create_signed_message("node2", &msg, &kp2);

        let agg = AggregateAuthMessage::create(vec![signed1, signed2], 2, 2).unwrap();

        let result = auth.broadcast_aggregate(&agg).await;
        assert!(result.is_ok(), "broadcast should succeed");

        let msg1 = rx1.recv().await.unwrap();
        assert!(matches!(msg1, DriftMessage::AuthAggregate(_)));

        let msg2 = rx2.recv().await.unwrap();
        assert!(matches!(msg2, DriftMessage::AuthAggregate(_)));
    }

    #[test]
    fn test_coordinator_auth_log_status_disabled() {
        let config = create_train_config(false, 2);
        let auth = CoordinatorAuth::new(config);
        let status = auth.log_status_string();
        assert!(status.contains("disabled"));
    }

    #[test]
    fn test_coordinator_auth_log_status_no_aggregator() {
        let config = create_train_config(true, 2);
        let mut auth = CoordinatorAuth::new(config);

        let (tx, _rx) = mpsc::channel(16);
        auth.register_node("node1".to_string(), tx);

        let status = auth.log_status_string();
        assert!(status.contains("1 nodes registered"));
        assert!(status.contains("threshold 2"));
    }

    #[test]
    fn test_coordinator_auth_log_status_with_aggregator() {
        let config = create_train_config(true, 3);
        let mut auth = CoordinatorAuth::new(config);

        let (tx1, _rx1) = mpsc::channel(16);
        let (tx2, _rx2) = mpsc::channel(16);
        let (tx3, _rx3) = mpsc::channel(16);
        let (tx4, _rx4) = mpsc::channel(16);
        let (tx5, _rx5) = mpsc::channel(16);

        auth.register_node("n0".to_string(), tx1);
        auth.register_node("n1".to_string(), tx2);
        auth.register_node("n2".to_string(), tx3);
        auth.register_node("n3".to_string(), tx4);
        auth.register_node("n4".to_string(), tx5);

        let mut rng = drift_auth::CryptoOsRng::new();
        let kp = SigningKey::generate(&mut rng);
        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);

        let signed = create_signed_message("n0", &msg, &kp);

        let agg_ref = auth.collect_signatures(
            vec!["n0".to_string(), "n1".to_string(), "n2".to_string()],
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_coordinator_auth_log_status_missing_nodes() {
        let config = create_train_config(true, 2);
        let mut auth = CoordinatorAuth::new(config);

        let (tx1, _rx1) = mpsc::channel(16);
        let (tx2, _rx2) = mpsc::channel(16);
        let (tx3, _rx3) = mpsc::channel(16);

        auth.register_node("n0".to_string(), tx1);
        auth.register_node("n1".to_string(), tx2);
        auth.register_node("n2".to_string(), tx3);

        let mut rng = drift_auth::CryptoOsRng::new();
        let kp = SigningKey::generate(&mut rng);
        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);
        let signed = create_signed_message("n0", &msg, &kp);

        auth.aggregator = Some(Aggregator::new(
            vec!["n0".to_string(), "n1".to_string(), "n2".to_string()],
            2,
            Duration::from_secs(30),
        ));
        auth.aggregator.as_mut().unwrap().add_signature("n0", signed).unwrap();

        let status = auth.log_status_string();
        assert!(status.contains("1/3 signatures"));
        assert!(status.contains("Missing"));
    }

    #[test]
    fn test_coordinator_auth_rotate_keys_not_implemented() {
        let config = create_train_config(true, 2);
        let mut auth = CoordinatorAuth::new(config);

        let result = auth.rotate_coordinator_keys();
        assert!(result.is_err());
    }

    #[test]
    fn test_coordinator_auth_multiple_registrations_same_node() {
        let config = create_train_config(true, 2);
        let mut auth = CoordinatorAuth::new(config);

        let (tx1, _rx1) = mpsc::channel(16);
        let (tx2, _rx2) = mpsc::channel(16);

        auth.register_node("node1".to_string(), tx1);
        auth.register_node("node1".to_string(), tx2);

        assert_eq!(auth.registered_node_count(), 1);
    }

    #[test]
    fn test_coordinator_auth_expected_nodes() {
        let config = create_train_config(true, 3);
        let mut auth = CoordinatorAuth::new(config);

        let (tx1, _rx1) = mpsc::channel(16);
        let (tx2, _rx2) = mpsc::channel(16);

        auth.register_node("node1".to_string(), tx1);
        auth.register_node("node2".to_string(), tx2);

        let expected = auth.expected_nodes();
        assert_eq!(expected.len(), 2);
        assert!(expected.contains(&"node1".to_string()));
        assert!(expected.contains(&"node2".to_string()));
    }

    #[tokio::test]
    async fn test_coordinator_auth_full_flow() {
        let config = create_train_config(true, 2);
        let mut auth = CoordinatorAuth::new(config);

        let (tx1, mut rx1) = mpsc::channel(16);
        let (tx2, mut rx2) = mpsc::channel(16);
        let (tx3, mut _rx3) = mpsc::channel(16);

        auth.register_node("n0".to_string(), tx1);
        auth.register_node("n1".to_string(), tx2);
        auth.register_node("n2".to_string(), tx3);

        assert!(auth.is_auth_enabled());
        assert_eq!(auth.get_threshold(), 2);
        assert_eq!(auth.registered_node_count(), 3);

        let status = auth.log_status_string();
        assert!(status.contains("3 nodes registered"));

        let mut rng = drift_auth::CryptoOsRng::new();
        let kp1 = SigningKey::generate(&mut rng);
        let kp2 = SigningKey::generate(&mut rng);

        let msg = AuthMessage::with_values("coordinator", "abc123", 1000u64, 42u64, 1u64);
        let signed1 = create_signed_message("n0", &msg, &kp1);
        let signed2 = create_signed_message("n1", &msg, &kp2);

        let agg = AggregateAuthMessage::create(vec![signed1, signed2], 2, 3).unwrap();
        assert_eq!(agg.threshold, 2);
        assert_eq!(agg.total_nodes, 3);
        assert_eq!(agg.node_ids.len(), 2);

        let broadcast_result = auth.broadcast_aggregate(&agg);
        assert!(broadcast_result.await.is_ok(), "broadcast should succeed");

        let msg1 = rx1.recv().await.unwrap();
        assert!(matches!(msg1, DriftMessage::AuthAggregate(_)));

        let msg2 = rx2.recv().await.unwrap();
        assert!(matches!(msg2, DriftMessage::AuthAggregate(_)));
    }
}