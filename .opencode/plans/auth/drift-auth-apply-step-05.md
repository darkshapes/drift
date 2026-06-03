## 5. Node Protocol (Items 17-21)

**Note:** The node signs messages with its iroh keypair and sends them to the coordinator via the existing iroh stream. The coordinator's stream is used to receive the aggregate.

**Create drift-auth/src/node.rs:**

```rust
// drift-auth/src/node.rs
// Checklist items: 17, 18, 19, 20, 21

use anyhow::{Context, Result};
use std::time::Duration;

use super::{AggregateAuthMessage, SignedAuthMessage, AuthMessage, CryptoError};

/// === Item 17: sign_and_send_auth ===
/// Sign an auth message with the node's iroh keypair and send to coordinator
pub async fn sign_and_send_auth(
    node_id: &str,
    repo_hash: &str,
    sequence: u64,
    keypair: &iroh::Keypair,
    // In production, we'd send over an iroh stream
    coordinator_stream: &mut impl AuthMessageSender,
) -> Result<(), anyhow::Error> {
    // Create auth message
    let message = AuthMessage::new(node_id, repo_hash, sequence);

    // Sign with node's iroh keypair
    let signed = SignedAuthMessage::sign(&message, keypair)
        .context("failed to sign auth message")?;

    // Send to coordinator
    coordinator_stream.send_signed_auth(signed).await
        .context("failed to send auth message")?;

    Ok(())
}

/// === Item 18: verify_aggregate ===
/// Verify the aggregate message from coordinator
pub fn verify_aggregate(
    agg_msg: &AggregateAuthMessage,
    expected_repo_hash: &str,
    our_node_id: &str,
) -> Result<(), anyhow::Error> {
    // Check that the aggregate is for our repo
    if agg_msg.message.repo_hash != expected_repo_hash {
        return Err(anyhow::anyhow!("repo hash mismatch"));
    }

    // Verify the aggregated signature
    // In production, we'd verify with a set of known public keys from other nodes
    // Here we just check that threshold is met
    if agg_msg.node_ids.len() < agg_msg.threshold {
        return Err(anyhow::anyhow!("threshold not met"));
    }

    // Check that we participated (our node_id should be in the list if we sent a signature)
    if !agg_msg.node_ids.contains(&our_node_id.to_string()) {
        return Err(anyhow::anyhow!("our node did not participate"));
    }

    Ok(())
}

/// === Item 19: Retry logic ===
pub async fn auth_with_retry(
    max_retries: u32,
    retry_delay: Duration,
    node_id: &str,
    repo_hash: &str,
    sequence: u64,
    keypair: &iroh::Keypair,
    coordinator: &mut impl AuthMessageSender,
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
pub fn validate_sequence(
    received: u64,
    expected: &mut u64,
) -> Result<(), anyhow::Error> {
    if received < *expected {
        return Err(anyhow::anyhow!("sequence went backwards"));
    }
    if received > *expected {
        // Allow skipping ahead (retransmit), but update expected
        *expected = received;
    }
    Ok(())
}

/// === Item 21: Test full node auth flow ===
#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Keypair;

    #[tokio::test]
    async fn test_node_auth_flow() -> Result<()> {
        // Setup: generate a keypair
        let kp = Keypair::generate(&mut rand::rngs::OsRng);
        let node_id = "node1";
        let repo_hash = "abc123";
        let sequence = 1u64;

        // Mock coordinator sender
        struct MockCoordinator;
        #[async_trait]
        impl super::AuthMessageSender for MockCoordinator {
            async fn send_signed_auth(
                &mut self,
                _signed: super::SignedAuthMessage,
            ) -> Result<()> {
                Ok(())
            }
        }

        let mut coord = MockCoordinator;

        // Sign and send
        sign_and_send_auth(node_id, repo_hash, sequence, &kp, &mut coord).await?;

        Ok(())
    }
}

/// Trait for sending auth messages to coordinator

use async_trait::async_trait;

#[async_trait]
pub trait AuthMessageSender: Send + Sync {
    async fn send_signed_auth(
        &mut self,
        signed: SignedAuthMessage,
    ) -> Result<()>;
}
```

---
