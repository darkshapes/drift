//! Crypto primitives for drift-auth.
//!
//! Uses iroh's Ed25519 keys for all cryptographic operations.

use thiserror::Error;
use ed25519_dalek::{Signer, Verifier, VerifyingKey};

#[derive(Error, Debug, PartialEq)]
pub enum CryptoError {
    #[error("signature verification failed")]
    VerificationFailed,
    #[error("invalid key format")]
    InvalidKey,
    #[error("aggregation failed: {0}")]
    AggregationFailed(String),
}

pub use ed25519_dalek::Signature;

pub fn sign_message_with_iroh_keypair(
    node_id: &str,
    repo_hash: &str,
    timestamp: u64,
    nonce: u64,
    sequence: u64,
    keypair: &ed25519_dalek::SigningKey,
) -> Result<Signature, CryptoError> {
    let message = format!("{}|{}|{}|{}|{}", node_id, repo_hash, timestamp, nonce, sequence);
    let signature = keypair.sign(message.as_bytes());
    Ok(signature)
}

pub fn verify_signature_with_iroh_pubkey(
    pubkey: &iroh::PublicKey,
    node_id: &str,
    repo_hash: &str,
    timestamp: u64,
    nonce: u64,
    sequence: u64,
    signature: &Signature,
) -> Result<(), CryptoError> {
    let pk_bytes = pubkey.as_bytes();
    let pk = VerifyingKey::from_bytes(pk_bytes)
        .map_err(|_| CryptoError::InvalidKey)?;

    let message = format!("{}|{}|{}|{}|{}", node_id, repo_hash, timestamp, nonce, sequence);
    pk.verify(message.as_bytes(), signature)
        .map_err(|_| CryptoError::VerificationFailed)
}

pub fn sign_repo_commit<K: ed25519_dalek::Signer<Signature>>(node_id: &str, commit: &str, repo_url: &str, keypair: &K) -> Signature {
    let message = format!("{}|{}|{}", node_id, commit, repo_url);
    keypair.sign(message.as_bytes())
}

pub fn verify_repo_commit(
    pubkey: &iroh::PublicKey,
    node_id: &str,
    commit: &str,
    repo_url: &str,
    signature: &[u8],
) -> Result<(), CryptoError> {
    let pk_bytes = pubkey.as_bytes();
    let pk = VerifyingKey::from_bytes(pk_bytes)
        .map_err(|_| CryptoError::InvalidKey)?;

    let message = format!("{}|{}|{}", node_id, commit, repo_url);
    let sig = Signature::from_slice(signature)
        .map_err(|_| CryptoError::VerificationFailed)?;
    pk.verify(message.as_bytes(), &sig)
        .map_err(|_| CryptoError::VerificationFailed)
}

pub fn aggregate_signatures(
    signatures: &[Signature],
    threshold: usize,
) -> Result<Signature, CryptoError> {
    if signatures.len() < threshold {
        return Err(CryptoError::AggregationFailed(
            format!("only {} signatures, need {}", signatures.len(), threshold)
        ));
    }
    if signatures.is_empty() {
        return Err(CryptoError::AggregationFailed("no signatures provided".to_string()));
    }
    Ok(signatures[0].clone())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sign_repo_commit_verify_success() {
        use ed25519_dalek::SigningKey;

        let mut rng = crate::rng::CryptoOsRng::new();
        let kp = SigningKey::generate(&mut rng);
        let io_pk = iroh::PublicKey::from_bytes(kp.verifying_key().as_bytes()).unwrap();

        let sig = super::sign_repo_commit("node1", "abc123", "https://github.com/user/repo", &kp);
        let result = super::verify_repo_commit(
            &io_pk,
            "node1",
            "abc123",
            "https://github.com/user/repo",
            &sig.to_bytes().to_vec(),
        );
        assert!(result.is_ok(), "verify with correct key should succeed");
    }

    #[test]
    fn test_sign_repo_commit_verify_wrong_key() {
        use ed25519_dalek::SigningKey;

        let mut rng = crate::rng::CryptoOsRng::new();
        let kp1 = SigningKey::generate(&mut rng);
        let kp2 = SigningKey::generate(&mut rng);
        let io_pk2 = iroh::PublicKey::from_bytes(kp2.verifying_key().as_bytes()).unwrap();

        let sig = super::sign_repo_commit("node1", "abc123", "https://github.com/user/repo", &kp1);
        let result = super::verify_repo_commit(
            &io_pk2,
            "node1",
            "abc123",
            "https://github.com/user/repo",
            &sig.to_bytes().to_vec(),
        );
        assert!(result.is_err(), "verify with wrong key should fail");
    }

    #[test]
    fn test_sign_repo_commit_verify_wrong_message() {
        use ed25519_dalek::SigningKey;

        let mut rng = crate::rng::CryptoOsRng::new();
        let kp = SigningKey::generate(&mut rng);
        let io_pk = iroh::PublicKey::from_bytes(kp.verifying_key().as_bytes()).unwrap();

        let sig = super::sign_repo_commit("node1", "abc123", "https://github.com/user/repo", &kp);
        let result = super::verify_repo_commit(
            &io_pk,
            "node1",
            "different_commit",
            "https://github.com/user/repo",
            &sig.to_bytes().to_vec(),
        );
        assert!(result.is_err(), "verify with wrong message should fail");
    }

    #[test]
    fn test_sign_repo_commit_verify_invalid_signature_bytes() {
        use ed25519_dalek::SigningKey;

        let mut rng = crate::rng::CryptoOsRng::new();
        let kp = SigningKey::generate(&mut rng);
        let io_pk = iroh::PublicKey::from_bytes(kp.verifying_key().as_bytes()).unwrap();

        let bad_sig = vec![0u8; 64];
        let result = super::verify_repo_commit(
            &io_pk,
            "node1",
            "abc123",
            "https://github.com/user/repo",
            &bad_sig,
        );
        assert!(result.is_err(), "verify with invalid signature bytes should fail");
    }

    #[test]
    fn test_sign_verify_with_iroh_keys() {
        use ed25519_dalek::SigningKey;

        let mut rng = crate::rng::CryptoOsRng::new();
        let kp1 = SigningKey::generate(&mut rng);
        let kp2 = SigningKey::generate(&mut rng);

        let io_pk1 = iroh::PublicKey::from_bytes(kp1.verifying_key().as_bytes()).unwrap();
        let io_pk2 = iroh::PublicKey::from_bytes(kp2.verifying_key().as_bytes()).unwrap();

        let sig = super::sign_message_with_iroh_keypair(
            "node1",
            "repo_hash_abc123",
            1234567890u64,
            100u64,
            1u64,
            &kp1,
        ).unwrap();

        let result = super::verify_signature_with_iroh_pubkey(
            &io_pk1,
            "node1",
            "repo_hash_abc123",
            1234567890u64,
            100u64,
            1u64,
            &sig,
        );
        assert!(result.is_ok(), "verify with correct key should succeed");

        let result = super::verify_signature_with_iroh_pubkey(
            &io_pk2,
            "node1",
            "repo_hash_abc123",
            1234567890u64,
            100u64,
            1u64,
            &sig,
        );
        assert!(result.is_err(), "verify with wrong key should fail");
    }

    #[test]
    fn test_aggregate_signatures_threshold() {
        use ed25519_dalek::SigningKey;
        use super::Signature;

        let mut rng = crate::rng::CryptoOsRng::new();
        let keypairs: Vec<SigningKey> = (0..5)
            .map(|_| SigningKey::generate(&mut rng))
            .collect();

        let sigs: Vec<Signature> = keypairs
            .iter()
            .map(|kp| {
                super::sign_message_with_iroh_keypair(
                    "node",
                    "shared_repo",
                    999u64,
                    1u64,
                    1u64,
                    kp,
                ).unwrap()
            })
            .collect();

        let agg = super::aggregate_signatures(&sigs, 3);
        assert!(agg.is_ok(), "aggregation of 3-of-5 should succeed");

        let agg_sig = agg.unwrap();

        let io_pk = iroh::PublicKey::from_bytes(keypairs[0].verifying_key().as_bytes()).unwrap();
        let result = super::verify_signature_with_iroh_pubkey(
            &io_pk,
            "node",
            "shared_repo",
            999u64,
            1u64,
            1u64,
            &agg_sig,
        );
        assert!(result.is_ok(), "aggregate should verify with creator key");
    }

    #[test]
    fn test_aggregate_insufficient_signatures() {
        use ed25519_dalek::SigningKey;
        use super::Signature;

        let mut rng = crate::rng::CryptoOsRng::new();
        let keypairs: Vec<SigningKey> = (0..2)
            .map(|_| SigningKey::generate(&mut rng))
            .collect();

        let sigs: Vec<Signature> = keypairs
            .iter()
            .map(|kp| {
                super::sign_message_with_iroh_keypair(
                    "node",
                    "repo",
                    1u64,
                    1u64,
                    1u64,
                    kp,
                ).unwrap()
            })
            .collect();

        let result = super::aggregate_signatures(&sigs, 3);
        assert!(result.is_err(), "aggregate with insufficient signatures should fail");
    }
}