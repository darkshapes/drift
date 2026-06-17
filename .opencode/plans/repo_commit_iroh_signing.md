# Drift Train: RepoCommit Signing with Iroh Keypair

## Requirements

- Sign `node_id|commit|repo_url` using node's **iroh private key** (no separate signing key)
- Coordinator verifies using node's **iroh public key**
- Happens **automatically** when TrainConfig is received
- All other behavior stays the same

---

## Stage 1: Update drift-node to sign RepoCommit with iroh keypair

### Step-by-step

1. Remove `NODE_SIGNING_KEY` static mutex and related functions (`set_signing_key`, `get_signing_key`, `load_or_create_signing_key`, `signing_key_path`) from `drift-node/src/network.rs`
2. Remove `load_or_create_keypair` and `create_endpoint_with_keypair` from `drift-node/src/network.rs` (use iroh endpoint directly)
3. Update `drift-node/src/main.rs` to create endpoint without separate signing key (use `create_endpoint()` or inline the logic)
4. In `drift-node/src/network.rs` `handle_connection`, when TrainConfig with `train_repo_url` is received:
   - Get git commit via `get_git_commit`
   - Extract node's iroh private key from endpoint (via `conn.local_id()` or endpoint reference)
   - Convert iroh private key to `ed25519_dalek::SigningKey`
   - Call `drift_auth::crypto::sign_repo_commit(node_id, commit, repo_url, &keypair)`
   - Build `RepoCommit { commit, repo_url, signature }`
   - Send `DriftMessage::RepoCommit(repo_commit)` to coordinator
   - Start standby timer for TrainingReady

### Completion tracking

| Step | Description                                       | %   | Line refs                           |
| ---- | ------------------------------------------------- | --- | ----------------------------------- |
| 1.1  | Remove signing key infrastructure from network.rs | 0%  | `drift-node/src/network.rs:12-48`   |
| 1.2  | Update main.rs to use `create_endpoint()`         | 0%  | `drift-node/src/main.rs:76-77`      |
| 1.3  | Extract iroh private key in handle_connection     | 0%  | `drift-node/src/network.rs:79-290`  |
| 1.4  | Sign with iroh keypair and send RepoCommit        | 0%  | `drift-node/src/network.rs:120-166` |

**Stage 1 complete when:** Node sends `RepoCommit` signed with its iroh keypair when receiving `TrainConfig` with `train_repo_url`, without using any separate signing key file.

---

## Stage 2: Update coordinator to verify RepoCommit with iroh public key

### Step-by-step

1. In `drift-coord/src/main.rs` (or connection handler), when receiving `DriftMessage::RepoCommit`:
   - Extract node's iroh public key from the connection (`conn.remote_id()` or stored during NodeInfo)
   - Call `drift_auth::crypto::verify_repo_commit(&iroh_pubkey, node_id, commit, repo_url, &signature)`
2. If verification succeeds:
   - Store verified commit hash in TrainConfig (`git_commit`)
   - Wait for all nodes to report same commit
   - Broadcast `TrainingReady` when consensus reached
3. If verification fails:
   - Broadcast `TrainingCancel` to all nodes with reason `"signature verification failed"`
4. Update coordinator message handling loop to process `RepoCommit` messages

### Completion tracking

| Step | Description                                                       | %   | Line refs                                     |
| ---- | ----------------------------------------------------------------- | --- | --------------------------------------------- |
| 2.1  | Handle RepoCommit in coordinator message loop                     | 0%  | `drift-coord/src/main.rs` (message handling)  |
| 2.2  | Extract iroh public key from node connection                      | 0%  | `drift-coord/src/main.rs` (NodeInfo handling) |
| 2.3  | Verify signature with `verify_repo_commit`                        | 0%  | `drift-auth/src/crypto.rs:61-77`              |
| 2.4  | Broadcast TrainingReady on consensus or TrainingCancel on failure | 0%  | `drift-coord/src/main.rs`                     |

**Stage 2 complete when:** Coordinator verifies all `RepoCommit` signatures using iroh public keys, broadcasts `TrainingReady` when all nodes agree on commit, or `TrainingCancel` on verification failure.

---

## Stage 3: Clean up unused signing key infrastructure

### Step-by-step

1. Remove `signing_key_path()` function
2. Remove `.drift/identity/signing_key` file creation/loading
3. Remove `NODE_SIGNING_KEY` static mutex
4. Update any remaining imports or references to old signing key functions
5. Verify `drift-auth/src/crypto.rs` functions work with iroh `PublicKey` (already implemented)

### Completion tracking

| Step | Description                                | %   | Line refs                                             |
| ---- | ------------------------------------------ | --- | ----------------------------------------------------- |
| 3.1  | Remove signing key file operations         | 0%  | `drift-node/src/network.rs:36-48`                     |
| 3.2  | Remove static mutex and accessors          | 0%  | `drift-node/src/network.rs:12-44`                     |
| 3.3  | Clean up imports in network.rs and main.rs | 0%  | `drift-node/src/main.rs`, `drift-node/src/network.rs` |

**Stage 3 complete when:** No separate signing key infrastructure remains; all signing uses iroh keypair.

---

## Stage 4: Update tests

### Step-by-step

1. Review `drift-proto/tests/repo_commit_verification.rs` - ensure tests still pass with iroh keypairs
2. Review `drift-auth/src/node.rs` tests - update any that use separate signing keys
3. Run `cargo test` in all crates
4. Fix any compilation errors from removed signing key API

### Completion tracking

| Step | Description                       | %   | Line refs                                                                 |
| ---- | --------------------------------- | --- | ------------------------------------------------------------------------- |
| 4.1  | Run tests and identify failures   | 0%  | All test files                                                            |
| 4.2  | Update tests to use iroh keypairs | 0%  | `drift-proto/tests/repo_commit_verification.rs`, `drift-auth/src/node.rs` |
| 4.3  | Verify all tests pass             | 0%  | `cargo test --workspace`                                                  |

**Stage 4 complete when:** All tests pass with iroh keypair-based signing.

---

## Stage 5: Integration verification

### Step-by-step

1. Build entire workspace: `cargo build --workspace`
2. Run all tests: `cargo test --workspace`
3. Verify TrainConfig flow end-to-end:
   - Coordinator sends TrainConfig with `train_repo_url`
   - Node receives TrainConfig, fetches commit, signs with iroh key
   - Node sends RepoCommit
   - Coordinator verifies with iroh pubkey
   - Coordinator broadcasts TrainingReady or TrainingCancel
4. Verify no separate signing key file is created or used

### Completion tracking

| Step | Description             | %   | Line refs                                           |
| ---- | ----------------------- | --- | --------------------------------------------------- |
| 5.1  | Build workspace         | 0%  | `cargo build --workspace`                           |
| 5.2  | Run all tests           | 0%  | `cargo test --workspace`                            |
| 5.3  | Manual integration test | 0%  | `drift-coord/src/main.rs`, `drift-node/src/main.rs` |

**Stage 5 complete when:** Build succeeds, all tests pass, and end-to-end TrainConfig→RepoCommit→TrainingReady flow works with iroh keypairs.

---

## Overall Progress

- [ ] Stage 1: drift-node signs with iroh keypair
- [ ] Stage 2: coordinator verifies with iroh pubkey
- [ ] Stage 3: remove separate signing key infrastructure
- [ ] Stage 4: update tests
- [ ] Stage 5: integration verification
