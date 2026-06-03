## 3. Message Structures (Items 6-10)

**Continue in drift-auth/src/lib.rs:**

```rust
// drift-auth/src/lib.rs (continued)
// Checklist items: 6, 7, 8, 9, 10

use std::time::{SystemTime, UNIX_EPOCH};

/// === Item 6: AuthMessage ===
/// Message that a node signs and sends to coordinator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthMessage {
    pub node_id: String,
    pub repo_hash: String,  // commit SHA
    pub timestamp: u64,    // UNIX seconds
    pub nonce: u64,       // random nonce
    pub sequence: u64,    // sequence number
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

    /// Check if timestamp is within acceptable window (e.g., 5 minutes)
    pub fn is_timestamp_valid(&self, max_age_secs: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now.saturating_sub(self.timestamp) <= max_age_secs
    }

    /// Serialize message for signing
    pub fn as_bytes(&self) -> Vec<u8> {
        format!("{}|{}|{}|{}|{}",
            self.node_id,
            self.repo_hash,
            self.timestamp,
            self.nonce,
            self.sequence
        ).into_bytes()
    }
}

/// === Item 7: SignedAuthMessage ===
/// An auth message signed by a node (includes the signature)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedAuthMessage {
    pub node_id: String,
    pub message: AuthMessage,
    pub signature: Vec<u8>,  // Ed25519 signature
}

impl SignedAuthMessage {
    /// Sign an auth message with a node's iroh keypair
    pub fn sign(
        message: &AuthMessage,
        keypair: &iroh::Keypair,
    ) -> Result<Self, CryptoError> {
        let msg_bytes = message.as_bytes();
        let signature = sign_message_with_iroh_keypair(
            &message.node_id,
            &message.repo_hash,
            message.timestamp,
            message.nonce,
            message.sequence,
            keypair,
        )?;

        Ok(Self {
            node_id: message.node_id.clone(),
            message: message.clone(),
            signature: signature.as_bytes().to_vec(),
        })
    }

    /// Verify the signature using the node's iroh public key
    pub fn verify(&self, pubkey: &PublicKey) -> Result<(), CryptoError> {
        verify_signature_with_iroh_pubkey(
            pubkey,
            &self.message.node_id,
            &self.message.repo_hash,
            self.message.timestamp,
            self.message.nonce,
            self.message.sequence,
            &Signature::from_bytes(&self.signature)
                .map_err(|_| CryptoError::InvalidKey)?,
        )
    }
}

/// === Item 8: AggregateAuthMessage ===
/// Collection of signatures with aggregated threshold signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateAuthMessage {
    pub node_ids: Vec<String>,  // nodes that participated
    pub message: AuthMessage,  // the message they all signed
    pub aggregated_signature: Vec<u8>,  // Combined threshold signature
    pub threshold: usize,
    pub total_nodes: usize,
}

impl AggregateAuthMessage {
    /// Create aggregate from collected signatures
    pub fn create(
        signed_messages: Vec<SignedAuthMessage],
        threshold: usize,
        total_nodes: usize,
    ) -> Result<Self, CryptoError> {
        if signed_messages.is_empty() {
            return Err(CryptoError::VerificationFailed);
        }
        if signed_messages.len() < threshold {
            return Err(CryptoError::AggregationFailed(
                format!("only {} signatures, need {}", signed_messages.len(), threshold)
            ));
        }

        // All messages must be identical (except signature)
        let first_msg = &signed_messages[0].message;
        for sm in signed_messages.iter().skip(1) {
            if sm.message.node_id != first_msg.node_id ||
               sm.message.repo_hash != first_msg.repo_hash ||
               sm.message.timestamp != first_msg.timestamp ||
               sm.message.nonce != first_msg.nonce ||
               sm.message.sequence != first_msg.sequence {
                return Err(CryptoError::VerificationFailed);
            }
        }

        // Collect signatures
        let ed25519_sigs: Result<Vec<Signature>, _> = signed_messages.iter()
            .map(|s| Signature::from_bytes(&s.signature)
                .map_err(|_| CryptoError::InvalidKey))
            .collect();
        let ed25519_sigs = ed25519_sigs?;

        // Aggregate threshold number of signatures
        let aggregated = aggregate_signatures(&ed25519_sigs, threshold)?;

        Ok(Self {
            node_ids: signed_messages.iter().map(|s| s.node_id.clone()).collect(),
            message: first_msg.clone(),
            aggregated_signature: aggregated.as_bytes().to_vec(),
            threshold,
            total_nodes,
        })
    }

    /// Verify aggregate signature using any of the node's public keys
    pub fn verify(&self, pubkeys: &[PublicKey]) -> Result<(), CryptoError> {
        if self.node_ids.is_empty() {
            return Err(CryptoError::VerificationFailed);
        }

        // Verify aggregated signature with any of the public keys
        // (since they all signed the same message)
        let msg_bytes = self.message.as_bytes();
        let agg_sig = Signature::from_bytes(&self.aggregated_signature)
            .map_err(|_| CryptoError::InvalidKey)?;

        // Try each public key until one verifies
        let mut verified = false;
        for pk in pubkeys {
            pk.verify(msg_bytes, &agg_sig)
                .map_err(|_| CryptoError::VerificationFailed)?;
            verified = true;
            break;
        }

        if !verified {
            return Err(CryptoError::VerificationFailed);
        }

        Ok(())
    }
}

/// === Items 9-10: Serialization tests ===
#[cfg(test)]
mod message_tests {
    use super::*;
    use ed25519_dalek::Keypair;
    use iroh::PublicKey;

    #[test]
    fn test_auth_message_serialization() {
        let msg = AuthMessage::new("node1", "abc123", 1);
        let bytes = msg.as_bytes();
        // Verify deterministic serialization
        let msg2 = AuthMessage::new("node1", "abc123", 1);
        assert_eq!(bytes, msg2.as_bytes());
    }

    #[test]
    fn test_signed_message_roundtrip() {
        let kp = Keypair::generate(&mut rand::rngs::OsRng);
        let msg = AuthMessage::new("node1", "abc123", 1);
        let signed = SignedAuthMessage::sign(&msg, &kp).unwrap();

        // Verify signature
        let pk = PublicKey::from_bytes(kp.public.as_bytes()).unwrap();
        assert!(signed.verify(&pk).is_ok());
    }

    #[test]
    fn test_aggregate_message_verify() {
        use rand::rngs::OsRng;
        let mut rng = OsRng;
        let mut keypairs: Vec<Keypair> = Vec::new();
        let mut pubkeys: Vec<PublicKey> = Vec::new();
        for _ in 0..5 {
            let kp = Keypair::generate(&mut rng);
            keypairs.push(kp);
            pubkeys.push(PublicKey::from_bytes(keypairs.last().unwrap().public.as_bytes()).unwrap());
        }
        let msg = AuthMessage::new("node1", "abc123", 1);

        let signed_messages: Vec<SignedAuthMessage] = keypairs.iter()
            .map(|kp| SignedAuthMessage::sign(&msg, kp).unwrap())
            .collect();

        let agg = AggregateAuthMessage::create(signed_messages, 3, 5).unwrap();

        assert!(agg.verify(&pubkeys).is_ok());
    }
}
```
