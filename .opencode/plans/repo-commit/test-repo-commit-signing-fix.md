# Test Failure Diagnosis: test_repo_commit_signed_with_iroh_key

## Summary

**Test**: `drift-node/tests/stage1_signing.rs::test_repo_commit_signed_with_iroh_key`
**Status**: ✅ FIXED
**Root Cause**: `get_git_commit()` in `drift-node/src/network.rs:266` uses `repo_url.contains("://")` to distinguish local paths from remote URLs. The test sent `"hf://model"` which contains `://`, causing `git ls-remote` to be called on a non-URL string, failing silently, and preventing `RepoCommit` from being sent.

---

## Stage 1: Diagnosis

| Step                             | Completion | Details                                                    |
| -------------------------------- | ---------- | ---------------------------------------------------------- |
| 1. Run tests to identify failure | 100%       | `cargo test` showed timeout at `stage1_signing.rs:97`      |
| 2. Read test file                | 100%       | Lines 8-126 in `drift-node/tests/stage1_signing.rs`        |
| 3. Trace handle_connection flow  | 100%       | Lines 23-178 in `drift-node/src/network.rs`                |
| 4. Identify get_git_commit logic | 100%       | Lines 266-290 in `drift-node/src/network.rs`               |
| 5. Confirm root cause            | 100%       | `"hf://model"` triggers remote path, `git ls-remote` fails |

**Stage 1 Completion**: [x] 100%
**Verification**: Error message `"timeout waiting for RepoCommit"` at line 97 confirmed no message sent.

---

## Stage 2: Solution Design

| Step                                 | Completion | Details                                              |
| ------------------------------------ | ---------- | ---------------------------------------------------- |
| 1. Determine fix approach            | 100%       | User requested: symlink + mock network requests      |
| 2. Plan symlink creation             | 100%       | Create `~/local/drift-test-repo` symlink to temp dir |
| 3. Plan model_artifact update        | 100%       | Change from `"hf://model"` to symlink path           |
| 4. Confirm no get_git_commit changes | 100%       | User confirmed: modify test only                     |

**Stage 2 Completion**: [x] 100%
**Verification**: Design aligns with user constraints (symlink, mock network, test-only changes).

---

## Stage 3: Implementation

| Step                        | Completion | Line Numbers              | Details                                          |
| --------------------------- | ---------- | ------------------------- | ------------------------------------------------ |
| 1. Add symlink creation     | 100%       | `stage1_signing.rs:44-53` | Create `~/local` dir, symlink to temp_dir        |
| 2. Update repo_url variable | 100%       | `stage1_signing.rs:62`    | Use symlink path instead of temp_dir             |
| 3. Update model_artifact    | 100%       | `stage1_signing.rs:78`    | Use `repo_url.clone()` instead of `"hf://model"` |

**Stage 3 Completion**: [x] 100%
**Verification**: Code compiles without errors.

### Code Changes

```rust
// Added after line 60 (after git commit):
let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
let local_dir = std::path::PathBuf::from(&home_dir).join("local");
std::fs::create_dir_all(&local_dir).unwrap();
let symlink_path = local_dir.join("drift-test-repo");
if symlink_path.exists() {
    std::fs::remove_file(&symlink_path).unwrap_or(());
}
std::os::unix::fs::symlink(&temp_dir, &symlink_path).unwrap();

// Changed line 62:
// Before: let repo_url = temp_dir.display().to_string();
// After:  let repo_url = symlink_path.display().to_string();

// Changed line 78:
// Before: model_artifact: Some("hf://model".to_string()),
// After:  model_artifact: Some(repo_url.clone()),
```

---

## Stage 4: Verification

| Step                     | Completion | Result                                           |
| ------------------------ | ---------- | ------------------------------------------------ |
| 1. Run specific test     | 100%       | `cargo test -p drift-node --test stage1_signing` |
| 2. Check test output     | 100%       | `test_repo_commit_signed_with_iroh_key ... ok`   |
| 3. Verify execution time | 100%       | Finished in 3.67s (was 122.69s timeout)          |

**Stage 4 Completion**: [x] 100%
**Verification**: Test passes with output:

```
running 1 test
test test_repo_commit_signed_with_iroh_key ... ok

test result: ok. 1 passed; 0 failed
```

---

## Files Modified

| File                                 | Lines Changed | Change Type                    |
| ------------------------------------ | ------------- | ------------------------------ |
| `drift-node/tests/stage1_signing.rs` | 44-62, 78     | Symlink creation + path update |

---

## Verification State

- [x] Test compiles without errors
- [x] Test passes (0 failed)
- [x] Execution time < 5s (was 122s timeout)
- [x] No changes to production code (test-only fix)
- [x] Symlink created at `~/local/drift-test-repo`
- [x] `get_git_commit` receives local path, executes `git rev-parse HEAD` successfully

---

## Notes

- **Constraint**: User requested symlink approach to mock network requests
- **Scope**: Test modification only, no changes to `get_git_commit` logic
- **Trade-off**: Symlink persists in `~/local` until manually cleaned up
- **Future**: Consider making `get_git_commit` more robust for mixed URL/path schemes
