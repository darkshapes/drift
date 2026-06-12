# RepoCommit Signing Validation

## Stage 1: Signature Function Verification

**Goal:** Verify that `sign_repo_commit` and `verify_repo_commit` functions are correctly implemented and tested.

### Steps

| Step | Description                                                                                        | %   | Code Location                    |
| ---- | -------------------------------------------------------------------------------------------------- | --- | -------------------------------- |
| 1.1  | Verify `sign_repo_commit` function signature and implementation        | 0%  | `drift-auth/src/crypto.rs:51-59` |
| 1.2  | Verify `verify_repo_commit` function signature and implementation      | 0%  | `drift-auth/src/crypto.rs:61-77` |
| 1.3  | Confirm node uses `sign_repo_commit` in `TrainConfig` handler          | 0%  | `drift-node/src/network.rs:145`  |
| 1.4  | Confirm coordinator calls `verify_repo_commit` on received commits     | 0%  | `drift-cli/src/coord.rs:441-448` |

**Stage completion checkbox:** ☐

**Verifiable state:**

- `sign_repo_commit` creates Ed25519 signature of `node_id|commit|repo_url`
- `verify_repo_commit` validates signature using node's public key
- Node sends signed RepoCommit to coordinator
- Coordinator verifies signature before processing commit

---

## Stage 2: Key Flow Implementation

**Goal:** Verify the node's signing key loading and usage flow.

### Steps

| Step | Description                                                                                        | %   | Code Location                    |
| ---- | -------------------------------------------------------------------------------------------------- | --- | -------------------------------- |
| 2.1  | Node loads 32-byte signing key from file                              | 0%  | `drift-node/src/network.rs:20-34`|
| 2.2  - Node converts key to `SigningKey::from_bytes(&seed)`             | 0%  | `drift-node/src/network.rs:142-144`|
| 2.3  - Node signs commit with node's own ID                            | 0%  | `drift-node/src/network.rs:145`  |
| 2.4  - Node attaches signature to RepoCommit message                   | 0%  | `drift-node/src/network.rs:152-156`|

**Stage completion checkbox:** ☐

**Verifiable state:**

- Node reads 32-byte key file from `.drift/identity/signing_key`
- Node creates `SigningKey` from key bytes
- Node signs `node_id|commit|repo_url` with its key
- Node sends `RepoCommit` with signature to coordinator

---

## Stage 3: Coordinator Verification Flow

**Goal:** Verify coordinator's signature verification and error handling.

### Steps

| Step | Description                                                                                        | %   | Code Location                    |
| ---- | -------------------------------------------------------------------------------------------------- | --- | -------------------------------- |
| 3.1  - Coordinator derives public key from node ID (base58)            | 0%  | `drift-cli/src/coord.rs:444`   |
| 3.2  - Coordinator calls `verify_repo_commit` with all 5 params        | 0%  | `drift-cli/src/coord.rs:446`   |
| 3.3  - Coordinator broadcasts `TrainingCancel` on signature failure    | 0%  | `drift-cli/src/coord.rs:166-172`|
| 3.4  - Coordinator continues on verified commits                       | 0%  | `drift-cli/src/coord.rs:174`   |

**Stage completion checkbox:** ☐

**Verifiable state:**

- Coordinator converts node ID (base58) to `PublicKey`
- Coordinator verifies RepoCommit signature using node's public key
- Invalid signatures cause immediate `TrainingCancel` broadcast
- Valid signatures allow commit processing to continue

---

## Stage 4: Signature Test Coverage

**Goal:** Add unit and integration tests for signature verification flow.

### Steps

| Step | Description                                                                                        | %   | Code Location                    |
| ---- | -------------------------------------------------------------------------------------------------- | --- | -------------------------------- |
| 4.1  | Add round-trip test: sign RepoCommit → verify with same key            | 0%  | `drift-auth/src/crypto.rs`       |
| 4.2  | Add invalid signature test: verification fails with wrong key          | 0%  | `drift-auth/src/crypto.rs`       |
| 4.3  | Add node-to-coordinator flow test: node signs → coordinator verifies   | 0%  | `drift-proto/tests/*.rs`         |
| 4.4  | Add integration test scenario: valid signature → training proceeds     | 0%  | Manual test                      |
| 4.5  | Add integration test scenario: invalid signature → TrainingCancel      | 0%  | Manual test                      |

**Stage completion checkbox:** ☐

**Verifiable state:**

- Unit tests confirm `sign_repo_commit`/`verify_repo_commit` round-trip success
- Unit tests confirm wrong key detection
- Integration tests confirm node→coordinator signature flow
- Manual tests confirm valid signatures allow training to start
- Manual tests confirm invalid signatures trigger coordinator cancel

---

## Stage 5: Documentation Completion

**Goal:** Complete signing implementation documentation and update progress tracking.

### Steps

| Step | Description                                                                                        | %   | Code Location                    |
| ---- | -------------------------------------------------------------------------------------------------- | --- | -------------------------------- |
| 5.1  | Create this document: `.opencode/plans/repo_commit_signing_validation.md` | 100%| `N/A`                            |
| 5.2  | Update `repo_commit_deadlock_fix.md` Stage 5 completion percentage    | 0%  | `.opencode/plans/repo_commit_deadlock_fix.md` |
| 5.3  | Update overall progress calculation in `repo_commit_deadlock_fix.md`   | 0%  | `.opencode/plans/repo_commit_deadlock_fix.md` |
| 5.4  | Mark all Stage 5 sub-steps as completed when implemented               | 0%  | `Various`                        |

**Stage completion checkbox:** ☐

**Verifiable state:**

- This document exists and describes signing implementation
- Stage 5 in deadlock fix plan shows >0% completion
- Overall progress percentage updated accordingly
- All verification checkpoints can be validated

---

## Overall Progress

**Total completion:** 0%

**Last updated:** 2026-06-12

**Notes:**
- Stages 1-4: Describe signing implementation requirements
- Stage 5: Documentation and progress tracking
- Upon completing all steps, update `repo_commit_deadlock_fix.md` to reflect implementation