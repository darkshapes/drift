//! Message structures for drift-auth multi-signature authentication.
//!
//! - AuthMessage: Core message format for auth requests
//! - SignedAuthMessage: AuthMessage with node signature
//! - AggregateAuthMessage: Collection of signatures meeting threshold

use ed25519_dalek::{Signer, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub type RoundId = Uuid;

use crate::crypto::CryptoError;

/// === Item 6: AuthMessage ===
/// Message that a node signs and sends to coordinator
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthMessage {
    pub node_id: String,
    pub repo_hash: String,
    pub timestamp: u64,
    pub nonce: u64,
    pub sequence: u64,
}

impl AuthMessage {
    /// Create new auth message with current timestamp
    pub fn new(node_id: &str, repo_hash: &str, sequence: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let nonce = rand::random();

        Self {
            node_id: node_id.to_string(),
            repo_hash: repo_hash.to_string(),
            timestamp,
            nonce,
            sequence,
        }
    }

    /// Create with explicit values (for testing)
    pub fn with_values(node_id: &str, repo_hash: &str, timestamp: u64, nonce: u64, sequence: u64) -> Self {
        Self {
            node_id: node_id.to_string(),
            repo_hash: repo_hash.to_string(),
            timestamp,
            nonce,
            sequence,
        }
    }

    /// Check if timestamp is within acceptable window
    pub fn is_timestamp_valid(&self, max_age_secs: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now.saturating_sub(self.timestamp) <= max_age_secs
    }

    /// Serialize message for signing
    pub fn as_bytes(&self) -> Vec<u8> {
        format!(
            "{}|{}|{}|{}|{}",
            self.node_id,
            self.repo_hash,
            self.timestamp,
            self.nonce,
            self.sequence
        )
        .into_bytes()
    }
}

/// === Item 7: SignedAuthMessage ===
/// An auth message signed by a node (includes the signature)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedAuthMessage {
    pub node_id: String,
    pub message: AuthMessage,
    pub signature: Vec<u8>,
}

impl SignedAuthMessage {
    /// Sign an auth message with a node's ed25519_dalek keypair
    pub fn sign(
        message: &AuthMessage,
        keypair: &ed25519_dalek::SigningKey,
    ) -> Result<Self, CryptoError> {
        let msg_bytes = message.as_bytes();
        let sig = keypair.sign(&msg_bytes);

        Ok(Self {
            node_id: message.node_id.clone(),
            message: message.clone(),
            signature: sig.to_bytes().to_vec(),
        })
    }

    /// Verify the signature using the node's iroh public key
    pub fn verify(&self, pubkey: &iroh::PublicKey) -> Result<(), CryptoError> {
        let msg_bytes = self.message.as_bytes();
        let pk_bytes = pubkey.as_bytes();
        let pk = VerifyingKey::from_bytes(pk_bytes)
            .map_err(|_| CryptoError::InvalidKey)?;
        let sig_bytes: [u8; 64] = self.signature.clone().try_into().map_err(|_| CryptoError::InvalidKey)?;
        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);

        pk.verify(&msg_bytes, &sig)
            .map_err(|_| CryptoError::VerificationFailed)
    }
}

/// === Item 8: AggregateAuthMessage ===
/// Collection of signatures with aggregated threshold signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateAuthMessage {
    pub node_ids: Vec<String>,
    pub message: AuthMessage,
    pub aggregated_signature: Vec<u8>,
    pub threshold: usize,
    pub total_nodes: usize,
}

impl AggregateAuthMessage {
    /// Create aggregate from collected signatures
    pub fn create(
        signed_messages: Vec<SignedAuthMessage>,
        threshold: usize,
        total_nodes: usize,
    ) -> Result<Self, CryptoError> {
        if signed_messages.is_empty() {
            return Err(CryptoError::AggregationFailed("no signatures provided".to_string()));
        }
        if signed_messages.len() < threshold {
            return Err(CryptoError::AggregationFailed(
                format!("only {} signatures, need {}", signed_messages.len(), threshold)
            ));
        }

        let first_msg = &signed_messages[0].message;
        for sm in signed_messages.iter().skip(1) {
            if sm.message.node_id != first_msg.node_id
                || sm.message.repo_hash != first_msg.repo_hash
                || sm.message.timestamp != first_msg.timestamp
                || sm.message.nonce != first_msg.nonce
                || sm.message.sequence != first_msg.sequence
            {
                return Err(CryptoError::VerificationFailed);
            }
        }

        let ed25519_sigs: Result<Vec<_>, _> = signed_messages
            .iter()
            .map(|s| {
                let sig_bytes: [u8; 64] = s.signature.clone().try_into().map_err(|_| CryptoError::InvalidKey)?;
                Ok(ed25519_dalek::Signature::from_bytes(&sig_bytes))
            })
            .collect();
        let ed25519_sigs = ed25519_sigs?;

        let aggregated = crate::crypto::aggregate_signatures(&ed25519_sigs, threshold)?;

        Ok(Self {
            node_ids: signed_messages.iter().map(|s| s.node_id.clone()).collect(),
            message: first_msg.clone(),
            aggregated_signature: aggregated.to_bytes().to_vec(),
            threshold,
            total_nodes,
        })
    }

    /// Verify aggregate signature using any of the node's public keys
    pub fn verify(&self, pubkeys: &[iroh::PublicKey]) -> Result<(), CryptoError> {
        if self.node_ids.is_empty() {
            return Err(CryptoError::VerificationFailed);
        }

        let msg_bytes = self.message.as_bytes();
        let sig_bytes: [u8; 64] = self.aggregated_signature.clone().try_into().map_err(|_| CryptoError::InvalidKey)?;
        let agg_sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);

        for pk in pubkeys {
            let pk_bytes = pk.as_bytes();
            let vk = VerifyingKey::from_bytes(pk_bytes)
                .map_err(|_| CryptoError::InvalidKey)?;
            if vk.verify(&msg_bytes, &agg_sig).is_ok() {
                return Ok(());
            }
        }

        Err(CryptoError::VerificationFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_message_new() {
        let msg = AuthMessage::new("node1", "abc123", 1);
        assert_eq!(msg.node_id, "node1");
        assert_eq!(msg.repo_hash, "abc123");
        assert_eq!(msg.sequence, 1u64);
    }

    #[test]
    fn test_auth_message_with_values() {
        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);
        assert_eq!(msg.node_id, "node1");
        assert_eq!(msg.repo_hash, "abc123");
        assert_eq!(msg.timestamp, 1000u64);
        assert_eq!(msg.nonce, 42u64);
        assert_eq!(msg.sequence, 1u64);
    }

    #[test]
    fn test_auth_message_as_bytes() {
        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);
        let bytes = msg.as_bytes();
        let expected = b"node1|abc123|1000|42|1";
        assert_eq!(bytes, expected);
    }

    #[test]
    fn test_auth_message_serialization() {
        let msg1 = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);
        let bytes = serde_json::to_vec(&msg1).unwrap();
        let msg2: AuthMessage = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(msg1, msg2);
    }

    #[test]
    fn test_signed_auth_message_sign_verify() {
        let mut rng = rand::rngs::OsRng;
        let kp = ed25519_dalek::SigningKey::generate(&mut rng);

        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);
        let signed = SignedAuthMessage::sign(&msg, &kp).unwrap();

        let pk = iroh::PublicKey::from_bytes(kp.verifying_key().as_bytes()).unwrap();
        assert!(signed.verify(&pk).is_ok());
    }

    #[test]
    fn test_signed_auth_message_verify_wrong_key() {
        let mut rng = rand::rngs::OsRng;
        let kp1 = ed25519_dalek::SigningKey::generate(&mut rng);
        let kp2 = ed25519_dalek::SigningKey::generate(&mut rng);

        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);
        let signed = SignedAuthMessage::sign(&msg, &kp1).unwrap();

        let pk2 = iroh::PublicKey::from_bytes(kp2.verifying_key().as_bytes()).unwrap();
        assert!(signed.verify(&pk2).is_err());
    }

    #[test]
    fn test_aggregate_auth_message_create_verify() {
        let mut rng = rand::rngs::OsRng;

        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);

        let keypairs: Vec<ed25519_dalek::SigningKey> = (0..5).map(|_| ed25519_dalek::SigningKey::generate(&mut rng)).collect();
        let pubkeys: Vec<iroh::PublicKey> = keypairs
            .iter()
            .map(|kp| iroh::PublicKey::from_bytes(kp.verifying_key().as_bytes()).unwrap())
            .collect();

        let signed_messages: Vec<SignedAuthMessage> = keypairs
            .iter()
            .map(|kp| SignedAuthMessage::sign(&msg, kp).unwrap())
            .collect();

        let agg = AggregateAuthMessage::create(signed_messages, 3, 5).unwrap();
        assert_eq!(agg.threshold, 3);
        assert_eq!(agg.total_nodes, 5);
        assert_eq!(agg.node_ids.len(), 5);

        assert!(agg.verify(&pubkeys).is_ok());
    }

    #[test]
    fn test_aggregate_auth_message_insufficient_signatures() {
        let mut rng = rand::rngs::OsRng;
        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);

        let keypairs: Vec<ed25519_dalek::SigningKey> = (0..2).map(|_| ed25519_dalek::SigningKey::generate(&mut rng)).collect();

        let signed_messages: Vec<SignedAuthMessage> = keypairs
            .iter()
            .map(|kp| SignedAuthMessage::sign(&msg, kp).unwrap())
            .collect();

        let result = AggregateAuthMessage::create(signed_messages, 3, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_aggregate_auth_message_mismatched_messages() {
        let mut rng = rand::rngs::OsRng;

        let msg1 = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);
        let msg2 = AuthMessage::with_values("node2", "def456", 1000u64, 42u64, 1u64);

        let kp1 = ed25519_dalek::SigningKey::generate(&mut rng);
        let kp2 = ed25519_dalek::SigningKey::generate(&mut rng);

        let signed1 = SignedAuthMessage::sign(&msg1, &kp1).unwrap();
        let signed2 = SignedAuthMessage::sign(&msg2, &kp2).unwrap();

        let result = AggregateAuthMessage::create(vec![signed1, signed2], 2, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_aggregate_auth_message_empty() {
        let result = AggregateAuthMessage::create(vec![], 1, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_signed_auth_message_roundtrip() {
        let mut rng = rand::rngs::OsRng;
        let kp = ed25519_dalek::SigningKey::generate(&mut rng);

        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);
        let signed = SignedAuthMessage::sign(&msg, &kp).unwrap();

        let bytes = serde_json::to_vec(&signed).unwrap();
        let parsed: SignedAuthMessage = serde_json::from_slice(&bytes).unwrap();

        let pk = iroh::PublicKey::from_bytes(kp.verifying_key().as_bytes()).unwrap();
        assert!(parsed.verify(&pk).is_ok());
    }

    #[test]
    fn test_aggregate_auth_message_roundtrip() {
        let mut rng = rand::rngs::OsRng;

        let msg = AuthMessage::with_values("node1", "abc123", 1000u64, 42u64, 1u64);

        let keypairs: Vec<ed25519_dalek::SigningKey> = (0..5).map(|_| ed25519_dalek::SigningKey::generate(&mut rng)).collect();
        let pubkeys: Vec<iroh::PublicKey> = keypairs
            .iter()
            .map(|kp| iroh::PublicKey::from_bytes(kp.verifying_key().as_bytes()).unwrap())
            .collect();

        let signed_messages: Vec<SignedAuthMessage> = keypairs
            .iter()
            .map(|kp| SignedAuthMessage::sign(&msg, kp).unwrap())
            .collect();

        let agg = AggregateAuthMessage::create(signed_messages, 3, 5).unwrap();

        let bytes = serde_json::to_vec(&agg).unwrap();
        let parsed: AggregateAuthMessage = serde_json::from_slice(&bytes).unwrap();

        assert!(parsed.verify(&pubkeys).is_ok());
    }

    #[test]
    fn test_auth_message_timestamp_valid_fresh() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let msg = AuthMessage::with_values("node1", "abc123", now, 42u64, 1u64);
        assert!(msg.is_timestamp_valid(300u64));
    }

    #[test]
    fn test_auth_message_timestamp_valid_expired() {
        let msg = AuthMessage::with_values("node1", "abc123", 1u64, 42u64, 1u64);
        assert!(!msg.is_timestamp_valid(300u64));
    }
}