## 7. Integration with drift-proto (Items 28-32)

**Modify drift-proto/src/lib.rs:**

```rust
// drift-proto/src/lib.rs
// Checklist items: 28, 29, 30, 31, 32

// At top of file, add import for drift-auth
// (Will be added as dependency in Cargo.toml)
// use drift_auth::{AuthMessage, SignedAuthMessage, AggregateAuthMessage, AuthConfig};

// === Item 28: Add DriftMessage variant for auth challenge ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DriftMessage {
    // ... existing variants ...

    /// === NEW: Auth Challenge ===
    /// Coordinator sends to node: "please authenticate"
    AuthChallenge(AuthMessage),

    /// === Item 29: Auth Response ===
    /// Node sends signed auth message to coordinator
    AuthResponse(SignedAuthMessage),

    /// === Item 29: Aggregate Response ===
    /// Coordinator broadcasts aggregate back to all nodes
    AuthAggregate(AggregateAuthMessage),
}

// === Item 30: Add AuthConfig to TrainConfig ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainConfig {
    pub model_path: String,
    pub dataset_path: String,
    pub batch_size: u32,
    pub learning_rate: f64,
    pub epochs: u32,

    // === NEW FIELDS ===
    /// Enable multi-signature authentication
    #[serde(default)]
    pub enable_auth: bool,

    /// Threshold for signature aggregation (e.g., 3 for 3-of-n)
    #[serde(default = "3")]
    pub auth_threshold: usize,

    /// Repository hash to authenticate against
    #[serde(default)]
    pub repo_hash: Option<String>,
}

// === Item 31: Update drift-proto dependencies ===
// In drift-proto/Cargo.toml, add:
// [dependencies]
// drift-auth = { path = "../drift-auth" }
// async-trait = "0.1"

// === Item 32: Integration test ===
// In drift-proto/tests/integration.rs, add:
#[cfg(test)]
mod auth_integration {
    use super::*;

    #[tokio::test]
    async fn test_auth_handshake() {
        // Setup: coordinator + 3 nodes with auth enabled
        // Verify that coordinator collects signatures and broadcasts aggregate
        // Verify all nodes accept the aggregate
        todo!("implement auth integration test")
    }
}
```
