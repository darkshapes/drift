# Analysis: Unused Import Warnings

## Problem

The Rust compiler warns about unused imports in `drift-cli/src/node.rs` and `drift-cli/src/coord.rs`:

```
warning: unused imports: `ShardAssignment` and `TrainConfig`
 --> drift-cli/src/node.rs:4:99
```

## Root Cause

The types `ShardAssignment` and `TrainConfig` **are used**, but indirectly:

1. **Pattern matching on `DriftMessage` enum variants:**

   ```rust
   DriftMessage::TrainConfig(config) => { ... }
   DriftMessage::ShardAssignment(s) => { ... }
   ```

2. **Fully qualified type paths in function signatures:**
   ```rust
   async fn run_training(
       config: &drift_proto::TrainConfig,
       _shard: Option<&drift_proto::ShardAssignment>,
   )
   ```

The Rust compiler's `unused_imports` lint doesn't recognize pattern matching on enum variants as "using" the imported types, since the enum variant already carries the type information.

### Plan: Remove unused imports, use fully qualified paths ✅ **RECOMMENDED**

**Result**: Clean code, no warnings, explicit type paths, no duplication.

---

# Implementation Plan:

## Files to Modify

### Stage 1. `drift-cli/src/node.rs`

**Current imports (line 1-5):**

```rust
use std::collections::HashMap;
use anyhow::Result;
use drift_proto::{
    read_message, write_message, DriftMessage, NodeInfo, DRIFT_ALPN, DRIFT_RING_ALPN, RepoCommit, TrainConfig, ShardAssignment,
};
use iroh::{Endpoint, PublicKey};
```

**Modified imports:**

```rust
use anyhow::Result;
use drift_proto::{
    read_message, write_message, DriftMessage, NodeInfo, DRIFT_ALPN, DRIFT_RING_ALPN, RepoCommit,
};
use iroh::Endpoint;
```

**Changes:**

- ❌ Remove `std::collections::HashMap` (unused)
- ❌ Remove `TrainConfig` from `drift_proto` imports
- ❌ Remove `ShardAssignment` from `drift_proto` imports
- ❌ Remove `PublicKey` from `iroh` imports (unused)

**Impact on code:**

- Line 267: `DriftMessage::TrainConfig(config)` - ✅ No change needed (enum variant)
- Line 296: `DriftMessage::ShardAssignment(s)` - ✅ No change needed (enum variant)
- Line 335: `config: &drift_proto::TrainConfig` - ✅ Already fully qualified
- Line 338: `_shard: Option<&drift_proto::ShardAssignment>` - ✅ Already fully qualified
- Line 354: `config: &drift_proto::TrainConfig` - ✅ Already fully qualified

---

### Stage 2. `drift-cli/src/coord.rs`

**Current imports (line 1-5):**

```rust
use std::collections::HashMap;
use anyhow::{Context, Result};
use drift_proto::{
    read_message, write_message, DriftMessage, NodeInfo, TrainConfig, TrainingCancel, DRIFT_ALPN,
};
```

**Modified imports:**

```rust
use anyhow::{Context, Result};
use drift_proto::{
    read_message, write_message, DriftMessage, NodeInfo, TrainingCancel, DRIFT_ALPN,
};
```

**Changes:**

- ❌ Remove `std::collections::HashMap` (unused)
- ❌ Remove `TrainConfig` from `drift_proto` imports

**Impact on code:**

- Line 512: `-> Vec<drift_proto::ShardAssignment>` - ✅ Already fully qualified
- Line 537: `drift_proto::ShardAssignment { ... }` - ✅ Already fully qualified

---

### Stage 3. `drift-coord/src/main.rs`

**Current imports (line 1):**

```rust
use drift_coord::{checkpoint, monitor, scheduler};
```

**Modified imports:**

```rust
use drift_coord::{monitor, scheduler};
```

**Changes:**

- ❌ Remove `checkpoint` (unused import)

---

### Stage 4. `drift-cli/src/node.rs` - Dead Code Removal

**Remove unused functions (lines 532-623):**

- ❌ `discover_script_entrypoint()` - duplicate of `drift-node/src/script_discovery.rs`
- ❌ `find_ati_plug()` - duplicate of `drift-node/src/script_discovery.rs`
- ❌ `detect_venv_activation()` - duplicate of `drift-node/src/script_discovery.rs`
- ❌ `resolve_entrypoint_to_spawn_cmd()` - duplicate of `drift-node/src/script_discovery.rs`

These functions are never called in `drift-cli` and exist in `drift-node` where they're actually used.

---

### Stage 5. `drift-cli/src/node.rs` - Dead Code Removal

**Line 247:** Remove `repo_commit_sent` variable (dead code, never read).

---

### Stage 6. `drift-cli/src/coord.rs` - Dead Code Removal

**Line 160:** Remove `received` variable (dead code, never read).

---

### Stage 7. `drift-cli/src/node.rs` - Unused Parameter Fix

**Line 355:**

```rust
// Before:
coord_send: &mut iroh::endpoint::SendStream,

// After:
_coord_send: &mut iroh::endpoint::SendStream,  // Prefix with underscore
```

---

### Stage 8. `drift-cli/src/ipc.rs` - Dead Code

**Line 41:** Remove `format_stop()` function (only used in tests, dead code in production).

---

## Summary Table

| File              | Issue                              | Fix                                | Status  |
| ----------------- | ---------------------------------- | ---------------------------------- | ------- |
| `node.rs`         | Unused `HashMap` import            | Remove import                      | ⬜ Pending |
| `node.rs`         | Unused `PublicKey` import          | Remove import                      | ⬜ Pending |
| `node.rs`         | Unused `TrainConfig` import        | Remove import, use fully qualified | ⬜ Pending |
| `node.rs`         | Unused `ShardAssignment` import    | Remove import, use fully qualified | ⬜ Pending |
| `node.rs`         | Unused `repo_commit_sent` variable | Remove (dead code)                 | ⬜ Pending |
| `node.rs`         | Unused `coord_send` parameter     | Prefix with `_`                    | ✅ Done |
| `node.rs`         | Dead code functions (4 functions)  | Remove                             | ⬜ Pending |
| `coord.rs`        | Unused `received` variable         | Remove (dead code)                 | ✅ Done |
| `coord.rs`        | Unused `HashMap` import            | Remove import                      | ⬜ Pending |
| `coord.rs`        | Unused `TrainConfig` import        | Remove import, use fully qualified | ⬜ Pending |
| `ipc.rs`          | Dead code `format_stop()`          | Remove                             | ✅ Done |
| `main.rs` (coord) | Unused `checkpoint` import         | Remove import                      | ⬜ Pending |

---

## Benefits of Plan

✅ **No compiler warnings** - imports match actual usage
✅ **No duplication** - one source of truth per type
✅ **Explicit type paths** - clear where types come from (`drift_proto::TrainConfig`)
✅ **Easier refactoring** - if types move modules, only change qualified paths
✅ **Smaller scope** - each file only imports what it directly uses
✅ **No dead code** - unused functions removed

---

## Verification

After implementation, run:

```bash
cargo build --release
```

Expected: Zero warnings about unused imports/variables.
