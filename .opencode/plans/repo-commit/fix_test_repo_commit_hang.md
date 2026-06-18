# Plan: Fix test_repo_commit_signed_with_iroh_key Hanging Test

## Overview

The test `test_repo_commit_signed_with_iroh_key` hangs because:

1. The coordinator waits indefinitely for a RepoCommit message from the node
2. The node's `get_git_commit` function returns a dummy string instead of the actual git commit hash

This plan addresses both issues with minimal, targeted changes.

---

## Stage 1: Add Timeout for RepoCommit in Test

**Scope:** Test file only (`drift-node/tests/stage1_signing.rs`)

**Objective:** Prevent the test from hanging indefinitely by adding a 60-second timeout for receiving the RepoCommit message.

### Steps

| #   | Step                                                                      | File                               | Line  | % Complete |
| --- | ------------------------------------------------------------------------- | ---------------------------------- | ----- | ---------- |
| 1   | Import tokio::time::Duration at top of test file                          | drift-node/tests/stage1_signing.rs | 1     | 100%       |
| 2   | Wrap read_message call with tokio::time::timeout(Duration::from_secs(60)) | drift-node/tests/stage1_signing.rs | 71    | 100%       |
| 3   | Handle timeout error by constructing TrainingCancel message               | drift-node/tests/stage1_signing.rs | 71-77 | 100%       |
| 4   | Send TrainingCancel to node and assert timeout occurred                   | drift-node/tests/stage1_signing.rs | 71-77 | 100%       |

### Verification State

- [ ] Test completes within 60 seconds even if RepoCommit is not received
- [ ] Test properly handles timeout case with TrainingCancel broadcast

### Stage Checkbox

- [x] Stage 1 Complete

---

## Stage 2: Replace Dummy Commit with Real git ls-remote

**Scope:** Network module (`drift-node/src/network.rs`)

**Objective:** Make `get_git_commit` use actual git commands to retrieve the HEAD commit hash instead of returning "dummy".

### Steps

| #   | Step                                                       | File                      | Line    | % Complete |
| --- | ---------------------------------------------------------- | ------------------------- | ------- | ---------- |
| 1   | Replace dummy return with git ls-remote command execution  | drift-node/src/network.rs | 330-332 | 100%       |
| 2   | Execute `git ls-remote <repo_url> HEAD` to get commit hash | drift-node/src/network.rs | 330-332 | 100%       |
| 3   | Parse first line of stdout to extract commit hash          | drift-node/src/network.rs | 330-332 | 100%       |
| 4   | Return error if git command fails or no HEAD ref found     | drift-node/src/network.rs | 330-332 | 100%       |

### Implementation Details

```rust
async fn get_git_commit(repo_url: &str) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["ls-remote", repo_url, "HEAD"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("git ls-remote failed for {}", repo_url);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().next())
        .map(|hash| hash.to_string())
        .ok_or_else(|| anyhow::anyhow!("no HEAD ref found"))
}
```

### Verification State

- [ ] Node sends RepoCommit with actual git commit hash from the test repository
- [ ] Coordinator receives valid commit hash that matches its expectation
- [ ] Signature verification succeeds with real commit data

### Stage Checkbox

- [x] Stage 2 Complete

---

## Final Verification

- [ ] Test `test_repo_commit_signed_with_iroh_key` passes without hanging
- [ ] All existing tests continue to pass
- [ ] No changes to production coordinator code (timeout logic remains test-only)

---

## Notes

- The production coordinator in `drift-cli/src/coord.rs` already has a 30-second timeout per node for RepoCommit collection (lines 170-188)
- This plan keeps the timeout change scoped to the test only, as requested
- The git ls-remote approach mirrors the existing `run_git_ls_remote` function in `drift-cli/src/node.rs:596`
