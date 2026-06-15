# Plan: Fix Compilation Errors in Drift Project

## Stage 1: Add Missing Dependencies to drift-node/Cargo.toml

**Objective**: Add the missing crate dependencies that drift-node needs but currently lacks.

### Steps:

| Step | Description                           | % Complete | Lines                 | Checkbox |
| ---- | ------------------------------------- | ---------- | --------------------- | -------- |
| 1    | Add `drift-auth` as a path dependency | 0%         | drift-node/Cargo.toml | [ ]      |
| 2    | Add `ed25519-dalek` dependency        | 0%         | drift-node/Cargo.toml | [ ]      |
| 3    | Add `rand` dependency                 | 0%         | drift-node/Cargo.toml | [ ]      |

**Verification State**: Run `cargo build -p drift-node` and confirm all three import errors are resolved:

- `use drift_auth::crypto::sign_repo_commit`
- `use ed25519_dalek::SigningKey`
- `use rand::random()`

---

## Stage 2: Fix Type Error in network.rs Line 142

**Objective**: Correct the incorrect usage of `[u8; 32]` as a value.

### Steps:

| Step | Description                                | % Complete | Lines                         | Checkbox |
| ---- | ------------------------------------------ | ---------- | ----------------------------- | -------- |
| 1    | Identify the incorrect type-as-value usage | 100%       | drift-node/src/network.rs:142 | [x]     |
| 2    | Replace with proper array initialization   | 100%       | drift-node/src/network.rs:150 | [x]     |

**Current Code** (line 142):

```rust
let seed = [u8; 32];
```

**Fixed Code**:

```rust
let mut seed = [0u8; 32];
```

**Verification State**: Run `cargo build -p drift-node` and confirm error E0423 is resolved.

Note: The fix was applied at line 150 (not 142) where the array initialization is actually used.

---

## Stage 3: Fix Return Type Mismatch in load_or_create_signing_key

**Objective**: Ensure the function properly returns `Result<Vec<u8>, anyhow::Error>`.

### Steps:

| Step | Description                                      | % Complete | Lines                           | Checkbox |
| ---- | ------------------------------------------------ | ---------- | ------------------------------- | -------- |
| 1    | Review function signature and control flow       | 100%       | drift-node/src/network.rs:19-33 | [x]     |
| 2    | Fix the else branch to return proper Result type | 100%       | drift-node/src/network.rs:23-27 | [x]     |

**Current Issue**: The function body has control flow that returns `()` in certain paths instead of `Result<Vec<u8>, Error>`.

**Verification State**: Run `cargo build -p drift-node` and confirm error E0308 is resolved.

---

## Stage 4: Clean Up Unused Imports in drift-cli

**Objective**: Remove unused imports to eliminate warnings.

### Steps:

| Step | Description                                     | % Complete | Lines                    | Checkbox |
| ---- | ----------------------------------------------- | ---------- | ------------------------ | -------- |
| 1    | Remove unused `anyhow::Result` import           | 100%         | drift-cli/src/cli.rs:1   | [x]      |
| 2    | Remove unused signature functions from coord.rs | 100%         | drift-cli/src/coord.rs:5 | [x]      |
| 3    | Remove unused `Sha256` and `Digest` imports     | 100%         | drift-cli/src/coord.rs:7 | [x]      |

**Verification State**: Run `cargo build -p drift-cli` and confirm warnings are eliminated.

---

## Stage 5: Clean Up Unused Variables in drift-cli/src/node.rs

**Objective**: Prefix unused variables with underscore or remove them.

### Steps:

| Step | Description                                        | % Complete | Lines                     | Checkbox |
| ---- | -------------------------------------------------- | ---------- | ------------------------- | -------- |
| 1    | Rename `stdin_writer` to `_stdin_writer`           | 0%         | drift-cli/src/node.rs:386 | [ ]      |
| 2    | Rename `last_barrier_step` to `_last_barrier_step` | 0%         | drift-cli/src/node.rs:415 | [ ]      |

**Verification State**: Run `cargo build -p drift-cli` and confirm warnings are eliminated.

---

## Stage 6: Clean Up Unused Parameters in drift-coord/src/peer_registry.rs

**Objective**: Prefix unused parameters with underscore.

### Steps:

| Step | Description                                                     | % Complete | Lines                                | Checkbox |
| ---- | --------------------------------------------------------------- | ---------- | ------------------------------------ | -------- |
| 1    | Rename `node_id` to `_node_id` in handle_ask_for_more_more_work | 100%         | drift-coord/src/peer_registry.rs:205 | [x]      |
| 2    | Rename `step` to `_step` in update_on_progress                  | 100%         | drift-coord/src/peer_registry.rs:215 | [x]      |

**Verification State**: Run `cargo build -p drift-coord` and confirm warnings are eliminated.

---

## Stage 7: Clean Up Unused Imports in drift-coord/src/auth.rs

**Objective**: Remove unused imports.

### Steps:

| Step | Description                              | % Complete | Lines                      | Checkbox |
| ---- | ---------------------------------------- | ---------- | -------------------------- | -------- |
| 1    | Remove unused `SignedAuthMessage` import | 100%         | drift-coord/src/auth.rs:12 | [x]      |

**Verification State**: Run `cargo build -p drift-coord` and confirm warnings are eliminated.

---

## Final Verification

Run full project build:

```bash
cargo build --workspace
```

Expected result: Zero errors, minimal warnings.
