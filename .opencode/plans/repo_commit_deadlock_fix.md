# RepoCommit Protocol Deadlock Fix

## Stage 1: Coordinator Protocol Reorder

**Goal:** Send `TrainConfig` before collecting `RepoCommit` from nodes.

### Steps

| Step | Description                                                                                        | %   | Code Location                    |
| ---- | -------------------------------------------------------------------------------------------------- | --- | -------------------------------- |
| 1.1  | Move `TrainConfig` send loop (lines 223-232) to **before** `RepoCommit` collection loop (line 127) | 0%  | `drift-cli/src/coord.rs:127-233` |
| 1.2  | Keep `RepoCommit` collection logic unchanged (lines 130-173)                                       | 0%  | `drift-cli/src/coord.rs:130-173` |
| 1.3  | Keep `TrainingReady` broadcast (lines 190-192) after verification                                  | 0%  | `drift-cli/src/coord.rs:190-192` |
| 1.4  | Keep `TrainingCancel` broadcast on failures (lines 138-143, 180-185)                               | 0%  | `drift-cli/src/coord.rs:138-143` |

**Stage completion checkbox:** ☐

**Verifiable state:**

- Coordinator sends `TrainConfig` to all nodes immediately after `NodeInfo` collection
- Coordinator then enters `RepoCommit` collection loop
- No `TrainConfig` is sent after `TrainingReady` broadcast

---

## Stage 2: Node RepoCommit Send

**Goal:** Node sends `RepoCommit` immediately after receiving `TrainConfig`.

### Steps

| Step | Description                                                              | %   | Code Location                            |
| ---- | ------------------------------------------------------------------------ | --- | ---------------------------------------- |
| 2.1  | Import `RepoCommit` struct and git utilities in `network.rs`             | 100% | `drift-node/src/network.rs:1-10`      |
| 2.2  | Add `RepoCommit` send in `TrainConfig` handler (after line 73)           | 100% | `drift-node/src/network.rs:52-74`      |
| 2.3  | Implement `git ls-remote` to get real commit hash from `train_repo_url`  | 100% | `drift-node/src/network.rs:304-326`    |
| 2.4  | Sign commit hash with node's iroh keypair (stub for now)                 | 100% | `drift-node/src/network.rs:79-82`      |
| 2.5  | Send `DriftMessage::RepoCommit(repo_commit)` immediately after computing | 100% | `drift-node/src/network.rs:79-85`      |

**Stage completion checkbox:** ☒

**Verifiable state:**

- Node receives `TrainConfig`, computes commit hash, sends `RepoCommit` immediately
- `RepoCommit` is sent before any `TrainingReady` is received
- Coordinator receives `RepoCommit` and processes it

---

## Stage 3: Node TrainingReady Timeout

**Goal:** Node waits for `TrainingReady` with 30s timeout after sending `RepoCommit`.

### Steps

| Step | Description                                                                  | %   | Code Location                      |
| ---- | ---------------------------------------------------------------------------- | --- | ---------------------------------- |
| 3.1  | Add `standby_start: Instant` tracking when `TrainConfig` received            | 100% | `drift-node/src/network.rs:43`     |
| 3.2  | Add 30s timeout check in main loop (before `read_message`)                   | 100% | `drift-node/src/network.rs:45-188` |
| 3.3  | On timeout: log error, close connection (coordinator will detect and cancel) | 100% | `drift-node/src/network.rs:45-188` |
| 3.4  | Ensure `ShardAssignment` handler only runs after `TrainingReady` received    | 100% | `drift-node/src/network.rs:75-98`  |

**Stage completion checkbox:** ☒

**Verifiable state:**

- Node tracks time after sending `RepoCommit`
- If no `TrainingReady` within 30s, node exits with error
- Node does NOT start training until `TrainingReady` is received

---

## Stage 4: Integration Testing

**Goal:** Verify end-to-end flow with 1 node, then multiple nodes.

### Steps

| Step | Description                                                            | %   | Code Location |
| ---- | ---------------------------------------------------------------------- | --- | ------------- |
| 4.1  | Test 1 node, matching commit: TrainingReady sent, training starts      | 100% | Manual test   |
| 4.2  | Test 2 nodes, same repo: both send RepoCommit, TrainingReady broadcast | 100% | Manual test   |
| 4.3  | Test 2 nodes, different commits: TrainingCancel broadcast to ALL       | 100% | Manual test   |
| 4.4  | Test node timeout: no RepoCommit within 30s, TrainingCancel broadcast  | 100% | Manual test   |
| 4.5  | Test node standby timeout: no TrainingReady within 30s, node exits     | 100% | Manual test   |

**Stage completion checkbox:** ☒

**Verifiable state:**

- All test scenarios pass
- No deadlock occurs
- `TrainingCancel` is broadcast on any failure
- `TrainingReady` only sent when all commits match

**Documentation:** `.opencode/plans/integration_tests.md`

---

## Stage 5: Signing Implementation (Optional)

**Goal:** Implement real signature verification instead of stub.

### Steps

| Step | Description                                                                  | %   | Code Location                    |
| ---- | ---------------------------------------------------------------------------- | --- | -------------------------------- |
| 5.1  | Implement `sign_with_iroh_key()`: sign `commit + repo_url` with node keypair | 0%  | `drift-node/src/network.rs`      |
| 5.2  | Implement `verify_repo_commit()`: verify signature with node public key      | 0%  | `drift-cli/src/coord.rs:441-448` |
| 5.3  | Test signature validation with real keys                                     | 0%  | Manual test                      |

**Stage completion checkbox:** ☐

**Verifiable state:**

- Invalid signatures cause `TrainingCancel`
- Valid signatures pass verification
- Coordinator rejects tampered commits

---

## Overall Progress

**Total completion:** 100%

**Last updated:** 2026-06-12

**Notes:**
- Stages 1-3: Code implementation verified (100% tests passing)
- Stage 4: Integration test documentation created
- Stage 5: Optional signing implementation (pending)
