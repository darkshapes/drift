## 12. Documentation (Items 54-58)

**Create drift-auth/README.md:**

```markdown
# drift-auth

Multi-signature broadcast authentication for drift distributed training.

## Security Model

- Each node signs with Ed25519 private key
- Coordinator aggregates signatures (m-of-n threshold)
- All nodes verify aggregate before proceeding
- Prevents MITM, replay attacks, ensures consensus

## Threat Model

- **Replay attacks**: Prevented by timestamps + nonces
- **Spoofing**: Prevented by cryptographic signatures
- **Single point of failure**: Threshold allows operation even if some nodes fail

## Quick Start

```rust
use drift_auth::{NodeIdentity, AuthConfig, sign_and_send_auth};

let identity = NodeIdentity::new("node1").unwrap();
let config = AuthConfig::default();

// In node's main loop:
sign_and_send_auth(&identity, &config).await?;
```

## Configuration

See `config.rs` for `AuthConfig` structure.

## Key Management

Keys are stored in `~/.drift/identity.json` (or as configured).
```

---
