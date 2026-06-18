# Commit Signing Fix

## Goal

Fix signature verification failure when running `drift train` by ensuring RepoCommit signatures are created by drift-node (which has the Ed25519 private key), not drift-cli (which currently uses a SHA256 hash).

## Problem Statement

The current flow has two different signing mechanisms:

- **drift-node**: Uses `sign_repo_commit(node_id, commit, repo_url, keypair)` → creates Ed25519 signature
- **drift-cli**: Uses `sign_with_iroh_key(pubkey, commit, repo_url)` → creates SHA256 hash

When running `drift train`, drift-cli acts as a "node" but lacks access to the iroh private key, so it creates a hash instead of a proper signature. The coordinator's `verify_repo_commit()` expects Ed25519 but receives a hash → verification fails.

## Required Design

1. drift-cli **forwards** TrainConfig to drift-node (no new message type)
2. TrainConfig already contains `train_repo_url`
3. drift-node **owns** `run_git_ls_remote()` to compute commit hash
4. drift-node **signs** RepoCommit with Ed25519 using its private key from `~/.drift/identity/signing_key`
5. drift-node **returns** signed RepoCommit to drift-cli
6. drift-cli **forwards** RepoCommit to coordinator

---

## Stage 1: Modify drift-cli to forward TrainConfig to drift-node

### Steps

1. Add iroh connection to drift-node process in drift-cli
2. Forward TrainConfig message to drift-node
3. Receive RepoCommit response from drift-node
4. Forward RepoCommit to coordinator

### Implementation Table

| Step | Description                                     | Location                                    | Status |
| ---- | ----------------------------------------------- | ------------------------------------------- | ------ |
| 1.1  | Add iroh connection to drift-node               | `drift-cli/src/node.rs:handle_connection()` | [ ]    |
| 1.2  | Forward TrainConfig to drift-node               | `drift-cli/src/node.rs:handle_connection()` | [ ]    |
| 1.3  | Receive signed RepoCommit from drift-node       | `drift-cli/src/node.rs:handle_connection()` | [ ]    |
| 1.4  | Replace local signing with forwarded RepoCommit | `drift-cli/src/node.rs:handle_connection()` | [ ]    |

### Stage Complete When

- drift-cli can connect to drift-node and receive a properly signed RepoCommit
- Coordinator verification passes for both nodes

---

## Stage 2: Verify drift-node handles TrainConfig and returns RepoCommit

### Steps

1. Ensure drift-node accepts TrainConfig from drift-cli
2. drift-node calls run_git_ls_remote to compute commit hash
3. drift-node retrieves signing key from ~/.drift/identity/signing_key
4. drift-node signs RepoCommit with Ed25519
5. drift-node returns signed RepoCommit to drift-cli

### Implementation Table

| Step | Description                               | Location                                        | Status |
| ---- | ----------------------------------------- | ----------------------------------------------- | ------ |
| 2.1  | Accept TrainConfig from drift-cli         | `drift-node/src/network.rs:handle_connection()` | [ ]    |
| 2.2  | Extract repo_url from TrainConfig         | `drift-node/src/network.rs`                     | [ ]    |
| 2.3  | Compute commit hash via run_git_ls_remote | `drift-node/src/network.rs`                     | [ ]    |
| 2.4  | Retrieve signing key from storage         | `drift-node/src/network.rs`                     | [ ]    |
| 2.5  | Sign RepoCommit with Ed25519              | `drift-node/src/network.rs`                     | [ ]    |
| 2.6  | Send RepoCommit back to drift-cli         | `drift-node/src/network.rs`                     | [ ]    |

### Stage Complete When

- drift-node produces identical output to standalone mode
- Ed25519 signature verifies correctly with coordinator

---

## Stage 3: Remove broken sign_with_iroh_key function

### Steps

1. Remove sign_with_iroh_key function from drift-cli
2. Remove sign_with_iroh_key imports if unused
3. Update RepoCommit handling to use forwarded signature
4. Run tests to verify removal doesn't break compilation

### Implementation Table

| Step | Description                        | Location                        | Status |
| ---- | ---------------------------------- | ------------------------------- | ------ |
| 3.1  | Remove sign_with_iroh_key function | `drift-cli/src/node.rs:604-610` | [ ]    |
| 3.2  | Remove SHA256 import if unused     | `drift-cli/src/node.rs`         | [ ]    |
| 3.3  | Update RepoCommit construction     | `drift-cli/src/node.rs:270-278` | [ ]    |
| 3.4  | Verify compilation passes          | Build verification              | [ ]    |

### Stage Complete When

- sign_with_iroh_key is removed
- All tests pass
- No compilation errors

---

## Verification Commands

```bash
cargo build --package drift-cli --package drift-node
cargo test --package drift-cli --package drift-node
cargo check --all
```

## Verification State

- `drift train` completes without "Signature verification failed" error
- Both nodes have identical commit hashes
- Ed25519 signature verification passes for both
