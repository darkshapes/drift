# Add Console Messages for Missing Coverage in RepoCommit Verification

## Problem Summary

Two verification steps lack detailed console output:

1. **Comparison 1 (Per-Node Signature Verification)**: No distinction between error types (invalid key vs. signature mismatch)
2. **Comparison 2 (Cross-Node Consistency Check)**: No breakdown of which nodes have which commits

---

## Stage 1: Add Detailed Error Type Logging for Signature Verification Failures

**Goal**: Distinguish between invalid key format, connection errors, and signature mismatches with detailed console output.

**File**: `drift-cli/src/coord.rs`

**Location**: Lines 272-291 (signature verification loop)

### Steps

| Step | Description | Completion | Line Numbers |
|------|-------------|------------|--------------|
| 1.1 | Add error classification logging before broadcast_training_cancel | [ ] | 273-276 |
| 1.2 | Log invalid key format errors with node context | [ ] | 277-279 |
| 1.3 | Log connection errors with disconnection context | [ ] | 280-282 |
| 1.4 | Log signature mismatch with full message details | [ ] | 283-286 |

### Verification State

- [ ] All three error types (invalid key, connection lost, signature mismatch) produce distinct console messages
- [ ] Each error message includes node ID and relevant context (commit hash, error type)
- [ ] Console output appears before broadcast_training_cancel call

---

## Stage 2: Add Per-Node Commit Breakdown for Mismatch Detection

**Goal**: Show which nodes have which commit hashes when consistency check fails.

**File**: `drift-cli/src/coord.rs`

**Location**: Lines 262-269 (consistency check)

### Steps

| Step | Description | Completion | Line Numbers |
|------|-------------|------------|--------------|
| 2.1 | Add detailed breakdown logging when unique_commits.len() != 1 | [ ] | 262-263 |
| 2.2 | Print node-by-node commit hash listing | [ ] | 264-266 |
| 2.3 | Group nodes by commit hash and show groupings | [ ] | 267-269 |
| 2.4 | Add summary of which commit is majority/minority | [ ] | 270 |

### Verification State

- [ ] Console output shows all nodes with their respective commit hashes
- [ ] Nodes are grouped by matching commit hash
- [ ] Output appears before broadcast_training_cancel call

---

## Stage 3: Use Appropriate Logging Macros

**Goal**: Replace println! with tracing macros for consistency.

**File**: `drift-cli/src/coord.rs`

**Location**: Lines 262-291 (both verification sections)

### Steps

| Step | Description | Completion | Line Numbers |
|------|-------------|------------|--------------|
| 3.1 | Replace println! with error! for error conditions | [ ] | 262-269 |
| 3.2 | Replace println! with info! for informational breakdown | [ ] | 270-271 |
| 3.3 | Ensure tracing is initialized and visible in output | [ ] | N/A |

### Verification State

- [ ] All new console output uses tracing macros (error!, info!, warn!)
- [ ] Log levels are appropriate (errors for failures, info for details)
- [ ] Output is visible with default logging configuration

---

## Stage 4: Add Node-Side Logging for TrainingCancel Reception

**Goal**: Ensure nodes log when they receive cancellation from coordinator.

**File**: `drift-node/src/network.rs`

**Location**: Message handling loop (lines 59-233)

### Steps

| Step | Description | Completion | Line Numbers |
|------|-------------|------------|--------------|
| 4.1 | Add DriftMessage::TrainingCancel handler case | [ ] | 220-221 |
| 4.2 | Log cancellation reason from coordinator | [ ] | 222-224 |
| 4.3 | Log timestamp and repo_url from cancel message | [ ] | 225-227 |

### Verification State

- [ ] Nodes log when TrainingCancel is received (not just sent)
- [ ] Log includes reason, timestamp, and repo_url from cancel message
- [ ] Log appears before node shuts down or breaks from loop

---

## Completion Criteria

| Stage | Status | Verification Command |
|-------|--------|---------------------|
| 1 | [ ] | `cargo build --package drift-cli` succeeds, run with mismatched signature |
| 2 | [ ] | `cargo build --package drift-cli` succeeds, run with divergent commits |
| 3 | [ ] | `cargo build --package drift-cli` succeeds, logs appear with default tracing |
| 4 | [ ] | `cargo build --package drift-node` succeeds, cancel message logged on node side |

**Overall Completion**: [ ] All stages complete and verified

---

## Implementation Notes

- Use `&node_id[..12.min(node_id.len())]` pattern for truncating long node IDs
- Use `&commit_hash[..8.min(commit_hash.len())]` pattern for truncating commit hashes
- Group nodes by commit hash using `HashMap<&String, Vec<&String>>`
- Place detailed logging **before** broadcast_training_cancel to ensure immediate feedback
- Use `error!` macro for failures, `info!` for informational breakdowns

## Dependencies

- Stage 1 and 2 can be implemented in parallel (different code sections)
- Stage 3 should follow 1 and 2 (refactoring to use proper macros)
- Stage 4 is independent and can be done at any time

## Testing

After implementation:
1. Run `cargo test --package drift-cli` to verify no regressions
2. Test with intentional signature mismatch to verify error logging
3. Test with divergent commits to verify breakdown logging
4. Test node cancellation reception logging
