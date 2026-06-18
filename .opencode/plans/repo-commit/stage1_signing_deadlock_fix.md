# Stage1-Signing Test Deadlock Fix

## Requirements

- Fix test deadlock in `drift-node/tests/stage1_signing.rs`
- Test should verify end-to-end TrainConfig→RepoCommit→signing flow with iroh keypair
- No separate signing key file should be created or used

---

## Stage 1: Diagnose and Fix Deadlock

### The Problem

The test deadlocks at `node_endpoint.accept()` because:

1. `coord_task` (line 59-95) is spawned but never polled
2. `node_endpoint.accept()` (line 97) is awaited directly, which blocks the runtime
3. `accept()` waits for an incoming connection before returning
4. `coord_task` never runs because the test is blocked waiting on `accept()`
5. Neither task makes progress → deadlock

### Step-by-step

1. Move `node_endpoint.accept()` to a spawned task so it runs concurrently with `coord_task`
2. Capture the accepted connection from the spawned accept task
3. Pass the connection to `handle_connection` after it's available
4. Await both tasks to complete

### Completion tracking

| Step | Description                                    | %   | Line refs                                    |
| ---- | ---------------------------------------------- | --- | -------------------------------------------- |
| 1.1  | Spawn accept as background task                | 0%  | `drift-node/tests/stage1_signing.rs:97`      |
| 1.2  | Await spawned accept task before getting conn  | 0%  | `drift-node/tests/stage1_signing.rs:97-98`   |
| 1.3  | Spawn handle_connection with the accepted conn | 0%  | `drift-node/tests/stage1_signing.rs:106-108` |

**Stage 1 complete when:** Test no longer deadlocks at `accept()`

---

## Stage 2: Verify End-to-End Flow

### Step-by-step

1. Build workspace: `cargo build --workspace`
2. Run the test: `cargo test --package drift_node test_repo_commit_signed_with_iroh_key`
3. Verify TrainConfig is sent with `train_repo_url`
4. Verify Node receives config, calls `get_git_commit`, signs with iroh key
5. Verify Node sends `RepoCommit` back to coordinator
6. Verify coordinator verifies signature with node's iroh public key

### Completion tracking

| Step | Description                   | %   | Line refs                                  |
| ---- | ----------------------------- | --- | ------------------------------------------ |
| 2.1  | Build workspace               | 0%  | `cargo build --workspace`                  |
| 2.2  | Run test                      | 0%  | `cargo test --package drift_node`          |
| 2.3  | Verify TrainConfig flow       | 0%  | `drift-node/src/network.rs:63-112`         |
| 2.4  | Verify RepoCommit response    | 0%  | `drift-node/src/network.rs:90-112`         |
| 2.5  | Verify signature verification | 0%  | `drift-node/tests/stage1_signing.rs:89-94` |

**Stage 2 complete when:** Test passes, verifying full TrainConfig→RepoCommit flow with iroh keypair signing.

---

## Overall Progress

- [ ] Stage 1: Fix accept deadlock
- [ ] Stage 2: Verify end-to-end flow
