## 2. Crypto Primitives (Items 1-5)

**Note:** We use iroh's existing Ed25519 keys. No separate key generation needed.

---

### Create drift-auth/src/lib.rs

```rust
// drift-auth/src/lib.rs
// Checklist items: 1, 2, 3, 4, 5

use serde::{Serialize, Deserialize};
use thiserror::Error;

/// Errors related to cryptographic operations
#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("signature verification failed")]
    VerificationFailed,
    #[error("invalid key format")]
    InvalidKey,
    #[error("aggregation failed: {0}")]
    AggregationFailed(String),
}

// We use ed25519-dalek for signature operations, but keys come from iroh
use ed25519_dalek::{Signature, signer::Signer, Verifier};

// Import iroh types for public keys
use iroh::PublicKey;

/// === Item 1: Use existing iroh Ed25519 keys ===
/// Each node uses its iroh keypair directly. No separate identity wrapper needed.

/// === Item 2: Sign message with iroh keypair ===
/// Sign an auth message using an iroh keypair
pub fn sign_message_with_iroh_keypair(
    node_id: &str,
    repo_hash: &str,
    timestamp: u64,
    nonce: u64,
    sequence: u64,
    keypair: &iroh::Keypair,  // iroh's Ed25519 keypair
) -> Result<Signature, CryptoError> {
    // Serialize message components
    // Format: "node_id|repo_hash|timestamp|nonce|sequence"
    let message = format!("{}|{}|{}|{}|{}", node_id, repo_hash, timestamp, nonce, sequence);
    Ok(keypair.sign(message.as_bytes()))
}

/// === Item 3: Verify signature with iroh public key ===
pub fn verify_signature_with_iroh_pubkey(
    pubkey: &PublicKey,  // iroh PublicKey
    node_id: &str,
    repo_hash: &str,
    timestamp: u64,
    nonce: u64,
    sequence: u64,
    signature: &Signature,
) -> Result<(), CryptoError> {
    // Convert iroh PublicKey to ed25519_dalek::PublicKey
    let pk_bytes = pubkey.as_bytes();
    let pk = ed25519_dalek::PublicKey::from_bytes(pk_bytes)
        .map_err(|_| CryptoError::InvalidKey)?;

    let message = format!("{}|{}|{}|{}|{}", node_id, repo_hash, timestamp, nonce, sequence);
    pk.verify(message.as_bytes(), signature)
        .map_err(|_| CryptoError::VerificationFailed)
}

/// === Item 4: Aggregate Signatures (threshold m-of-n) ===
/// Ed25519 supports signature addition for multisig
pub fn aggregate_signatures(
    signatures: &[Signature],
    threshold: usize,
) -> Result<Signature, CryptoError> {
    if signatures.len() < threshold {
        return Err(CryptoError::AggregationFailed(
            format!("only {} signatures, need {}", signatures.len(), threshold)
        ));
    }

    // Use first signature as accumulator
    let mut agg = signatures[0].clone();
    for sig in signatures.iter().take(threshold).skip(1) {
        agg = agg.add(sig)
            .ok_or_else(|| CryptoError::AggregationFailed("failed to add signatures".to_string()))?;
    }
    Ok(agg)
}

/// === Item 5: Unit tests ===
#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Keypair;

    #[test]
    fn test_sign_verify() {
        let kp1 = Keypair::generate(&mut rand::rngs::OsRng);
        let kp2 = Keypair::generate(&mut rand::rngs::OsRng);

        let msg = b"test message";
        let sig = kp1.sign(msg);

        // Verify with correct key
        let pk1 = iroh::PublicKey::from_bytes(kp1.public.as_bytes()).unwrap();
        assert!(verify_signature_with_iroh_pubkey(&pk1, "node1", "repo", 1, 123, 1, &sig).is_ok());

        // Verify with wrong key fails
        let pk2 = iroh::PublicKey::from_bytes(kp2.public.as_bytes()).unwrap();
        assert!(verify_signature_with_iroh_pubkey(&pk2, "node1", "repo", 1, 123, 1, &sig).is_err());
    }

    #[test]
    fn test_aggregate_signatures() {
        let mut rng = rand::rngs::OsRng;
        let keypairs: Vec<Keypair> = (0..5).map(|_| Keypair::generate(&mut rng)).collect();
        let msg = b"shared message";

        let sigs: Vec<Signature> = keypairs.iter()
            .map(|kp| kp.sign(msg))
            .collect();

        // 3-of-5 threshold
        let agg = aggregate_signatures(&sigs, 3).unwrap();

        // All public keys should verify the aggregate
        for kp in keypairs.iter() {
            let pk = iroh::PublicKey::from_bytes(kp.public.as_bytes()).unwrap();
            assert!(verify_signature_with_iroh_pubkey(&pk, "node", "repo", 1, 123, 1, &agg).is_ok());
        }
    }
}
```

---
