## 13. Integration (Items 59-63)

**Modify drift-node/src/main.rs:**

```rust
// In join() function, after receiving TrainConfig:
use drift_auth::{AuthConfig, NodeIdentity};

async fn join(name: Option<String>) -> Result<()> {
    // ... existing code ...

    // Check if auth is enabled
    if train_config.enable_auth {
        // Load or create identity
        let identity = NodeIdentity::load_or_create(&node_id_str)
            .context("failed to load identity")?;

        // Perform auth handshake
        match perform_auth_handshake(
            &mut send_stream,
            &identity,
            &auth_message,
            train_config.auth_threshold,
        ).await {
            Ok(()) => println!("auth successful"),
            Err(e) => {
                eprintln!("auth failed: {}", e);
                return Err(e.into());
            }
        }
    }

    // ... continue with training ...
}
```

**Modify drift-coord/src/main.rs:**

```rust
// In train() function:
use drift_auth::{CoordinatorAuth, AggregationError};

async fn train(...) -> Result<()> {
    // ... existing setup ...

    if train_config.enable_auth {
        let mut auth = CoordinatorAuth::new(train_config);

        // Wait for all nodes to send AuthResponse
        let agg = auth.collect_signatures(
            peer_node_ids,
            Duration::from_secs(60),
        ).await?;

        // Broadcast aggregate back to all nodes
        auth.broadcast_aggregate(&agg).await?;
    }

    // ... continue ...
}
```

---
