# Plan: Fix Venv Activation by Using Bash Instead of Sh

## Overview

The spawn command uses `source <activate_path> && <script_key>`, but `source` is a bash builtin not available in `sh`. Change all `sh -c` invocations to `bash -c` to support venv activation.

---

## Stage 1: Analysis (100% ✅)

| Step | Completion | Details                                                                 |
| ---- | ---------- | --------------------------------------------------------------------- |
| 1.1  | ✅ 100%    | Identified `source` command requires bash, not sh                     |
| 1.2  | ✅ 100%    | Located two files using `sh -c` for spawn commands                    |
| 1.3  | ✅ 100%    | Confirmed spawn_cmd format includes `source <path> && <command>`      |

**Stage Completion:** ✅ 100%
**State:** Analysis complete

---

## Stage 2: Implementation (100% ✅)

| Step | Completion | File:Line                    | Details                                           |
| ---- | ---------- | ---------------------------- | ------------------------------------------------- |
| 2.1  | ✅ 100%    | `drift-cli/src/node.rs:403`  | Changed `Command::new("sh")` to `Command::new("bash")` |
| 2.2  | ✅ 100%    | `drift-node/src/training.rs:98` | Changed `Command::new("sh")` to `Command::new("bash")` |

**Stage Completion:** ✅ 100%
**State:** Implementation complete

---

## Stage 3: Testing (100% ✅)

| Step | Completion | Details                              |
| ---- | ---------- | ------------------------------------ |
| 3.1  | ✅ 100%    | drift-cli unit tests: 16 passed      |
| 3.2  | ✅ 100%    | drift-node tests: 24 passed (including venv activation tests) |
| 3.3  | ✅ 100%    | Verified spawn command now uses bash for source command |

**Stage Completion:** ✅ 100%
**State:** All tests passing

---

## Files to Modify

1. `drift-cli/src/node.rs` - line 403
2. `drift-node/src/training.rs` - line 98

## Implementation Details

**Before:**
```rust
let mut c = tokio::process::Command::new("sh");
c.arg("-c").arg(spawn_cmd);
```

**After:**
```rust
let mut c = tokio::process::Command::new("bash");
c.arg("-c").arg(spawn_cmd);
```

**Rationale:** The `source` command is a bash builtin. Using `sh` causes "command not found" errors when trying to activate virtual environments.

---

**Created:** Sat Jun 20 2026
**Status:** ✅ Complete - All tests passing
