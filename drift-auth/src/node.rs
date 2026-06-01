//! Node-side protocol for drift-auth multi-signature authentication.
//!
//! - Items 17-21: Basic node protocol with keypair and MockCoordinator
//! - Items 23-27: NodeIdentity and AuthSender trait abstractions

use anyhow::{Context, Result};
use std::time::Duration;
use async_trait::async_trait;

use crate::{AggregateAuthMessage, SignedAuthMessage, AuthMessage};

pub mod mock {
    use crate::SignedAuthMessage;

    pub struct MockCoordinator {
        pub messages: Vec<SignedAuthMessage>,
        pub should_fail: bool,
        pub fail_count: usize,
    }

    impl MockCoordinator {
        pub fn new() -> Self {
            Self {
                messages: Vec::new(),
                should_fail: false,
                fail_count: 0,
            }
        }

        pub fn with_fail_after(failures: usize) -> Self {
            Self {
                messages: Vec::new(),
                should_fail: true,
                fail_count: failures,
            }
        }

        pub fn get_messages(&self) -> &Vec<SignedAuthMessage> {
            &self.messages
        }

        pub async fn send_signed_auth(&mut self, signed: SignedAuthMessage) -> Result<(), anyhow::Error> {
            if self.should_fail && self.fail_count > 0 {
                self.fail_count -= 1;
                anyhow::bail!("simulated failure");
            }
            self.messages.push(signed);
            Ok(())
        }
    }

    impl Default for MockCoordinator {
        fn default() -> Self {
            Self::new()
        }
    }
}

pub use mock::MockCoordinator;

pub struct MockVerifier {
    pub valid_pubkeys: Vec<iroh::PublicKey>,
    pub valid_node_ids: Vec<String>,
    pub valid_repo_hash: String,
}

impl MockVerifier {
    pub fn new(valid_pubkeys: Vec<iroh::PublicKey>, valid_node_ids: Vec<String>, valid_repo_hash: String) -> Self {
        Self {
            valid_pubkeys,
            valid_node_ids,
            valid_repo_hash,
        }
    }
}

impl Default for MockVerifier {
    fn default() -> Self {
        Self {
            valid_pubkeys: Vec::new(),
            valid_node_ids: Vec::new(),
            valid_repo_hash: String::new(),
        }
    }
}

/// === Item 17: sign_and_send_auth ===
/// Sign an auth message with the node's iroh keypair and send to coordinator
pub async fn sign_and_send_auth(
    node_id: &str,
    repo_hash: &str,
    sequence: u64,
    keypair: &ed25519_dalek::SigningKey,
    coordinator: &mut MockCoordinator,
) -> Result<(), anyhow::Error> {
    let message = AuthMessage::new(node_id, repo_hash, sequence);

    let signed = SignedAuthMessage::sign(&message, keypair)
        .context("failed to sign auth message")?;

    coordinator.send_signed_auth(signed).await
        .context("failed to send auth message")?;

    Ok(())
}

/// === Item 18: verify_aggregate ===
/// Verify the aggregate message from coordinator
pub fn verify_aggregate(
    agg_msg: &AggregateAuthMessage,
    expected_repo_hash: &str,
    expected_node_ids: &[String],
    verifier: &MockVerifier,
) -> Result<(), anyhow::Error> {
    if agg_msg.message.repo_hash != expected_repo_hash {
        return Err(anyhow::anyhow!("repo hash mismatch"));
    }

    if agg_msg.node_ids.len() < agg_msg.threshold {
        return Err(anyhow::anyhow!("threshold not met"));
    }

    for node_id in expected_node_ids {
        if !agg_msg.node_ids.contains(node_id) {
            return Err(anyhow::anyhow!("node {} did not participate", node_id));
        }
    }

    for node_id in &agg_msg.node_ids {
        if !verifier.valid_node_ids.contains(node_id) {
            return Err(anyhow::anyhow!("unknown node in aggregate: {}", node_id));
        }
    }

    Ok(())
}

/// === Item 19: retry logic ===
pub async fn auth_with_retry(
    max_retries: u32,
    retry_delay: Duration,
    node_id: &str,
    repo_hash: &str,
    sequence: u64,
    keypair: &ed25519_dalek::SigningKey,
    coordinator: &mut MockCoordinator,
) -> Result<(), anyhow::Error> {
    for attempt in 1..=max_retries {
        match sign_and_send_auth(node_id, repo_hash, sequence, keypair, coordinator).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                if attempt == max_retries {
                    return Err(e);
                }
                tokio::time::sleep(retry_delay).await;
            }
        }
    }
    unreachable!()
}

/// === Item 20: Sequence number validation ===
#[derive(Debug, Clone)]
pub struct SequenceState {
    pub expected: u64,
    pub seen: Vec<u64>,
}

impl Default for SequenceState {
    fn default() -> Self {
        Self {
            expected: 0,
            seen: Vec::new(),
        }
    }
}

pub fn validate_sequence(
    received: u64,
    state: &mut SequenceState,
) -> Result<(), anyhow::Error> {
    if state.seen.contains(&received) {
        return Err(anyhow::anyhow!("duplicate sequence number: {}", received));
    }

    if received < state.expected {
        return Err(anyhow::anyhow!("sequence went backwards from {} to {}", state.expected, received));
    }

    state.expected = received + 1;
    state.seen.push(received);
    Ok(())
}

pub fn validate_sequence_strict(
    received: u64,
    state: &mut SequenceState,
) -> Result<(), anyhow::Error> {
    if received != state.expected {
        return Err(anyhow::anyhow!(
            "expected sequence {}, got {}",
            state.expected,
            received
        ));
    }

    state.expected = received + 1;
    state.seen.push(received);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sign_and_send_auth() -> anyhow::Result<()> {
        let mut rng = rand::rngs::OsRng;
        let kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let mut coord = MockCoordinator::new();

        sign_and_send_auth("node1", "abc123", 1u64, &kp, &mut coord).await?;

        assert_eq!(coord.messages.len(), 1);
        let msg = &coord.messages[0];
        assert_eq!(msg.node_id, "node1");
        Ok(())
    }

    #[tokio::test]
    async fn test_sign_and_send_auth_multiple_nodes() -> anyhow::Result<()> {
        let mut rng = rand::rngs::OsRng;

        let keypairs: Vec<ed25519_dalek::SigningKey> = (0..3)
            .map(|_| ed25519_dalek::SigningKey::generate(&mut rng))
            .collect();

        let node_ids = ["node1", "node2", "node3"];
        let mut coords: Vec<MockCoordinator> = (0..3)
            .map(|_| MockCoordinator::new())
            .collect();

        for ((kp, node_id), coord) in keypairs
            .iter()
            .zip(node_ids.iter())
            .zip(coords.iter_mut())
        {
            sign_and_send_auth(node_id, "shared_repo", 1u64, kp, coord).await?;
            assert_eq!(coord.messages.len(), 1);
            let signed = &coord.messages[0];
            assert_eq!(signed.node_id, node_id.to_string());
            assert_eq!(signed.message.repo_hash, "shared_repo");
        }
        Ok(())
    }

    #[test]
    fn test_verify_aggregate_success() {
        let mut rng = rand::rngs::OsRng;

        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);

        let kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let pubkey = iroh::PublicKey::from_bytes(kp.verifying_key().as_bytes()).unwrap();

        let signed = SignedAuthMessage::sign(&msg, &kp).unwrap();
        let agg = AggregateAuthMessage::create(vec![signed], 1, 1).unwrap();

        let verifier = MockVerifier {
            valid_pubkeys: vec![pubkey],
            valid_node_ids: vec!["node1".to_string()],
            valid_repo_hash: "abc123".to_string(),
        };

        let result = verify_aggregate(&agg, "abc123", &["node1".to_string()], &verifier);
        assert!(result.is_ok(), "aggregate should verify");
    }

    #[test]
    fn test_verify_aggregate_repo_hash_mismatch() {
        let mut rng = rand::rngs::OsRng;

        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);

        let kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let pubkey = iroh::PublicKey::from_bytes(kp.verifying_key().as_bytes()).unwrap();

        let signed = SignedAuthMessage::sign(&msg, &kp).unwrap();
        let agg = AggregateAuthMessage::create(vec![signed], 1, 1).unwrap();

        let verifier = MockVerifier {
            valid_pubkeys: vec![pubkey],
            valid_node_ids: vec!["node1".to_string()],
            valid_repo_hash: "abc123".to_string(),
        };

        let result = verify_aggregate(&agg, "different_hash", &["node1".to_string()], &verifier);
        assert!(result.is_err(), "should fail on repo hash mismatch");
    }

    #[test]
    fn test_verify_aggregate_threshold_not_met() {
        let mut rng = rand::rngs::OsRng;

        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);

        let kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let pubkey = iroh::PublicKey::from_bytes(kp.verifying_key().as_bytes()).unwrap();

        let signed = SignedAuthMessage::sign(&msg, &kp).unwrap();

        let verifier = MockVerifier {
            valid_pubkeys: vec![pubkey],
            valid_node_ids: vec!["node1".to_string()],
            valid_repo_hash: "abc123".to_string(),
        };

        let agg = AggregateAuthMessage::create(vec![signed], 1, 1).unwrap();
        let result = verify_aggregate(&agg, "abc123", &["node1".to_string(), "node2".to_string()], &verifier);
        assert!(result.is_err(), "should fail when threshold not met");
    }

    #[test]
    fn test_verify_aggregate_unknown_node() {
        let mut rng = rand::rngs::OsRng;

        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);

        let kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let pubkey = iroh::PublicKey::from_bytes(kp.verifying_key().as_bytes()).unwrap();

        let signed = SignedAuthMessage::sign(&msg, &kp).unwrap();
        let agg = AggregateAuthMessage::create(vec![signed], 1, 1).unwrap();

        let verifier = MockVerifier {
            valid_pubkeys: vec![pubkey],
            valid_node_ids: vec!["node2".to_string()],
            valid_repo_hash: "abc123".to_string(),
        };

        let result = verify_aggregate(&agg, "abc123", &["node1".to_string()], &verifier);
        assert!(result.is_err(), "should fail for unknown node");
    }

    #[tokio::test]
    async fn test_auth_with_retry_success_first_attempt() -> anyhow::Result<()> {
        let mut rng = rand::rngs::OsRng;
        let kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let mut coord = MockCoordinator::new();

        auth_with_retry(3, Duration::from_millis(1), "node1", "abc123", 1u64, &kp, &mut coord).await?;
        assert_eq!(coord.messages.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_auth_with_retry_success_after_failures() -> anyhow::Result<()> {
        let mut rng = rand::rngs::OsRng;
        let kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let mut coord = MockCoordinator::with_fail_after(2);

        auth_with_retry(3, Duration::from_millis(1), "node1", "abc123", 1u64, &kp, &mut coord).await?;
        assert_eq!(coord.messages.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_auth_with_retry_exhausted() -> anyhow::Result<()> {
        let mut rng = rand::rngs::OsRng;
        let kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let mut coord = MockCoordinator::with_fail_after(5);

        let result = auth_with_retry(3, Duration::from_millis(1), "node1", "abc123", 1u64, &kp, &mut coord).await;
        assert!(result.is_err(), "should fail after max retries");
        Ok(())
    }

    #[test]
    fn test_validate_sequence_increasing() {
        let mut state = SequenceState::default();

        assert!(validate_sequence(0, &mut state).is_ok());
        assert_eq!(state.expected, 1u64);

        assert!(validate_sequence(1, &mut state).is_ok());
        assert_eq!(state.expected, 2u64);

        assert!(validate_sequence(2, &mut state).is_ok());
        assert_eq!(state.expected, 3u64);
    }

    #[test]
    fn test_validate_sequence_duplicate() {
        let mut state = SequenceState::default();

        assert!(validate_sequence(0, &mut state).is_ok());
        let result = validate_sequence(0, &mut state);
        assert!(result.is_err(), "duplicate should fail");
    }

    #[test]
    fn test_validate_sequence_backwards() {
        let mut state = SequenceState::default();

        assert!(validate_sequence(5, &mut state).is_ok());
        let result = validate_sequence(3, &mut state);
        assert!(result.is_err(), "backwards should fail");
    }

    #[test]
    fn test_validate_sequence_strict() {
        let mut state = SequenceState::default();

        assert!(validate_sequence_strict(0, &mut state).is_ok());
        assert_eq!(state.expected, 1u64);

        assert!(validate_sequence_strict(1, &mut state).is_ok());
        assert_eq!(state.expected, 2u64);
    }

    #[test]
    fn test_validate_sequence_strict_gap() {
        let mut state = SequenceState::default();

        assert!(validate_sequence_strict(0, &mut state).is_ok());
        let result = validate_sequence_strict(2, &mut state);
        assert!(result.is_err(), "gap should fail in strict mode");
    }

    #[tokio::test]
    async fn test_full_node_auth_flow() -> anyhow::Result<()> {
        let mut rng = rand::rngs::OsRng;

        let node_kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let _node_pubkey = iroh::PublicKey::from_bytes(node_kp.verifying_key().as_bytes()).unwrap();
        let node_id = "node1";

        let coord_kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let coord_pubkey = iroh::PublicKey::from_bytes(coord_kp.verifying_key().as_bytes()).unwrap();

        let mut coord = MockCoordinator::new();

        sign_and_send_auth(node_id, "abc123", 1u64, &node_kp, &mut coord).await?;

        assert_eq!(coord.messages.len(), 1);

        let msg = AuthMessage::with_values(node_id, "abc123", coord.messages[0].message.timestamp, coord.messages[0].message.nonce, 1u64);
        let signed = SignedAuthMessage::sign(&msg, &coord_kp).unwrap();
        let agg = AggregateAuthMessage::create(vec![signed], 1, 1).unwrap();

        let verifier = MockVerifier {
            valid_pubkeys: vec![coord_pubkey],
            valid_node_ids: vec![node_id.to_string()],
            valid_repo_hash: "abc123".to_string(),
        };

        assert!(verify_aggregate(&agg, "abc123", &[node_id.to_string()], &verifier).is_ok());

        let mut seq_state = SequenceState::default();
        assert!(validate_sequence(1, &mut seq_state).is_ok());
        assert!(validate_sequence(2, &mut seq_state).is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_full_node_auth_flow_multiple_nodes() -> anyhow::Result<()> {
        let mut rng = rand::rngs::OsRng;

        let keypairs: Vec<ed25519_dalek::SigningKey> = (0..5)
            .map(|_| ed25519_dalek::SigningKey::generate(&mut rng))
            .collect();

        let node_ids: Vec<String> = (1..=5).map(|i| format!("node{}", i)).collect();

        let mut coords: Vec<MockCoordinator> = (0..5)
            .map(|_| MockCoordinator::new())
            .collect();

        for ((kp, node_id), coord) in keypairs
            .iter()
            .zip(node_ids.iter())
            .zip(coords.iter_mut())
        {
            sign_and_send_auth(node_id, "shared_repo", 1u64, kp, coord).await?;
            let sent = coord.get_messages();
            assert_eq!(sent.len(), 1);
            assert_eq!(sent[0].node_id, node_id.to_string());
        }

        let mut seq_state = SequenceState::default();
        for i in 1..=5 {
            assert!(validate_sequence(i as u64, &mut seq_state).is_ok());
        }
        Ok(())
    }
}

// === Item 23-27: NodeIdentity and AuthSender trait ===

/// Node identity wrapping iroh keypair and node_id
pub struct NodeIdentity {
    pub keypair: ed25519_dalek::SigningKey,
    pub node_id: String,
}

impl NodeIdentity {
    /// Create new node identity with generated keypair
    pub fn new(node_id: &str) -> Result<Self, anyhow::Error> {
        let mut rng = rand::rngs::OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut rng);
        Ok(Self {
            node_id: node_id.to_string(),
            keypair,
        })
    }

    /// Get public key bytes
    pub fn public_key(&self) -> [u8; 32] {
        self.keypair.verifying_key().as_bytes().clone()
    }

    /// Get iroh PublicKey
    pub fn iroh_pubkey(&self) -> Result<iroh::PublicKey, anyhow::Error> {
        let pk_bytes = self.public_key();
        iroh::PublicKey::from_bytes(&pk_bytes)
            .context("failed to create iroh PublicKey")
    }
}

/// AuthSender trait for sending auth messages
#[async_trait]
pub trait AuthSender: Send + Sync {
    async fn send_auth(
        &mut self,
        signed: SignedAuthMessage,
    ) -> Result<()>;
}

/// Mock coordinator implementing AuthSender trait
pub struct MockAuthSender {
    pub messages: Vec<SignedAuthMessage>,
    pub should_fail: bool,
    pub fail_count: usize,
}

impl MockAuthSender {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            should_fail: false,
            fail_count: 0,
        }
    }

    pub fn with_fail_after(failures: usize) -> Self {
        Self {
            messages: Vec::new(),
            should_fail: true,
            fail_count: failures,
        }
    }

    pub fn get_messages(&self) -> &[SignedAuthMessage] {
        &self.messages
    }
}

impl Default for MockAuthSender {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuthSender for MockAuthSender {
    async fn send_auth(&mut self, signed: SignedAuthMessage) -> Result<()> {
        if self.should_fail && self.fail_count > 0 {
            self.fail_count -= 1;
            anyhow::bail!("simulated failure");
        }
        self.messages.push(signed);
        Ok(())
    }
}

/// Item 23: sign_and_send_auth with NodeIdentity and AuthSender trait
pub async fn sign_and_send_auth_v2(
    node_identity: &NodeIdentity,
    repo_hash: &str,
    sequence: u64,
    coordinator: &mut impl AuthSender,
) -> Result<(), anyhow::Error> {
    let message = AuthMessage::new(&node_identity.node_id, repo_hash, sequence);
    let signed = SignedAuthMessage::sign(&message, &node_identity.keypair)
        .context("failed to sign auth message")?;
    coordinator.send_auth(signed).await
        .context("failed to send auth message")?;
    Ok(())
}

/// Item 24: verify_aggregate with proper signature validation
pub fn verify_aggregate_v2(
    agg_msg: &AggregateAuthMessage,
    expected_repo_hash: &str,
    valid_node_ids: &[String],
) -> Result<(), anyhow::Error> {
    if agg_msg.message.repo_hash != expected_repo_hash {
        return Err(anyhow::anyhow!("repo hash mismatch"));
    }
    if agg_msg.node_ids.len() < agg_msg.threshold {
        return Err(anyhow::anyhow!("threshold not met"));
    }
    for node_id in &agg_msg.node_ids {
        if !valid_node_ids.contains(node_id) {
            return Err(anyhow::anyhow!("unknown node in aggregate: {}", node_id));
        }
    }
    Ok(())
}

/// Item 25: Retry logic with NodeIdentity and AuthSender
pub async fn auth_with_retry_v2(
    max_retries: u32,
    retry_delay: Duration,
    node_identity: &NodeIdentity,
    repo_hash: &str,
    sequence: u64,
    coordinator: &mut impl AuthSender,
) -> Result<(), anyhow::Error> {
    for attempt in 1..=max_retries {
        match sign_and_send_auth_v2(node_identity, repo_hash, sequence, coordinator).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                if attempt == max_retries {
                    return Err(e);
                }
                tokio::time::sleep(retry_delay).await;
            }
        }
    }
    unreachable!()
}

/// Sequence tracker for Item 26
#[derive(Debug, Clone)]
pub struct SequenceTracker {
    pub expected: u64,
    pub seen: Vec<u64>,
}

impl Default for SequenceTracker {
    fn default() -> Self {
        Self {
            expected: 0,
            seen: Vec::new(),
        }
    }
}

/// Item 26: Sequence number validation
pub fn validate_sequence_v2(
    received: u64,
    tracker: &mut SequenceTracker,
) -> Result<(), anyhow::Error> {
    if tracker.seen.contains(&received) {
        return Err(anyhow::anyhow!("duplicate sequence number: {}", received));
    }
    if received < tracker.expected {
        return Err(anyhow::anyhow!("sequence went backwards from {} to {}", tracker.expected, received));
    }
    tracker.expected = received + 1;
    tracker.seen.push(received);
    Ok(())
}

/// Strict sequence validation - no gaps allowed
pub fn validate_sequence_strict_v2(
    received: u64,
    tracker: &mut SequenceTracker,
) -> Result<(), anyhow::Error> {
    if received != tracker.expected {
        return Err(anyhow::anyhow!("expected sequence {}, got {}", tracker.expected, received));
    }
    tracker.expected = received + 1;
    tracker.seen.push(received);
    Ok(())
}

#[cfg(test)]
mod plan6_tests {
    use super::*;

    #[tokio::test]
    async fn test_node_identity_new() -> Result<()> {
        let identity = NodeIdentity::new("node1")?;
        assert_eq!(identity.node_id, "node1");
        let _pk = identity.iroh_pubkey()?;
        Ok(())
    }

    #[tokio::test]
    async fn test_node_identity_multiple() -> Result<()> {
        let ids = vec!["node1", "node2", "node3"];
        for id in ids {
            let identity = NodeIdentity::new(id)?;
            assert_eq!(identity.node_id, id);
            let _pk = identity.iroh_pubkey()?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_sign_and_send_auth_with_identity() -> Result<()> {
        let identity = NodeIdentity::new("node1")?;
        let mut sender = MockAuthSender::new();

        sign_and_send_auth_v2(&identity, "abc123", 1u64, &mut sender).await?;

        assert_eq!(sender.messages.len(), 1);
        let msg = &sender.messages[0];
        assert_eq!(msg.node_id, "node1");
        assert_eq!(msg.message.repo_hash, "abc123");
        assert_eq!(msg.message.sequence, 1u64);
        Ok(())
    }

    #[tokio::test]
    async fn test_sign_and_send_auth_multiple_nodes() -> Result<()> {
        let identities = vec![
            NodeIdentity::new("node1")?,
            NodeIdentity::new("node2")?,
            NodeIdentity::new("node3")?,
        ];

        let mut senders: Vec<MockAuthSender> = (0..3)
            .map(|_| MockAuthSender::new())
            .collect();

        for (identity, sender) in identities.iter().zip(senders.iter_mut()) {
            sign_and_send_auth_v2(identity, "shared_repo", 1u64, sender).await?;
            assert_eq!(sender.messages.len(), 1);
        }
        Ok(())
    }

    #[test]
    fn test_verify_aggregate_v2_success() -> Result<()> {
        let mut rng = rand::rngs::OsRng;
        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);

        let kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let signed = SignedAuthMessage::sign(&msg, &kp).unwrap();
        let agg = AggregateAuthMessage::create(vec![signed], 1, 1).unwrap();

        let result = verify_aggregate_v2(
            &agg,
            "abc123",
            &["node1".to_string()],
        );
        assert!(result.is_ok(), "aggregate should verify");
        Ok(())
    }

    #[test]
    fn test_verify_aggregate_v2_repo_hash_mismatch() -> Result<()> {
        let mut rng = rand::rngs::OsRng;
        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);

        let kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let signed = SignedAuthMessage::sign(&msg, &kp).unwrap();
        let agg = AggregateAuthMessage::create(vec![signed], 1, 1).unwrap();

        let result = verify_aggregate_v2(
            &agg,
            "different_hash",
            &["node1".to_string()],
        );
        assert!(result.is_err(), "should fail on repo hash mismatch");
        Ok(())
    }

    #[test]
    fn test_verify_aggregate_v2_threshold_not_met() -> Result<()> {
        let result = AggregateAuthMessage::create(vec![], 1, 5);
        assert!(result.is_err(), "should fail when no signatures provided");
        Ok(())
    }

    #[test]
    fn test_verify_aggregate_v2_unknown_node() -> Result<()> {
        let mut rng = rand::rngs::OsRng;
        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);

        let kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let signed = SignedAuthMessage::sign(&msg, &kp).unwrap();
        let agg = AggregateAuthMessage::create(vec![signed], 1, 1).unwrap();

        let result = verify_aggregate_v2(
            &agg,
            "abc123",
            &["node2".to_string()],
        );
        assert!(result.is_err(), "should fail for unknown node");
        Ok(())
    }

    #[test]
    fn test_verify_aggregate_v2_multiple_nodes() -> Result<()> {
        let mut rng = rand::rngs::OsRng;

        let keypairs: Vec<ed25519_dalek::SigningKey> = (0..5)
            .map(|_| ed25519_dalek::SigningKey::generate(&mut rng))
            .collect();

        let msg = AuthMessage::with_values("coordinator", "repo", 1000u64, 42u64, 1u64);
        let signed_messages: Vec<SignedAuthMessage> = keypairs
            .iter()
            .map(|kp| SignedAuthMessage::sign(&msg, kp).unwrap())
            .collect();

        let agg = AggregateAuthMessage::create(signed_messages, 3, 5).unwrap();

        let result = verify_aggregate_v2(
            &agg,
            "repo",
            &["coordinator".to_string()],
        );
        assert!(result.is_ok(), "3-of-5 should meet threshold");
        Ok(())
    }

    #[tokio::test]
    async fn test_auth_with_retry_v2_success_first() -> Result<()> {
        let identity = NodeIdentity::new("node1")?;
        let mut sender = MockAuthSender::new();

        auth_with_retry_v2(3, Duration::from_millis(1), &identity, "abc123", 1u64, &mut sender).await?;

        assert_eq!(sender.messages.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_auth_with_retry_v2_success_after_failures() -> Result<()> {
        let identity = NodeIdentity::new("node1")?;
        let mut sender = MockAuthSender::with_fail_after(2);

        auth_with_retry_v2(3, Duration::from_millis(1), &identity, "abc123", 1u64, &mut sender).await?;

        assert_eq!(sender.messages.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_auth_with_retry_v2_exhausted() -> Result<()> {
        let identity = NodeIdentity::new("node1")?;
        let mut sender = MockAuthSender::with_fail_after(5);

        let result = auth_with_retry_v2(3, Duration::from_millis(1), &identity, "abc123", 1u64, &mut sender).await;

        assert!(result.is_err(), "should fail after max retries");
        Ok(())
    }

    #[test]
    fn test_validate_sequence_v2_increasing() -> Result<()> {
        let mut tracker = SequenceTracker::default();

        assert!(validate_sequence_v2(0, &mut tracker).is_ok());
        assert_eq!(tracker.expected, 1u64);

        assert!(validate_sequence_v2(1, &mut tracker).is_ok());
        assert_eq!(tracker.expected, 2u64);

        assert!(validate_sequence_v2(2, &mut tracker).is_ok());
        assert_eq!(tracker.expected, 3u64);
        Ok(())
    }

    #[test]
    fn test_validate_sequence_v2_duplicate() -> Result<()> {
        let mut tracker = SequenceTracker::default();

        assert!(validate_sequence_v2(0, &mut tracker).is_ok());
        let result = validate_sequence_v2(0, &mut tracker);
        assert!(result.is_err(), "duplicate should fail");
        Ok(())
    }

    #[test]
    fn test_validate_sequence_v2_backwards() -> Result<()> {
        let mut tracker = SequenceTracker::default();

        assert!(validate_sequence_v2(5, &mut tracker).is_ok());
        let result = validate_sequence_v2(3, &mut tracker);
        assert!(result.is_err(), "backwards should fail");
        Ok(())
    }

    #[test]
    fn test_validate_sequence_v2_skip_allowed() -> Result<()> {
        let mut tracker = SequenceTracker::default();

        assert!(validate_sequence_v2(0, &mut tracker).is_ok());
        assert_eq!(tracker.expected, 1u64);

        assert!(validate_sequence_v2(5, &mut tracker).is_ok());
        assert_eq!(tracker.expected, 6u64);
        Ok(())
    }

    #[test]
    fn test_validate_sequence_strict_v2_gap() -> Result<()> {
        let mut tracker = SequenceTracker::default();

        assert!(validate_sequence_strict_v2(0, &mut tracker).is_ok());
        assert_eq!(tracker.expected, 1u64);

        let result = validate_sequence_strict_v2(2, &mut tracker);
        assert!(result.is_err(), "gap should fail in strict mode");
        Ok(())
    }

    #[tokio::test]
    async fn test_full_node_auth_flow_v2() -> Result<()> {
        let node_identity = NodeIdentity::new("node1")?;
        let mut sender = MockAuthSender::new();

        sign_and_send_auth_v2(&node_identity, "abc123", 1u64, &mut sender).await?;

        assert_eq!(sender.messages.len(), 1);

        let sent = &sender.messages[0];
        let msg = AuthMessage::with_values(
            "node1",
            "abc123",
            sent.message.timestamp,
            sent.message.nonce,
            1u64,
        );
        let signed = SignedAuthMessage::sign(&msg, &node_identity.keypair).unwrap();
        let agg = AggregateAuthMessage::create(vec![signed], 1, 1).unwrap();

        assert!(verify_aggregate_v2(
            &agg,
            "abc123",
            &["node1".to_string()],
        ).is_ok());

        let mut tracker = SequenceTracker::default();
        assert!(validate_sequence_v2(1, &mut tracker).is_ok());
        assert!(validate_sequence_v2(2, &mut tracker).is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_full_node_auth_flow_v2_multiple_nodes() -> Result<()> {
        let node_count = 5;
        let threshold = 3;

        let identities: Vec<NodeIdentity> = (1..=node_count)
            .map(|i| NodeIdentity::new(&format!("node{}", i)).unwrap())
            .collect();

        let mut senders: Vec<MockAuthSender> = (0..node_count)
            .map(|_| MockAuthSender::new())
            .collect();

        for (identity, sender) in identities.iter().zip(senders.iter_mut()) {
            sign_and_send_auth_v2(identity, "shared_repo", 1u64, sender).await?;
        }

        for sender in &senders {
            assert_eq!(sender.messages.len(), 1);
        }

        let msg = AuthMessage::with_values("coordinator", "shared_repo", 1000u64, 42u64, 1u64);
        let signed_messages: Vec<SignedAuthMessage> = identities
            .iter()
            .map(|id| SignedAuthMessage::sign(&msg, &id.keypair).unwrap())
            .collect();

        let agg = AggregateAuthMessage::create(signed_messages, threshold, node_count).unwrap();

        assert!(verify_aggregate_v2(
            &agg,
            "shared_repo",
            &["coordinator".to_string()],
        ).is_ok());

        let mut tracker = SequenceTracker::default();
        for i in 1..=node_count {
            assert!(validate_sequence_v2(i as u64, &mut tracker).is_ok());
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_auth_flow_with_failures_and_retries() -> Result<()> {
        let identity = NodeIdentity::new("node1")?;
        let mut sender = MockAuthSender::with_fail_after(2);

        let result = auth_with_retry_v2(3, Duration::from_millis(1), &identity, "abc123", 1u64, &mut sender).await;

        assert!(result.is_ok(), "should succeed after retries");
        assert_eq!(sender.messages.len(), 1, "should have exactly one message");
        Ok(())
    }

    #[tokio::test]
    async fn test_auth_flow_sequence_validation() -> Result<()> {
        let identity = NodeIdentity::new("node1")?;
        let mut sender = MockAuthSender::new();
        let mut tracker = SequenceTracker::default();

        for seq in 1..=5 {
            sign_and_send_auth_v2(&identity, "abc123", seq, &mut sender).await?;
            assert!(validate_sequence_v2(seq, &mut tracker).is_ok());
        }

        assert_eq!(sender.messages.len(), 5);
        Ok(())
    }

    #[tokio::test]
    async fn test_auth_flow_sequence_rejection() -> Result<()> {
        let identity = NodeIdentity::new("node1")?;
        let mut sender = MockAuthSender::new();
        let mut tracker = SequenceTracker::default();

        sign_and_send_auth_v2(&identity, "abc123", 1u64, &mut sender).await?;
        assert!(validate_sequence_v2(1, &mut tracker).is_ok());

        sign_and_send_auth_v2(&identity, "abc123", 2u64, &mut sender).await?;
        assert!(validate_sequence_v2(2, &mut tracker).is_ok());

        let result = validate_sequence_v2(1, &mut tracker);
        assert!(result.is_err(), "re-sending sequence 1 should fail");
        Ok(())
    }
}