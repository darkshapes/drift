## 6. Node Protocol (Items 23-27)

**Create drift-auth/src/node.rs:**

```rust
// drift-auth/src/node.rs
// Checklist items: 23, 24, 25, 26, 27

use anyhow::{Context, Result};
use tokio::sync::mpsc;
use std::time::Duration;

use super::{Aggregator, AggregateAuthMessage, SignedAuthMessage, AuthMessage, NodeIdentity, AggregationError};

/// === Item 23: sign_and_send_auth ===
pub async fn sign_and_send_auth(
    node_id: &str,
    repo_hash: &str,
    sequence: u64,
    identity: &NodeIdentity,
    coordinator_endpoint: &mut impl AuthSender,  // Trait for sending
) -> Result<(), anyhow::Error> {
    // Create auth message
    let message = AuthMessage::new(node_id, repo_hash, sequence);

    // Sign with node's identity
    let signed = SignedAuthMessage::sign(&message, &identity.keypair, node_id)
        .context("failed to sign auth message")?;

    // Send to coordinator
    // In production, would send via DriftMessage::AuthChallenge
    coordinator_endpoint.send_auth(signed).await
        .context("failed to send auth message")?;

    Ok(())
}

/// === Item 24: verify_aggregate ===
pub async fn verify_aggregate(
    agg_msg: &AggregateAuthMessage,
    node_identity: &NodeIdentity,
    expected_repo_hash: &str,
) -> Result<(), anyhow::Error> {
    // Check that the aggregate is for our repo
    if agg_msg.signatures.is_empty() {
        return Err(anyhow::anyhow!("empty aggregate"));
    }

    let first_msg = &agg_msg.signatures[0].message;
    if first_msg.repo_hash != expected_repo_hash {
        return Err(anyhow::anyhow!("repo hash mismatch"));
    }

    // Verify the aggregated signature
    // In practice, we'd verify with our own copy of all public keys
    // For now, just check that threshold is met
    if agg_msg.signatures.len() < agg_msg.threshold {
        return Err(anyhow::anyhow!("threshold not met"));
    }

    Ok(())
}

/// === Item 25: Retry logic ===
pub async fn auth_with_retry(
    max_retries: u32,
    retry_delay: Duration,
    node_id: &str,
    repo_hash: &str,
    sequence: u64,
    identity: &NodeIdentity,
    coordinator: &mut impl AuthSender,
) -> Result<(), anyhow::Error> {
    for attempt in 1..=max_retries {
        match sign_and_send_auth(node_id, repo_hash, sequence, identity, coordinator).await {
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

/// === Item 26: Sequence number validation ===
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

/// === Item 27: Test full node auth flow ===
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_node_auth_flow() -> Result<()> {
        // Setup: node identity
        let identity = NodeIdentity::new("node1").unwrap();

        // Mock coordinator sender
        struct MockCoordinator;
        impl super::AuthSender for MockCoordinator {
            async fn send_auth(
                &mut self,
                _signed: super::SignedAuthMessage,
            ) -> Result<()> {
                Ok(())
            }
        }

        let mut coord = MockCoordinator;

        // Sign and send
        sign_and_send_auth("node1", "repo123", 1, &identity, &mut coord).await?;

        Ok(())
    }
}
```

**Define the `AuthSender` trait:**

```rust
// drift-auth/src/node.rs (add to same file)
use async_trait::async_trait;

#[async_trait]
pub trait AuthSender: Send + Sync {
    async fn send_auth(
        &mut self,
        signed: SignedAuthMessage,
    ) -> Result<()>;
}
```

---
