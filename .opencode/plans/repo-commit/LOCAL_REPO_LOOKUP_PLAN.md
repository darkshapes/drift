# Plan: Local Repo Cache Lookup for `--repo` Flag (COMPLETED)

## Summary

Implemented local repo cache lookup for `--repo` flag in drift-cli.

### Changes Made

**File: `drift-cli/src/node.rs`**

**Before:**
- Path: `~/.local/state/{covn|drift}` (incorrect)
- Only checked short name (repo_name)
- Duplicate code block

**After:**
- Path: `~/.local/share/{covn|drift}` (correct)
- Checks both `owner/repo` and `repo_name` variants
- Consolidated logic, no duplication
- Handles empty URL edge case

### New Files

- `drift-cli/src/lib.rs` - Library export for tests
- `drift-cli/tests/node_test.rs` - Unit tests

### Test Results

```
running 4 tests
test test_empty_url_handling ... ok
test test_path_construction ... ok
test test_repo_parsing ... ok
test test_single_slash_url ... ok
test result: ok. 4 passed; 0 failed
```

### Verification

1. Run: `cargo test --package drift-cli`
2. Build: `cargo build --package drift-cli`