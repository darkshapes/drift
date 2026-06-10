# Implementation Plan: Git Commit Verification for Distributed Training

## Context

When `train` is launched (NOT simulated) in `darkshapes/drift/`, it should:

1. Check for a repo in `~/.local/state/covn/` or `~/.local/state/drift/`
2. Trigger `git ls-remote` for that repo folder
3. Retrieve the git commit to send back to the coordinator

The coordinator can _only_ proceed with training if all other nodes confirm the commit hash, signed with their Iroh key.

---

## Protocol

### New messages in `drift-proto/src/lib.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoCommit {
    pub commit: String,      // git commit hash
    pub repo_url: String,   // bind commit to specific repo
    pub signature: Vec<u8>, // sign(commit + repo_url) with Iroh private key
}

pub enum DriftMessage {
    // ... existing ...
    RepoCommit(RepoCommit),
    TrainingReady,
}
```

### Signature format

- Sign: `commit + repo_url` using Iroh keypair
- This binds the commit to the specific repo URL

---

## Node Side (`drift-cli/src/node.rs`)

### Modify `run_real_training()` at line 327

**Flow:**

1. Extract `repo_url` from `config.train_repo_url`
2. Resolve path:
   - Check `~/.local/state/covn/<repo>` then `~/.local/state/drift/<repo>`
   - Error if not found
3. Run: `git ls-remote <path> HEAD` → parse commit hash
   - Error if fails
4. Sign(`commit + repo_url`) using `endpoint.key_pair()`
5. Send `RepoCommit{commit, repo_url, signature}`
6. Read message from coordinator:
   - `TrainingReady` → proceed
   - timeout (5 min) → error
7. Spawn Python subprocess (existing logic unchanged)

---

## Coordinator Side (`drift-cli/src/coord.rs`)

### In `train()` function, add state tracking

```rust
struct CommitState {
    confirmed: HashMap<String, (String, Vec<u8>)>, // node_id → (commit, signature)
}

On each connection:
  - Read RepoCommit messages
  - Verify signature using node's Iroh public key
  - On conflict (different commit than existing) → cancel training
  - Store confirmed (commit, signature)

When ALL nodes confirmed SAME commit:
  - Broadcast TrainingReady to all nodes
  - Proceed with existing TrainConfig/ShardAssignment
```

**On TrainConfig, set `train_config.git_commit = agreed_commit`.**

---

## Error Handling

| Failure                       | Action                             |
| ----------------------------- | ---------------------------------- |
| Repo not at either path       | Cancel with helpful message        |
| `git ls-remote` fails         | Cancel with message                |
| Signature invalid             | Cancel with "unauthorized"         |
| Commit conflict between nodes | Cancel with "commit hash mismatch" |
| `TrainingReady` timeout       | Cancel with timeout message        |

---

## Key Files to Modify

| File                     | Change                                       |
| ------------------------ | -------------------------------------------- |
| `drift-proto/src/lib.rs` | Add `RepoCommit` struct and messages         |
| `drift-cli/src/node.rs`  | Modify `run_real_training()` for commit flow |
| `drift-cli/src/coord.rs` | Add commit verification and state tracking   |

---

## Testing Strategy

1. Mock two nodes with different commits → expect cancel (commit hash mismatch)
2. Mock two nodes with same commit → expect TrainingReady broadcast
3. Mock invalid signature → expect unauthorized cancel
4. Mock missing repo → expect helpful error
