# COVEN/drift Multi-Signature Broadcast Implementation Checklist

Implementation of a new `drift-auth` crate for distributed consensus built on top of iroh's transport security.

---

## Overview

The iroh library provides:

- Transport layer: QUIC-based peer-to-peer connections with relay support
- Encryption: TLS 1.3 with each endpoint having an Ed25519 keypair
- Endpoint authentication: Each connection authenticates the remote endpoint's PublicKey (EndpointId)
- Stream abstraction: Uni/bi-directional streams for message passing

**What drift-auth adds on top of iroh:**

- Multi-signature aggregation (threshold m-of-n)
- Custom handshake protocols for consensus
- Replay protection with nonces
- Sequence number validation

**Crate name:** `drift-auth` (to be added to workspace)

**Key Design Principle:** Use iroh's existing Ed25519 keys as node identity. Each node already has an Ed25519 keypair from iroh. The drift-auth layer does NOT create separate keys; it uses iroh's keys for signing and verification.

---

## Implementation Plan Reference

For detailed implementation guidance, see:

- **[Plan 1](drift-auth-apply-step-01.md)**: Workspace Setup
- **[Plan 2](drift-auth-apply-step-02.md)**: Crypto Primitives (Items 1-5)
- **[Plan 3](drift-auth-apply-step-03.md)**: Message Structures (Items 6-10)
- **[Plan 4](drift-auth-apply-step-04.md)**: Coordinator Aggregation (Items 11-16)
- **[Plan 5](drift-auth-apply-step-05.md)**: Node Protocol (Items 17-21)
- **[Plan 6](drift-auth-apply-step-06.md)**: Node Protocol (Items 23-27)
- **[Plan 7](drift-auth-apply-step-07.md)**: Integration with drift-proto (Items 28-32)
- **[Plan 8](drift-auth-apply-step-08.md)**: Coordinator Protocol (Items 33-37)
- **[Plan 9](drift-auth-apply-step-09.md)**: Replay Prevention (Items 38-42)
- **[Plan 10](drift-auth-apply-step-10.md)**: Configuration (Items 43-47)
- **[Plan 11](drift-auth-apply-step-11.md)**: Testing (Items 48-53)
- **[Plan 12](drift-auth-apply-step-12.md)**: Documentation (Items 54-58)
- **[Plan 13](drift-auth-apply-step-13.md)**: Integration (Items 59-63)
- **[Plan 14](drift-auth-apply-step-14.md)**: Security Audit (Items 64-68)

---

## Workspace Setup

| #   | Task                                                       | Completion % | Reference                             |
| --- | ---------------------------------------------------------- | ------------ | ------------------------------------- |
| 1   | Add `drift-auth` to workspace members in root `Cargo.toml` | 100%         | [Plan 1](drift-auth-apply-step-01.md) |

---

## Crypto Primitives

| #   | Task                                                                       | Completion % | Reference                             |
| --- | -------------------------------------------------------------------------- | ------------ | ------------------------------------- |
| 1   | Use existing iroh Ed25519 keys via `iroh::PublicKey` and signing API       | 100%         | [Plan 2](drift-auth-apply-step-02.md) |
| 2   | Implement `sign_message(io_pubkey, repo_hash, timestamp, nonce, sequence)` | 100%         | [Plan 2](drift-auth-apply-step-02.md) |
| 3   | Implement `verify_signature(io_pubkey, message, signature)`                | 100%         | [Plan 2](drift-auth-apply-step-02.md) |
| 4   | Implement `aggregate_signatures([Signature])` for threshold m-of-n         | 100%         | [Plan 2](drift-auth-apply-step-02.md) |
| 5   | Add unit tests for sign/verify/aggregate                                   | 100%         | [Plan 2](drift-auth-apply-step-02.md) |

---

## Message Structures

| #   | Task                                                                                    | Completion % | Reference                             |
| --- | --------------------------------------------------------------------------------------- | ------------ | ------------------------------------- |
| 6   | Define `AuthMessage` struct: `node_id`, `repo_hash`, `timestamp`, `nonce`, `sequence`   | 100%         | [Plan 3](drift-auth-apply-step-03.md) |
| 7   | Define `SignedAuthMessage`: `AuthMessage` + `signature` + `node_id`                     | 100%         | [Plan 3](drift-auth-apply-step-03.md) |
| 8   | Define `AggregateAuthMessage`: collection of `SignedAuthMessage` + aggregated signature | 100%         | [Plan 3](drift-auth-apply-step-03.md) |
| 9   | Implement `serialize/deserialize` for all message types                                 | 100%         | [Plan 3](drift-auth-apply-step-03.md) |
| 10  | Add tests for message serialization and round-trip                                      | 100%         | [Plan 3](drift-auth-apply-step-03.md) |

---

## Key Management

**No separate key management needed** - each node uses its existing iroh Ed25519 keypair. The drift-auth crate works with `iroh::PublicKey` directly.

---

## Coordinator Aggregation Logic

| #   | Task                                                                  | Completion % | Reference                             |
| --- | --------------------------------------------------------------------- | ------------ | ------------------------------------- |
| 11  | Create `Aggregator` struct: tracks collected signatures per round     | 100%         | [Plan 4](drift-auth-apply-step-04.md) |
| 12  | Implement `add_signature(node_id, SignedAuthMessage)` -> `Result<()>` | 100%         | [Plan 4](drift-auth-apply-step-04.md) |
| 13  | Implement `check_threshold()`: have we reached m-of-n?                | 100%         | [Plan 4](drift-auth-apply-step-04.md) |
| 14  | Implement `create_aggregate()` -> `AggregateAuthMessage`              | 100%         | [Plan 4](drift-auth-apply-step-04.md) |
| 15  | Add timeout handling for signature collection                         | 100%         | [Plan 4](drift-auth-apply-step-04.md) |
| 16  | Test aggregation with 3-of-5 threshold                                | 100%         | [Plan 4](drift-auth-apply-step-04.md) |

---

## Node-Side Protocol

| #   | Task                                                           | Completion % | Reference                             |
| --- | -------------------------------------------------------------- | ------------ | ------------------------------------- |
| 17  | Implement `sign_and_send_auth(io_stream, repo_hash, sequence)` | 100%         | [Plan 5](drift-auth-apply-step-05.md) |
| 18  | Implement `verify_aggregate(AggregateAuthMessage)`             | 100%         | [Plan 5](drift-auth-apply-step-05.md) |
| 19  | Add retry logic for failed verification (max N retries)        | 100%         | [Plan 5](drift-auth-apply-step-05.md) |
| 20  | Add sequence number validation (detect reordering)             | 100%         | [Plan 5](drift-auth-apply-step-05.md) |
| 21  | Test full node auth flow: sign, receive aggregate, verify      | 100%         | [Plan 5](drift-auth-apply-step-05.md) |

---

## Node Protocol (Extended)

| #   | Task                                                                      | Completion % | Reference                             |
| --- | ------------------------------------------------------------------------- | ------------ | ------------------------------------- |
| 23  | Implement `sign_and_send_auth` with `NodeIdentity` and `AuthSender` trait | 100%         | [Plan 6](drift-auth-apply-step-06.md) |
| 24  | Implement `verify_aggregate` with proper signature validation             | 100%         | [Plan 6](drift-auth-apply-step-06.md) |
| 25  | Add retry logic for auth operations                                       | 100%         | [Plan 6](drift-auth-apply-step-06.md) |
| 26  | Add sequence number validation                                            | 100%         | [Plan 6](drift-auth-apply-step-06.md) |
| 27  | Test full node auth flow with `NodeIdentity`                              | 100%         | [Plan 6](drift-auth-apply-step-06.md) |

---

## Integration with drift-proto

| #   | Task                                                                       | Completion % | Reference                             |
| --- | -------------------------------------------------------------------------- | ------------ | ------------------------------------- |
| 28  | Add new `DriftMessage` variant: `AuthChallenge(AuthMessage)`               | 100%         | [Plan 7](drift-auth-apply-step-07.md) |
| 29  | Add new `DriftMessage` variant: `AuthResponse(AggregateAuthMessage)`       | 100%         | [Plan 7](drift-auth-apply-step-07.md) |
| 30  | Add `AuthConfig` to `TrainConfig`: `enable_auth: bool`, `threshold: usize` | 100%         | [Plan 7](drift-auth-apply-step-07.md) |
| 31  | Update `drift-proto` with new message types and dependencies               | 100%         | [Plan 7](drift-auth-apply-step-07.md) |
| 32  | Integration test: full handshake with auth messages                        | 100%         | [Plan 7](drift-auth-apply-step-07.md) |

---

## Coordinator-Side Protocol

| #   | Task                                                                               | Completion % | Reference                             |
| --- | ---------------------------------------------------------------------------------- | ------------ | ------------------------------------- |
| 33  | Implement `collect_signatures_from_nodes(timeout) -> Result<AggregateAuthMessage>` | 70%          | [Plan 8](drift-auth-apply-step-08.md) |
| 34  | Implement `broadcast_aggregate_to_all_nodes(AggregateAuthMessage)`                 | 0%           | [Plan 8](drift-auth-apply-step-08.md) |
| 35  | Add logging: which nodes have signed, missing signatures                           | 70%          | [Plan 8](drift-auth-apply-step-08.md) |
| 36  | Handle coordinator key rotation                                                    | 0%            | [Plan 8](drift-auth-apply-step-08.md) |
| 37  | Test coordinator auth flow                                                         | 100%          | [Plan 8](drift-auth-apply-step-08.md) |

---

## Replay Attack Prevention

| #   | Task                                                       | Completion % | Reference                             |
| --- | ---------------------------------------------------------- | ------------ | ------------------------------------- |
| 38  | Ensure timestamps are validated (e.g., within 5min window) | 100%         | [Plan 9](drift-auth-apply-step-09.md) |
| 39  | Ensure nonces are never reused (track seen nonces)         | 100%         | [Plan 9](drift-auth-apply-step-09.md) |
| 40  | Implement `NonceStore` with TTL to detect replays          | 100%         | [Plan 9](drift-auth-apply-step-09.md) |
| 41  | Test replay attack: reject duplicate nonce                 | 100%         | [Plan 9](drift-auth-apply-step-09.md) |
| 42  | Test expired timestamp rejection                           | 100%         | [Plan 9](drift-auth-apply-step-09.md) |

---

## Configuration & Error Handling

| #   | Task                                                              | Completion % | Reference                              |
| --- | ----------------------------------------------------------------- | ------------ | -------------------------------------- |
| 43  | Define `AuthConfig` struct: `threshold`, `key_rotation_interval`  | 100%         | [Plan 10](drift-auth-apply-step-10.md) |
| 44  | Add config toml/json support                                      | 100%         | [Plan 10](drift-auth-apply-step-10.md) |
| 45  | Define error types: `AuthError`, `SignatureError`, `TimeoutError` | 100%         | [Plan 10](drift-auth-apply-step-10.md) |
| 46  | Implement user-friendly error messages                            | 100%         | [Plan 10](drift-auth-apply-step-10.md) |
| 47  | Add metrics: number of nodes authenticated, time to consensus     | 100%         | [Plan 10](drift-auth-apply-step-10.md) |

---

## Testing

| #   | Task                                                     | Completion % | Reference                              |
| --- | -------------------------------------------------------- | ------------ | -------------------------------------- |
| 48  | Unit tests: sign/verify in isolation                     | 100%         | [Plan 11](drift-auth-apply-step-11.md) |
| 49  | Unit tests: aggregate signatures (2-of-3, 3-of-5)        | 100%         | [Plan 11](drift-auth-apply-step-11.md) |
| 50  | Integration test: 5 nodes, threshold 3, one node offline | 100%         | [Plan 11](drift-auth-apply-step-11.md) |
| 51  | Fuzzing test: malformed messages, edge cases             | 100%         | [Plan 11](drift-auth-apply-step-11.md) |
| 52  | Performance test: measure auth overhead per node         | 100%         | [Plan 11](drift-auth-apply-step-11.md) |
| 53  | Test key rotation during operation                       | 100%         | [Plan 11](drift-auth-apply-step-11.md) |

---

## Documentation

| #   | Task                                                   | Completion % | Reference                              |
| --- | ------------------------------------------------------ | ------------ | -------------------------------------- |
| 54  | Write crate-level documentation (`README.md`)          | 0%            | [Plan 12](drift-auth-apply-step-12.md) |
| 55  | Document security model and threat model               | 0%            | [Plan 12](drift-auth-apply-step-12.md) |
| 56  | Add examples: basic auth flow, threshold configuration | 0%            | [Plan 12](drift-auth-apply-step-12.md) |
| 57  | Document key generation and storage locations          | 0%            | [Plan 12](drift-auth-apply-step-12.md) |
| 58  | Add API docs with `cargo doc`                          | 0%            | [Plan 12](drift-auth-apply-step-12.md) |

---

## Integration with Existing drift

| #   | Task                                                              | Completion % | Reference                              |
| --- | ----------------------------------------------------------------- | ------------ | -------------------------------------- |
| 59  | Modify `drift-node` to use `drift-auth` after iroh connection     | 0%            | [Plan 13](drift-auth-apply-step-13.md) |
| 60  | Modify `drift-coord` to aggregate signatures                      | 80%           | [Plan 13](drift-auth-apply-step-13.md) |
| 61  | Add auth setup to `TrainConfig` in `drift-proto`                  | 100%          | [Plan 13](drift-auth-apply-step-13.md) |
| 62  | Test full system: coordinator + 3 nodes with auth enabled         | 100%          | [Plan 13](drift-auth-apply-step-13.md) |
| 63  | Test graceful degradation: auth disabled (backward compatibility) | 100%          | [Plan 13](drift-auth-apply-step-13.md) |

---

## Security Audit

| #   | Task                                                    | Completion % | Reference                              |
| --- | ------------------------------------------------------- | ------------ | -------------------------------------- |
| 64  | Verify constant-time comparisons for signatures         | 100%         | [Plan 14](drift-auth-apply-step-14.md) |
| 65  | Ensure private keys never logged or sent over network   | 100%         | [Plan 14](drift-auth-apply-step-14.md) |
| 66  | Check for side-channel leaks in signature verification  | 100%         | [Plan 14](drift-auth-apply-step-14.md) |
| 67  | Review nonce generation: use `rand` or `getrandom`      | 100%         | [Plan 14](drift-auth-apply-step-14.md) |
| 68  | Ensure iroh's existing transport security is maintained | 100%         | [Plan 14](drift-auth-apply-step-14.md) |

---

## Execution Order

1. **Crypto primitives** (1-5): Use iroh keys, sign/verify, aggregate
2. **Message structures** (6-10): Define signed and aggregate message types
3. **Coordinator aggregation** (11-16): Collect, threshold, broadcast
4. **Node protocol** (17-21): Node-side sign/verify flow
5. **drift-proto integration** (22-27): New message variants and config
6. **Coordinator protocol** (28-31): Coordinator-side collection and broadcast
7. **Replay prevention** (32-36): Timestamp, nonce, sequence validation
8. **Configuration** (37-41): Config structs, error handling
9. **Testing** (42-47): Comprehensive test suite
10. **Documentation** (48-52): User guides and API docs
11. **Integration** (53-57): Wire into existing node and coordinator binaries
12. **Security audit** (58-62): Final review before merge

---

## Verification Checklist

- [ ] All unit tests pass (`cargo test --all`)
- [ ] Integration test: full auth handshake with threshold 3-of-5
- [ ] Performance overhead < 100ms per node for auth phase
- [ ] Replay attacks are rejected
- [ ] Expired timestamps are rejected
- [ ] Coordinator can collect signatures even if some nodes fail
- [ ] Backward compatibility: system works with auth disabled
- [ ] No secrets logged or leaked in error messages
- [ ] iroh mTLS connection established before auth handshake
- [ ] All signatures verify with iroh public keys

---

## Dependencies to add to `Cargo.toml`

```toml
[dependencies]
ed25519-dalek = { version = "2", features = ["rand_core"] }
rand = "0.8"
thiserror = "1.0"
serde = { workspace = true }
lru = "0.1"
async-trait = "0.1"
```

Add `drift-auth` to workspace members in root `Cargo.toml`.
