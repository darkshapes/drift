# Stage1-Signing DNS Fix

## Requirements

- Fix DNS resolution failure in `drift-node/tests/stage1_signing.rs`
- Test should verify end-to-end TrainConfig→RepoCommit→signing flow with iroh keypair
- No separate signing key file should be created or used

---

## Stage 1: Fix DNS Resolution

### The Problem

The test fails because `Endpoint::builder()` includes DNS resolution:
1. Both endpoints use `Endpoint::builder().alpns(...).bind()` with default DNS
2. `coord_endpoint.connect(node_pubkey_for_coord, DRIFT_ALPN)` triggers DNS lookup
3. DNS fails with `No addressing information available` - no TXT records exist

### Step-by-step

1. Replace `Endpoint::builder()` with `Endpoint::empty_builder(RelayMode::Disabled)` in `create_endpoint`
2. Get node's listening address from `endpoint.addr()`
3. Connect using `EndpointAddr::from_parts(peer_id, peer_addr)` directly
4. Remove DNS dependency from test

### Completion tracking

| Step | Description                                  | %   | Line refs                                   |
| ---- | ---------------------------------------------- | --- | ------------------------------------------- |
| 1.1  | Replace Endpoint::builder with empty_builder | 0%  | `drift-node/src/network.rs:9-13`            |
| 1.2  | Get node's addr() after binding              | 0%  | `drift-node/src/network.rs:15-18`         |
| 1.3  | Connect using direct EndpointAddr           | 0%  | `drift-node/tests/stage1_signing.rs:60`   |

**Stage 1 complete when:** Test connects without DNS lookup

---

## Stage 2: Verify End-to-End Flow

### Step-by-step

1. Build workspace: `cargo build --workspace`
2. Run the test: `cargo test --package drift-node test_repo_commit_signed_with_iroh_key`
3. Verify TrainConfig is sent with `train_repo_url`
4. Verify Node receives config, calls `get_git_commit`, signs with iroh key
5. Verify Node sends `RepoCommit` back to coordinator
6. Verify coordinator verifies signature with node's iroh public key

### Completion tracking

| Step | Description                           | %   | Line refs                                     |
| ---- | ------------------------------------- | --- | -------------------------------------------- |
| 2.1  | Build workspace                      | 0%  | `cargo build --workspace`                   |
| 2.2  | Run test                             | 0%  | `cargo test --package drift-node`          |
| 2.3  | Verify TrainConfig flow              | 0%  | `drift-node/src/network.rs:63-112`        |
| 2.4  | Verify RepoCommit response          | 0%  | `drift-node/src/network.rs:90-112`       |
| 2.5  | Verify signature verification        | 0%  | `drift-node/tests/stage1_signing.rs:89-94`|

**Stage 2 complete when:** Test passes, verifying full TrainConfig→RepoCommit flow with iroh keypair signing.

---

## Overall Progress

- [ ] Stage 1: Fix DNS resolution
- [ ] Stage 2: Verify end-to-end flow
