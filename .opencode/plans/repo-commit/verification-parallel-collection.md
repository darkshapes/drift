# Drift Verification Stage Fix - Parallel Commit Collection

## Problem

The verification stage currently processes peers **sequentially**, verifying each commit immediately and bailing on first error. This causes the second peer's commit to never be read when the first peer fails verification.

### Current Flow (Broken)

```
for each peer:
  1. Read RepoCommit
  2. Verify immediately ← FAILS HERE
  3. Bail ← Second peer's commit never read
```

### Observed Behavior

- Peer 1 (5598e6486906...) sends RepoCommit ✓
- Coordinator verifies Peer 1's commit ✝ (signature error)
- Coordinator bails before reading Peer 2's RepoCommit ✝
- **Peer 2's commit sits unread in network buffer**

## Solution

Split verification into **two phases**:

### Phase 1: Collect All Commits (Parallel)

- Spawn async tasks for all peers simultaneously
- Each task has independent 30s timeout
- Collect all RepoCommit messages before any verification

### Phase 2: Verify Together

- Verify all commits after collection complete
- Check signature validity for all peers
- Check commit hash consistency
- Only then broadcast TrainingReady or TrainingCancel

## Implementation Plan

### Step 1: Refactor Collection Loop

**File:** `drift-cli/src/coord.rs:168-234`
**Change:** Replace sequential loop with parallel spawn

```rust
// Spawn async tasks for each peer
let mut commit_tasks = Vec::new();
for i in 0..connections.len() {
    let node_id = node_infos[i].node_id.clone();
    let recv = /* get recv stream */;
    let handle = tokio::spawn(async move {
        // 30s timeout per peer
        tokio::timeout(Duration::from_secs(30), async {
            loop {
                match read_message(&mut recv).await {
                    Ok(DriftMessage::RepoCommit(c)) => return Ok((node_id, c)),
                    Ok(other) => continue,
                    Err(e) => return Err(e),
                }
            }
        })
    });
    commit_tasks.push(handle);
}

// Wait for all tasks
let results: Vec<_> = commit_tasks.into_iter().map(|h| h.await?).collect();
```

**Status:** ⬜ Pending

### Step 2: Handle Partial Failures

**File:** `drift-cli/src/coord.rs:184-233`
**Change:** Track which peers succeeded/failed separately

```rust
let mut successful_commits = Vec::new();
let mut failed_nodes = Vec::new();

for result in results {
    match result {
        Ok((node_id, commit)) => successful_commits.push((node_id, commit)),
        Err(e) => failed_nodes.push((node_id, e)),
    }
}

if failed_nodes.is_empty() && successful_commits.len() == connections.len() {
    // All peers responded
} else if successful_commits.is_empty() {
    // All failed - cancel training
} else {
    // Partial - some responded, some timed out
    // Decide: proceed with subset or cancel?
}
```

**Status:** ⬜ Pending

### Step 3: Batch Verification

**File:** `drift-cli/src/coord.rs:195-216`
**Change:** Verify all commits together, not individually

```rust
// Collect all commits first
let commits: Vec<&String> = successful_commits.iter().map(|(_, c)| &c.commit).collect();
let unique_commits: HashSet<_> = commits.iter().collect();

// Check commit consistency across all peers
if unique_commits.len() != 1 {
    broadcast_training_cancel(..., "Commit mismatch");
    anyhow::bail!("Commit mismatch: {} different commits", unique_commits.len());
}

// Verify signatures for all successful commits
for (node_id, commit) in successful_commits {
    if let Err(e) = verify_repo_commit(&commit, &node_id) {
        broadcast_training_cancel(...);
        anyhow::bail!("Signature verification failed for node {}: {}", node_id, e);
    }
}
```

**Status:** ⬜ Pending

### Step 4: Broadcast TrainingReady

**File:** `drift-cli/src/coord.rs:252-254`
**Change:** Only after all verifications pass

```rust
// All verified - broadcast to ALL peers (including failed ones?)
for (send, _) in connections.iter_mut() {
    write_message(send, &DriftMessage::TrainingReady).await?;
}
```

**Status:** ⬜ Pending

## State Completion Table

| Stage               | Step                          | File     | Lines   | Status |
| ------------------- | ----------------------------- | -------- | ------- | ------ |
| **1. Collection**   | 1.1 Spawn parallel tasks      | coord.rs | 168-190 | ⬜     |
|                     | 1.2 Track results             | coord.rs | 190-210 | ⬜     |
| **2. Verification** | 2.1 Batch signature check     | coord.rs | 210-230 | ⬜     |
|                     | 2.2 Commit consistency        | coord.rs | 230-240 | ⬜     |
| **3. Broadcast**    | 3.1 TrainingReady on success  | coord.rs | 252-254 | ⬜     |
|                     | 3.2 TrainingCancel on failure | coord.rs | 186-213 | ⬜     |

## Verification Criteria

- [ ] All peers send RepoCommit simultaneously
- [ ] Each peer has independent 30s timeout
- [ ] Verification happens after all commits collected
- [ ] Second peer's commit is read even if first fails
- [ ] Training proceeds only if all peers verified successfully

## Timeout Behavior

**Per-peer timeout:** 30 seconds each (parallel)

- Peer 1: 0-30s ✓
- Peer 2: 0-30s ✓ (simultaneous, not sequential)
- Total wall time: ~30s (not 60s)

## Files Modified

1. `drift-cli/src/coord.rs` - Main coordinator logic
2. (Optional) `drift-proto/src/lib.rs` - Message types (if needed)

## Rollback Plan

If issues arise, revert to sequential verification with improved error messages showing which peer failed and why.
